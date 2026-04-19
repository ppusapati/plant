//! ADR-1 Handheld Controller Firmware
//!
//! Target: ESP32-S3-WROOM-1 (Xtensa LX7 dual-core @ 240MHz)
//! Framework: esp-hal + Embassy async runtime
//!
//! Subsystems:
//! - Display (3.5" ILI9488 TFT via SPI)
//! - LoRa Communication (SX1276 via SPI2)
//! - NavIC GNSS (u-blox MAX-M10S via UART)
//! - Input (2× Joysticks via ADC, 8× Buttons, E-STOP)
//! - Status LED (APA102-2020 industrial RGB LED via SPI)
//! - BLE 5.0 (integrated, for close-range config)
//! - WiFi (integrated, for data sync)

#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::SpiMode;
use esp_hal::prelude::*;
use static_cell::StaticCell;

mod comms;
mod drivers;
mod protocol;
mod ui;

use comms::{ControllerComms, ControllerCommsState};
use drivers::sx1276::{ControllerLoRaConfig, ControllerRadio};
use protocol::{buttons, ControllerInput, MessageId, RobotTelemetry};
use ui::{DisplayUpdate, Screen};

// ─── Concrete type aliases ────────────────────────────────────────────

/// Blocking SPI2 for SX1276 LoRa radio
type LoraSpi   = Spi<'static, esp_hal::Blocking>;
type LoraCs    = Output<'static>;
type LoraReset = Output<'static>;
/// Fully-typed LoRa radio driver for the comms task
type LoraRadio = ControllerRadio<LoraSpi, LoraCs, LoraReset>;

/// Blocking SPI3 for APA102-2020 status LEDs (GPIO46 CLK, GPIO45 MOSI)
type LedSpi = Spi<'static, esp_hal::Blocking>;

// ─── Inter-task communication ─────────────────────────────────────────

/// Latest telemetry from robot (comms task → display + status LED tasks)
static TELEMETRY_SIGNAL: Signal<CriticalSectionRawMutex, RobotTelemetry> = Signal::new();

/// Joystick / button input (input task → comms task)
static INPUT_CHANNEL: Channel<CriticalSectionRawMutex, ControllerInput, 4> = Channel::new();

/// Display update commands (comms task → display task)
static DISPLAY_CHANNEL: Channel<CriticalSectionRawMutex, DisplayUpdate, 8> = Channel::new();

/// Emergency stop (input task → comms task)
static ESTOP_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

/// Link quality (comms task → status LED task)
static LINK_STATUS: Signal<CriticalSectionRawMutex, ControllerCommsState> = Signal::new();

// ─── Static cells for hardware resources ──────────────────────────────

static LORA_RADIO: StaticCell<LoraRadio> = StaticCell::new();
static LED_SPI: StaticCell<LedSpi> = StaticCell::new();

// ─── Main entry point ─────────────────────────────────────────────────

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    info!("ADR-1 Controller Firmware v0.1.0 starting...");

    // Initialize ESP32-S3 peripherals
    let config = esp_hal::Config::default();
    let peripherals = esp_hal::init(config);

    // Initialize heap allocator (for WiFi/BLE stacks)
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // Initialize embassy time driver
    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("ESP32-S3 peripherals initialized");

    // ─── SPI2 → LoRa SX1276 ─────────────────────────────────────────
    // Pinout (controller PCB v1.2):
    //   GPIO12 = SCK, GPIO13 = MOSI, GPIO11 = MISO
    //   GPIO10 = CS,  GPIO9  = RESET
    let lora_spi_cfg = SpiConfig::default()
        .with_frequency(1_000_000.Hz())
        .with_mode(SpiMode::Mode0);
    let lora_spi = Spi::new(peripherals.SPI2, lora_spi_cfg)
        .with_sck(peripherals.GPIO12)
        .with_mosi(peripherals.GPIO13)
        .with_miso(peripherals.GPIO11);
    let lora_cs    = Output::new(peripherals.GPIO10, Level::High);
    let lora_reset = Output::new(peripherals.GPIO9,  Level::High);

    // Build the radio driver, promote to 'static
    let radio = ControllerRadio::new(lora_spi, lora_cs, lora_reset);
    let radio_static = LORA_RADIO.init(radio);

    // ─── SPI3 → APA102-2020 Status LEDs ──────────────────────────────
    // Pinout (controller PCB v1.2):
    //   GPIO46 = SCK, GPIO45 = MOSI  (data only, no MISO / no CS)
    let led_spi_cfg = SpiConfig::default()
        .with_frequency(8_000_000.Hz())
        .with_mode(SpiMode::Mode0);
    let led_spi = Spi::new(peripherals.SPI3, led_spi_cfg)
        .with_sck(peripherals.GPIO46)
        .with_mosi(peripherals.GPIO45);
    let led_spi_static = LED_SPI.init(led_spi);

    // ─── Spawn all subsystem tasks ────────────────────────────────────
    spawner.must_spawn(lora_comms_task(radio_static));
    spawner.must_spawn(input_task());
    spawner.must_spawn(display_task());
    spawner.must_spawn(status_led_task(led_spi_static));

    info!("ADR-1 Controller Firmware initialized — all tasks spawned");

    // Main loop: periodic system check
    loop {
        Timer::after(Duration::from_secs(10)).await;
        info!("Controller system heartbeat");
    }
}

// ─── LoRa Communication Task ──────────────────────────────────────────
// - Initializes the SX1276 radio
// - Polls for received packets (telemetry from robot)
// - Transmits drive commands and mode changes from INPUT_CHANNEL
// - Detects link loss and signals display / LED tasks

#[embassy_executor::task]
async fn lora_comms_task(radio: &'static mut LoraRadio) {
    info!("LoRa comms task started (SX1276 @ 866 MHz)");

    radio.init(ControllerLoRaConfig::default());

    let mut comms = ControllerComms::new();
    let mut telemetry = RobotTelemetry::default();
    let mut rx_buf = [0u8; 256];
    let mut tx_buf = [0u8; 256];
    let mut last_cmd_ms:  u64 = 0;
    let mut last_hb_ms:   u64 = 0;
    const CMD_INTERVAL_MS: u64 = 50;    // 20 Hz drive-command rate
    const HB_TIMEOUT_MS:   u64 = 3_000; // 3 s before declaring link lost

    loop {
        let now_ms = Instant::now().as_millis();

        // ── Receive (non-blocking poll) ───────────────────────────────
        let rx_len = radio.poll_receive(&mut rx_buf);
        if rx_len > 0 {
            if let Some(msg) = comms.process_received(&rx_buf[..rx_len], &mut telemetry) {
                telemetry.link_rssi = radio.last_rssi();
                last_hb_ms = now_ms;
                TELEMETRY_SIGNAL.signal(telemetry.clone());
                let _ = DISPLAY_CHANNEL.try_send(DisplayUpdate::RefreshTelemetry);

                if comms.state == ControllerCommsState::Disconnected
                    || comms.state == ControllerCommsState::Scanning
                {
                    comms.state = ControllerCommsState::Connected;
                    LINK_STATUS.signal(ControllerCommsState::Connected);
                    info!(
                        "Robot connected (RSSI={}dBm SNR={}dB)",
                        radio.last_rssi(),
                        radio.last_snr()
                    );
                }

                if matches!(msg, MessageId::Emergency) {
                    error!("Emergency stop received from robot");
                    let _ = DISPLAY_CHANNEL.try_send(DisplayUpdate::ShowAlert(0xFE));
                }
            }
        }

        // ── Heartbeat timeout ─────────────────────────────────────────
        if last_hb_ms > 0 && now_ms.saturating_sub(last_hb_ms) > HB_TIMEOUT_MS {
            if comms.state != ControllerCommsState::Disconnected {
                warn!("Robot heartbeat timeout — link lost");
                comms.state = ControllerCommsState::Disconnected;
                telemetry.connected = false;
                LINK_STATUS.signal(ControllerCommsState::Disconnected);
                TELEMETRY_SIGNAL.signal(telemetry.clone());
            }
        }

        // ── E-STOP button → send Emergency packet ────────────────────
        if ESTOP_SIGNAL.signaled() {
            let _ = ESTOP_SIGNAL.wait().await; // consume signal
            let estop_pkt = comms.build_estop_packet();
            let pkt_len = estop_pkt.len().min(tx_buf.len());
            tx_buf[..pkt_len].copy_from_slice(&estop_pkt[..pkt_len]);
            if radio.transmit(&tx_buf[..pkt_len]) {
                comms.tx_count += 1;
            }
            warn!("Emergency stop packet transmitted");
        }

        // ── Drive command at 20 Hz in manual mode ────────────────────
        if now_ms.saturating_sub(last_cmd_ms) >= CMD_INTERVAL_MS {
            last_cmd_ms = now_ms;

            if let Ok(input) = INPUT_CHANNEL.try_receive() {
                // E-STOP button (hardware)
                if input.button_pressed(buttons::ESTOP) {
                    ESTOP_SIGNAL.signal(true);
                }

                // MODE button: cycle Idle → Manual → Auto → Idle
                if input.button_pressed(buttons::MODE) {
                    let next_mode: u8 = match telemetry.mode {
                        0 => 1,
                        1 => 2,
                        _ => 0,
                    };
                    let pkt = comms.build_mode_command(next_mode);
                    let pkt_len = pkt.len().min(tx_buf.len());
                    tx_buf[..pkt_len].copy_from_slice(&pkt[..pkt_len]);
                    if radio.transmit(&tx_buf[..pkt_len]) {
                        comms.tx_count += 1;
                    }
                    info!("Mode command: {}", next_mode);
                }

                // HOME button: Return-to-home (mode = 4)
                if input.button_pressed(buttons::HOME) {
                    let pkt = comms.build_mode_command(4);
                    let pkt_len = pkt.len().min(tx_buf.len());
                    tx_buf[..pkt_len].copy_from_slice(&pkt[..pkt_len]);
                    if radio.transmit(&tx_buf[..pkt_len]) {
                        comms.tx_count += 1;
                    }
                    info!("Return-to-home command sent");
                }

                // L1 button: Start deseeding (mode = 3)
                if input.button_pressed(buttons::L1) {
                    let pkt = comms.build_mode_command(3);
                    let pkt_len = pkt.len().min(tx_buf.len());
                    tx_buf[..pkt_len].copy_from_slice(&pkt[..pkt_len]);
                    if radio.transmit(&tx_buf[..pkt_len]) {
                        comms.tx_count += 1;
                    }
                    info!("Deseeding mode command sent");
                }

                // Send joystick drive command in manual mode
                if telemetry.mode == 1
                    && comms.state != ControllerCommsState::Disconnected
                {
                    let pkt = comms.build_drive_command(&input);
                    let pkt_len = pkt.len().min(tx_buf.len());
                    tx_buf[..pkt_len].copy_from_slice(&pkt[..pkt_len]);
                    if radio.transmit(&tx_buf[..pkt_len]) {
                        comms.tx_count += 1;
                        comms.state = ControllerCommsState::ActiveControl;
                        LINK_STATUS.signal(ControllerCommsState::ActiveControl);
                    }
                }
            }
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}

// ─── Input Task ───────────────────────────────────────────────────────
// Reads joystick ADC values and button GPIO states at 50 Hz.
// Publishes ControllerInput to INPUT_CHANNEL.

#[embassy_executor::task]
async fn input_task() {
    info!("Input task started (50 Hz)");

    let mut prev_buttons: u16 = 0;
    let mut mode_held_since_ms: u64 = 0;

    loop {
        // Joystick ADC reads (GPIO1–GPIO4, 12-bit, centre = 2048)
        // TODO: replace with esp-hal ADC oneshot reads
        let joy_lx: u16 = 2048;
        let joy_ly: u16 = 2048;
        let joy_rx: u16 = 2048;
        let joy_ry: u16 = 2048;

        // Button GPIO reads (active-low, internal pull-up)
        // GPIO5=MODE, GPIO6=HOME, GPIO7=MENU
        // GPIO14=L1, GPIO15=L2, GPIO16=R1, GPIO17=R2
        // GPIO18=ESTOP
        // TODO: replace with real esp-hal GPIO reads
        let btn_raw: u16 = 0;
        let button_state:   u16 = btn_raw ^ 0x00FF; // active-low → active-high (8 buttons)
        let button_pressed: u16 = button_state & !prev_buttons;
        prev_buttons = button_state;

        // Long-press MODE (>2 s) → settings screen
        let now_ms = Instant::now().as_millis();
        if button_state & buttons::MODE != 0 {
            if mode_held_since_ms == 0 {
                mode_held_since_ms = now_ms;
            } else if now_ms.saturating_sub(mode_held_since_ms) > 2_000 {
                let _ = DISPLAY_CHANNEL.try_send(DisplayUpdate::SwitchScreen(Screen::Settings));
                mode_held_since_ms = u64::MAX; // prevent repeat
            }
        } else {
            mode_held_since_ms = 0;
        }

        // Hardware E-STOP → immediate signal (comms task handles transmission)
        if button_pressed & buttons::ESTOP != 0 {
            ESTOP_SIGNAL.signal(true);
            warn!("Hardware E-STOP button pressed");
        }

        let input = ControllerInput {
            joy_left_x:   normalize_joystick(joy_lx),
            joy_left_y:   normalize_joystick(joy_ly),
            joy_right_x:  normalize_joystick(joy_rx),
            joy_right_y:  normalize_joystick(joy_ry),
            buttons:       button_state,
            button_events: button_pressed,
        };

        let _ = INPUT_CHANNEL.try_send(input);

        Timer::after(Duration::from_millis(20)).await; // 50 Hz
    }
}

// ─── Display Task ─────────────────────────────────────────────────────
// Renders telemetry on the 3.5" ILI9488 TFT at up to 10 FPS.

#[embassy_executor::task]
async fn display_task() {
    info!("Display task started (ILI9488 480×320)");

    let mut current_screen = Screen::Dashboard;
    let mut last_refresh_ms: u64 = 0;
    const REFRESH_MS: u64 = 100; // 10 FPS

    loop {
        let now_ms = Instant::now().as_millis();

        // Process screen-change / alert commands
        while let Ok(update) = DISPLAY_CHANNEL.try_receive() {
            match update {
                DisplayUpdate::SwitchScreen(screen) => {
                    current_screen = screen;
                    info!("Display: → {:?}", screen);
                    last_refresh_ms = 0; // force immediate redraw
                }
                DisplayUpdate::ShowAlert(code) => {
                    info!("Display: alert 0x{:02X}", code);
                    // TODO: draw alert banner overlay on ILI9488
                }
                DisplayUpdate::RefreshTelemetry => {}
            }
        }

        // Periodic telemetry refresh
        if now_ms.saturating_sub(last_refresh_ms) >= REFRESH_MS && TELEMETRY_SIGNAL.signaled() {
            last_refresh_ms = now_ms;
            let telem = TELEMETRY_SIGNAL.wait().await;

            // TODO: drive the ILI9488 SPI display driver
            // For now emit key metrics via defmt
            match current_screen {
                Screen::Dashboard => {
                    debug!(
                        "[DSP] mode={} bat={}% rssi={}dBm lat={} lon={}",
                        telem.mode_str(),
                        telem.battery_soc,
                        telem.link_rssi,
                        telem.latitude  as f32,
                        telem.longitude as f32,
                    );
                }
                Screen::SensorDetail => {
                    debug!(
                        "[DSP] moisture={}% N={} P={} K={} pH={} health={}",
                        telem.soil_moisture as u16,
                        telem.soil_n, telem.soil_p, telem.soil_k,
                        (telem.soil_ph * 10.0) as u16,
                        telem.health_score,
                    );
                }
                Screen::PlantHealth => {
                    debug!(
                        "[DSP] NDVI={} health={}/1000 disease={}",
                        (telem.ndvi * 1000.0) as i16,
                        telem.health_score,
                        telem.disease_alert,
                    );
                }
                _ => {}
            }
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}

// ─── Status LED Task ──────────────────────────────────────────────────
// Drives the APA102-2020 industrial RGB LED via SPI.
//
// Colour encoding:
//   Cyan  scanning/slow-pulse   — no robot found
//   Green slow blink            — robot connected
//   Blue  fast blink            — actively controlling robot
//   Yellow pulse                — degraded link
//   Red   slow pulse            — link lost / E-STOP

#[embassy_executor::task]
async fn status_led_task(spi: &'static mut LedSpi) {
    use crate::drivers::ws2812::LedController;

    info!("Status LED task started (APA102-2020 via SPI3)");

    let mut ctrl = LedController::new();
    let mut link_state = ControllerCommsState::Scanning;

    loop {
        // Check for a new link state (non-blocking peek via signaled())
        if LINK_STATUS.signaled() {
            link_state = LINK_STATUS.wait().await;
        }

        // Map link state to connected / mode / rssi for LedController
        let (connected, mode, rssi) = match link_state {
            ControllerCommsState::Scanning     => (false, 0u8, -130i16),
            ControllerCommsState::Connected    => (true,  1,   -80),
            ControllerCommsState::ActiveControl => (true, 1,   -70),
            ControllerCommsState::Degraded     => (true,  2,   -110),
            ControllerCommsState::Disconnected => (false, 5,   -130),
        };

        // Update LED colours (battery_soc = 100 unless telemetry arrives)
        ctrl.update_status(connected, mode, 100, rssi);

        // Flush to hardware via SPI3 → APA102 chain
        if ctrl.write(spi).is_err() {
            warn!("APA102: SPI write error");
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}

// ─── Helper ───────────────────────────────────────────────────────────

/// Normalize 12-bit joystick ADC value (0–4095, centre 2048) to –1000…1000
/// with a ±100-count dead-zone.
fn normalize_joystick(raw: u16) -> i16 {
    const DEADZONE: i32 = 100;
    let centered = raw as i32 - 2048;
    if centered.abs() < DEADZONE {
        return 0;
    }
    let sign = if centered > 0 { 1i32 } else { -1 };
    let magnitude     = centered.abs() - DEADZONE;
    let max_magnitude = 2048 - DEADZONE;
    ((magnitude * 1000 / max_magnitude) * sign).clamp(-1000, 1000) as i16
}


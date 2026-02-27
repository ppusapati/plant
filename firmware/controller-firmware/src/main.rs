//! ADR-1 Handheld Controller Firmware
//!
//! Target: ESP32-S3-WROOM-1 (Xtensa LX7 dual-core @ 240MHz)
//! Framework: esp-hal + Embassy async runtime
//!
//! Subsystems:
//! - Display (3.5" ILI9488 TFT via SPI)
//! - LoRa Communication (SX1276 via SPI)
//! - NavIC GNSS (u-blox MAX-M10S via UART)
//! - Input (2× Joysticks, 8× Buttons, E-STOP)
//! - Haptic Feedback (DRV2605L via I2C)
//! - Data Logging (microSD via SPI)
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
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::prelude::*;

mod comms;
mod drivers;
mod protocol;
mod ui;

// ─── Inter-task communication ────────────────────────────────────────

/// Telemetry data from robot (LoRa RX → Display update)
static TELEMETRY_SIGNAL: Signal<CriticalSectionRawMutex, protocol::RobotTelemetry> = Signal::new();

/// Joystick/button input (Input task → Comms task)
static INPUT_CHANNEL: Channel<CriticalSectionRawMutex, protocol::ControllerInput, 4> =
    Channel::new();

/// Display update commands
static DISPLAY_CHANNEL: Channel<CriticalSectionRawMutex, ui::DisplayUpdate, 8> = Channel::new();

/// Emergency stop signal
static ESTOP_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

// ─── Main entry point ────────────────────────────────────────────────

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

    // Spawn tasks
    spawner.must_spawn(input_task());
    spawner.must_spawn(display_task());
    spawner.must_spawn(status_led_task());

    info!("All controller tasks spawned");

    // Main loop: system coordination
    loop {
        Timer::after(Duration::from_secs(1)).await;

        // Check for E-STOP
        if ESTOP_SIGNAL.signaled() {
            let estop = ESTOP_SIGNAL.wait().await;
            if estop {
                error!("CONTROLLER E-STOP PRESSED");
                // Send emergency stop to robot
            }
        }
    }
}

// ─── Input Task ──────────────────────────────────────────────────────

#[embassy_executor::task]
async fn input_task() {
    info!("Input task started");

    // ADC for joystick axes
    // GPIO for buttons

    let mut prev_buttons: u16 = 0;

    loop {
        // Read joystick ADC values (0-4095 for 12-bit ADC)
        // Placeholder values - actual implementation reads ADC pins
        let joy_lx: u16 = 2048; // Center = 2048
        let joy_ly: u16 = 2048;
        let joy_rx: u16 = 2048;
        let joy_ry: u16 = 2048;

        // Read button states (active low with pull-ups)
        let buttons: u16 = 0; // Each bit = one button

        // Detect button edge (press events)
        let button_pressed = buttons & !prev_buttons;
        prev_buttons = buttons;

        // Convert joystick to velocity commands
        let input = protocol::ControllerInput {
            joy_left_x: normalize_joystick(joy_lx),
            joy_left_y: normalize_joystick(joy_ly),
            joy_right_x: normalize_joystick(joy_rx),
            joy_right_y: normalize_joystick(joy_ry),
            buttons,
            button_events: button_pressed,
        };

        // Send to command channel
        let _ = INPUT_CHANNEL.try_send(input);

        // 50Hz input polling
        Timer::after(Duration::from_millis(20)).await;
    }
}

// ─── Display Task ────────────────────────────────────────────────────

#[embassy_executor::task]
async fn display_task() {
    info!("Display task started");

    let mut current_screen = ui::Screen::Dashboard;

    loop {
        // Check for display update commands
        if let Ok(update) = DISPLAY_CHANNEL.try_receive() {
            match update {
                ui::DisplayUpdate::SwitchScreen(screen) => {
                    current_screen = screen;
                    info!("Switching to screen: {:?}", screen);
                }
                ui::DisplayUpdate::RefreshTelemetry => {
                    // Redraw telemetry data on current screen
                }
                ui::DisplayUpdate::ShowAlert(msg_id) => {
                    info!("Display alert: {}", msg_id);
                }
            }
        }

        // Update display at 10 FPS
        Timer::after(Duration::from_millis(100)).await;
    }
}

// ─── Status LED Task ─────────────────────────────────────────────────

#[embassy_executor::task]
async fn status_led_task() {
    loop {
        // WS2812B LED patterns based on state:
        // Green pulse: Normal operation
        // Blue pulse: Receiving telemetry
        // Yellow: Degraded link
        // Red: Error / E-STOP
        Timer::after(Duration::from_millis(50)).await;
    }
}

// ─── Helper functions ────────────────────────────────────────────────

/// Normalize joystick ADC value (0-4095) to signed range (-1000 to 1000)
/// with deadzone (±50 around center)
fn normalize_joystick(raw: u16) -> i16 {
    let centered = raw as i32 - 2048;
    let deadzone = 100; // ~2.4% deadzone

    if centered.abs() < deadzone {
        return 0;
    }

    // Scale to -1000..1000 range
    let sign = if centered > 0 { 1 } else { -1 };
    let magnitude = (centered.abs() - deadzone) as i32;
    let max_magnitude = (2048 - deadzone) as i32;

    let normalized = (magnitude * 1000 / max_magnitude) * sign;
    normalized.clamp(-1000, 1000) as i16
}

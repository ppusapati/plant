//! ADR-1 Autonomous Deseeder Robot - Main Firmware
//!
//! Target: STM32H743VIT6 (Cortex-M7 @ 480MHz)
//! Framework: Embassy-rs async runtime
//!
//! Subsystems:
//! - Navigation (NavIC GNSS + IMU + Wheel Encoders + EKF)
//! - Communication (LoRa + BLE + WiFi via ESP32 co-processor)
//! - Sensor Fusion (Plant health + Soil health + Environment)
//! - Motor Control (4x BLDC via CAN + Deseeding mechanism)
//! - Safety (E-STOP, watchdog, geofence, tilt protection)

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use core::sync::atomic::{AtomicU8, Ordering};
use embassy_executor::Spawner;
use embassy_stm32::can::{Fdcan, FdcanTx, FdcanRx};
use embassy_stm32::can::frame::{ClassicFrame, Header};
use embassy_stm32::can::{StandardId, Id};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{Channel, CountingMode};
use embassy_stm32::gpio::OutputType;
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;

mod comms;
mod control;
mod drivers;
mod navigation;
mod protocol;
mod sensors;

use comms::{CommsManager, LinkStatus};
use control::{DifferentialDrive, DriveState, RobotMode};
use drivers::gnss::NmeaParser;
use drivers::modbus::{ModbusRtu, SoilData};
use drivers::sx1276::{LoRaConfig, Sx1276};
use navigation::{HomePosition, NavState};
use navigation::path_planner::PathPlanner;
use protocol::{
    CommandType, MessageId, Packet, RobotCommand, SensorPacket, TelemetryPacket,
    ADDR_CONTROLLER, ADDR_ROBOT,
};
use sensors::EnvironmentData;

// ─── Concrete peripheral type aliases ────────────────────────────────

/// USART1 @ 115200 baud — NavIC/GPS GNSS (PA9 TX, PA10 RX)
type GnssUart = Uart<'static, peripherals::USART1, peripherals::DMA1_CH0, peripherals::DMA1_CH1>;

/// USART2 @ 9600 baud — RS485 Modbus soil sensors (PD5 TX, PD6 RX)
type Rs485Uart = Uart<'static, peripherals::USART2, peripherals::DMA1_CH2, peripherals::DMA1_CH3>;

/// I2C1 400 kHz — BME280, BH1750, VEML6075, MLX90614 (PB6 SCL, PB7 SDA)
type SensorI2c = I2c<'static, peripherals::I2C1>;

/// SPI1 1 MHz — LoRa SX1276 (PA5 SCK, PA6 MISO, PA7 MOSI)
type LoraSpi = Spi<'static, peripherals::SPI1, peripherals::DMA2_CH0, peripherals::DMA2_CH1>;
/// Sx1276 driver using raw SpiBus + separate CS/RESET pins
type LoraRadio = Sx1276<LoraSpi>;

/// FDCAN1 transmit half — drives 4× BLDC motor controllers + deseeder (PD0 RX, PD1 TX, AF3)
type CanTx = FdcanTx<'static, peripherals::FDCAN1>;

/// FDCAN1 receive half — listens for BMS status, motor feedback
type CanRx = FdcanRx<'static, peripherals::FDCAN1>;

/// TIM3 PWM — BTS7960 deseeder H-bridge (PB4 = R_PWM CH1, PB5 = L_PWM CH2, 10 kHz)
type DeseedPwm = SimplePwm<'static, peripherals::TIM3>;

/// TIM4 PWM — Soil probe servos (PD12 = servo A CH1, PD13 = servo B CH2, 50 Hz)
type ServoPwm = SimplePwm<'static, peripherals::TIM4>;

// ─── Inter-task communication channels ───────────────────────────────

/// Fused navigation state (GNSS + IMU + odometry) — updated when GNSS fixes arrive
static NAV_STATE: Signal<ThreadModeRawMutex, NavState> = Signal::new();

/// Sensor data channel (sensor task → comms task)
static SENSOR_CHANNEL: Channel<ThreadModeRawMutex, SensorPacket, 8> = Channel::new();

/// Command channel (comms task / navigation task → motor control task)
static COMMAND_CHANNEL: Channel<ThreadModeRawMutex, RobotCommand, 8> = Channel::new();

/// Telemetry to transmit (various tasks → comms task)
static TELEMETRY_CHANNEL: Channel<ThreadModeRawMutex, TelemetryPacket, 16> = Channel::new();

/// Emergency stop signal (any task can activate; comms/safety tasks clear it)
static ESTOP_SIGNAL: Signal<ThreadModeRawMutex, bool> = Signal::new();

/// Robot operating mode (set by comms task from controller commands)
static MODE_SIGNAL: Signal<ThreadModeRawMutex, RobotMode> = Signal::new();

/// Current mode as atomic byte — readable by any task without consuming the signal
static CURRENT_MODE_ATOMIC: AtomicU8 = AtomicU8::new(0); // 0 = RobotMode::Idle

/// Battery state of charge 0–100 % (written by can_rx_task from BMS CAN frames)
static BATTERY_SOC_ATOMIC: AtomicU8 = AtomicU8::new(0);

// ─── Interrupt bindings ──────────────────────────────────────────────

bind_interrupts!(struct Irqs {
    USART1 => embassy_stm32::usart::InterruptHandler<peripherals::USART1>;
    USART2 => embassy_stm32::usart::InterruptHandler<peripherals::USART2>;
    USART3 => embassy_stm32::usart::InterruptHandler<peripherals::USART3>;
    UART5  => embassy_stm32::usart::InterruptHandler<peripherals::UART5>;
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
    I2C2_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C2>;
    SPI1 => embassy_stm32::spi::InterruptHandler<peripherals::SPI1>;
    FDCAN1_IT0 => embassy_stm32::can::IT0InterruptHandler<peripherals::FDCAN1>;
    FDCAN1_IT1 => embassy_stm32::can::IT1InterruptHandler<peripherals::FDCAN1>;
});

// ─── Main entry point ────────────────────────────────────────────────

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("ADR-1 Robot Firmware v0.1.0 starting...");

    // Configure system clocks: HSE 8 MHz → PLL → 480 MHz SYSCLK
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL120,
            divp: Some(PllDiv::DIV2), // 480 MHz SYSCLK
            divq: Some(PllDiv::DIV8), // 60 MHz for SPI
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV2;  // 240 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 120 MHz
    }

    let p = embassy_stm32::init(config);
    info!("System clocks configured: SYSCLK=480MHz");

    // ─── GPIO Setup ──────────────────────────────────────────────────
    let led_status = Output::new(p.PC0, Level::Low, Speed::Low);
    let led_error  = Output::new(p.PC1, Level::Low, Speed::Low);
    let led_comms  = Output::new(p.PC2, Level::Low, Speed::Low);
    let _relay_ctrl = Output::new(p.PD7, Level::Low, Speed::Low);

    // E-STOP sense input (active-low, pull-up)
    let _estop_pin = Input::new(p.PD3, Pull::Up);

    // ─── GNSS UART (USART1) ──────────────────────────────────────────
    // u-blox NEO-M9N: 115200 baud, 8N1
    let mut gnss_cfg = usart::Config::default();
    gnss_cfg.baudrate = 115_200;
    let gnss_uart = Uart::new(
        p.USART1, p.PA10, p.PA9, Irqs,
        p.DMA1_CH0, p.DMA1_CH1, gnss_cfg,
    )
    .expect("GNSS UART init failed");

    // ─── RS485 Modbus UART (USART2) ──────────────────────────────────
    // NPK / soil sensors: 9600 baud, 8N1
    let mut rs485_cfg = usart::Config::default();
    rs485_cfg.baudrate = 9_600;
    let rs485_uart = Uart::new(
        p.USART2, p.PD6, p.PD5, Irqs,
        p.DMA1_CH2, p.DMA1_CH3, rs485_cfg,
    )
    .expect("RS485 UART init failed");
    // DE/RE control pin (PD4): High = transmit, Low = receive
    let rs485_de = Output::new(p.PD4, Level::Low, Speed::High);

    // ─── I2C1 Sensor Bus ─────────────────────────────────────────────
    // BME280, BH1750, VEML6075, MLX90614 @ 400 kHz
    let i2c1 = I2c::new(
        p.I2C1, p.PB6, p.PB7, Irqs,
        p.DMA1_CH4, p.DMA1_CH5,
        Hertz(400_000),
        i2c::Config::default(),
    );

    // ─── SPI1 → LoRa SX1276 ──────────────────────────────────────────
    // SX1276: CPOL=0, CPHA=0, up to 10 MHz
    let mut lora_spi_cfg = spi::Config::default();
    lora_spi_cfg.frequency = Hertz(1_000_000);
    let lora_spi = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6,
        p.DMA2_CH0, p.DMA2_CH1, lora_spi_cfg,
    );
    let lora_cs    = Output::new(p.PA4, Level::High, Speed::High);
    let lora_reset = Output::new(p.PB0, Level::High, Speed::High);
    let radio = Sx1276::new(lora_spi, lora_cs, lora_reset);

    // ─── FDCAN1 → CAN bus (4× BLDC + deseeder controller + BMS) ─────
    // PD0 = FDCAN1_RX (AF3), PD1 = FDCAN1_TX (AF3)
    // Default bit timing: 1 Mbit/s with 60 MHz FDCAN kernel clock (PLL1_Q ÷ 8 × 8 = 60 MHz)
    let (can_tx, can_rx) = Fdcan::new(p.FDCAN1, p.PD0, p.PD1, Irqs)
        .into_normal_mode()
        .split();

    // ─── TIM3 PWM → BTS7960 deseeder H-bridge ────────────────────────
    // PB4 = TIM3_CH1 (R_PWM, forward),  PB5 = TIM3_CH2 (L_PWM, reverse)
    // 10 kHz: above audible range, within BTS7960 20 kHz limit
    let deseeder_pwm = SimplePwm::new(
        p.TIM3,
        Some(PwmPin::new_ch1(p.PB4, OutputType::PushPull)),
        Some(PwmPin::new_ch2(p.PB5, OutputType::PushPull)),
        None,
        None,
        Hertz(10_000),
        CountingMode::EdgeAlignedUp,
    );

    // ─── TIM4 PWM → Soil probe servos ────────────────────────────────
    // PD12 = TIM4_CH1 (probe A),  PD13 = TIM4_CH2 (probe B)
    // Standard RC servo frequency: 50 Hz (20 ms period)
    let servo_pwm = SimplePwm::new(
        p.TIM4,
        Some(PwmPin::new_ch1(p.PD12, OutputType::PushPull)),
        Some(PwmPin::new_ch2(p.PD13, OutputType::PushPull)),
        None,
        None,
        Hertz(50),
        CountingMode::EdgeAlignedUp,
    );

    // ─── Spawn subsystem tasks ────────────────────────────────────────
    spawner.must_spawn(heartbeat_task(led_status));
    spawner.must_spawn(safety_task(led_error));
    spawner.must_spawn(comms_led_task(led_comms));
    spawner.must_spawn(gnss_task(gnss_uart));
    spawner.must_spawn(sensor_task(rs485_uart, rs485_de, i2c1));
    spawner.must_spawn(navigation_task());
    spawner.must_spawn(comms_task(radio));
    spawner.must_spawn(can_rx_task(can_rx));
    spawner.must_spawn(motor_control_task(can_tx, deseeder_pwm, servo_pwm));

    info!("ADR-1 Robot Firmware initialized — all tasks spawned");

    // Main loop: system-level health monitor
    loop {
        Timer::after(Duration::from_secs(10)).await;
        info!("System heartbeat — all tasks running");
    }
}

// ─── Heartbeat LED Task ──────────────────────────────────────────────
// Blinks the status LED at 1 Hz to confirm the system is alive.

#[embassy_executor::task]
async fn heartbeat_task(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(100)).await;
        led.set_low();
        Timer::after(Duration::from_millis(900)).await;
    }
}

// ─── Safety Monitor Task ─────────────────────────────────────────────
// Monitors the E-STOP signal and drives the error LED red.

#[embassy_executor::task]
async fn safety_task(mut led_error: Output<'static>) {
    info!("Safety monitor started");

    loop {
        let estop = ESTOP_SIGNAL.wait().await;
        if estop {
            error!("EMERGENCY STOP ACTIVATED");
            led_error.set_high();
            MODE_SIGNAL.signal(RobotMode::EmergencyStop);
        } else {
            info!("E-STOP cleared");
            led_error.set_low();
        }
    }
}

// ─── Communication Status LED Task ───────────────────────────────────

#[embassy_executor::task]
async fn comms_led_task(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(500)).await;
    }
}

// ─── GNSS Task ───────────────────────────────────────────────────────
// Feeds UART bytes from the u-blox NEO-M9N into the NMEA parser.
// Signals NAV_STATE whenever a complete GPGGA / GNGGA / GNRMC sentence
// is successfully decoded.

#[embassy_executor::task]
async fn gnss_task(mut uart: GnssUart) {
    info!("GNSS task started (u-blox NEO-M9N @ 115200)");

    let mut parser = NmeaParser::new();
    let mut byte_buf = [0u8; 1];
    let mut home = HomePosition::default();

    loop {
        match uart.read(&mut byte_buf).await {
            Ok(()) => {
                if parser.feed(byte_buf[0]) {
                    let d = &parser.data;

                    // Set home position on first valid fix
                    if !home.set && d.valid {
                        home = HomePosition {
                            latitude:  d.latitude,
                            longitude: d.longitude,
                            altitude:  d.altitude,
                            set:       true,
                        };
                        info!(
                            "GNSS: home position set ({}, {})",
                            d.latitude as f32,
                            d.longitude as f32
                        );
                    }

                    let (local_x, local_y) = home.gps_to_local(d.latitude, d.longitude);

                    let nav = NavState {
                        x:            local_x,
                        y:            local_y,
                        theta:        d.heading.to_radians(),
                        velocity:     d.speed_kmh / 3.6, // km/h → m/s
                        omega:        0.0,
                        latitude:     d.latitude,
                        longitude:    d.longitude,
                        altitude:     d.altitude,
                        fix_quality:  d.fix_quality as u8,
                        sat_count:    d.satellites_used,
                        hdop:         d.hdop,
                        timestamp_ms: Instant::now().as_millis(),
                    };

                    NAV_STATE.signal(nav);
                }
            }
            Err(e) => {
                warn!("GNSS UART error: {:?}", e);
                Timer::after(Duration::from_millis(10)).await;
            }
        }
    }
}

// ─── Sensor Task ─────────────────────────────────────────────────────
// Polls RS485 Modbus soil sensors (NPK, moisture, temperature) and I2C
// environment sensors (BME280, BH1750) at 1 Hz, then publishes a
// SensorPacket to SENSOR_CHANNEL for the comms task to transmit.

#[embassy_executor::task]
async fn sensor_task(
    mut rs485: Rs485Uart,
    mut de_pin: Output<'static>,
    mut i2c: SensorI2c,
) {
    info!("Sensor task started (RS485 Modbus + I2C)");

    let modbus = ModbusRtu::new(200);

    // BME280 I2C address (SDO → GND = 0x76)
    const BME280_ADDR: u8 = 0x76;
    // BH1750 I2C address (ADDR → GND = 0x23)
    const BH1750_ADDR: u8 = 0x23;

    // BH1750: power on, then continuous 1-lux-resolution mode
    let _ = i2c.write(BH1750_ADDR, &[0x01]).await;
    let _ = i2c.write(BH1750_ADDR, &[0x10]).await;

    // BME280: soft-reset then configure normal mode
    // (0xE0 = reset register, 0xB6 = reset magic)
    let _ = i2c.write(BME280_ADDR, &[0xE0, 0xB6]).await;
    Timer::after(Duration::from_millis(10)).await;
    let _ = i2c.write(BME280_ADDR, &[0xF2, 0x01]).await; // ctrl_hum: osrs_h=1
    let _ = i2c.write(BME280_ADDR, &[0xF4, 0x27]).await; // ctrl_meas: osrs_t=1,osrs_p=1,mode=normal

    let mut env = EnvironmentData::default();

    loop {
        // ── RS485: Read NPK / moisture / temperature from Modbus sensor ──
        let request = modbus.build_npk_read_request();
        de_pin.set_high(); // transmit mode
        let tx_ok = rs485.write(request.as_slice()).await.is_ok();
        Timer::after(Duration::from_millis(1)).await; // RS485 turnaround
        de_pin.set_low(); // receive mode

        if tx_ok {
            let mut resp = [0u8; 21];
            match embassy_time::with_timeout(
                Duration::from_millis(200),
                rs485.read(&mut resp),
            )
            .await
            {
                Ok(Ok(())) => {
                    if let Some(modbus_resp) = modbus.parse_response(&resp) {
                        let soil = SoilData::from_npk_response(&modbus_resp);
                        let pkt = SensorPacket {
                            soil_moisture: (soil.moisture_pct * 10.0) as u16,
                            soil_temp:     (soil.temperature_c * 100.0) as i16,
                            soil_n:        soil.nitrogen_mg_kg,
                            soil_p:        soil.phosphorus_mg_kg,
                            soil_k:        soil.potassium_mg_kg,
                            soil_ph:       (soil.ph * 100.0) as u16,
                            soil_ec:       soil.ec_us_cm,
                            air_temp:      env.air_temp,
                            humidity:      env.humidity,
                            pressure:      env.pressure,
                            light_lux:     env.light_lux,
                            uv_index:      env.uv_index,
                            wind_speed:    env.wind_speed,
                            rainfall:      env.rainfall,
                        };
                        let _ = SENSOR_CHANNEL.try_send(pkt);
                    }
                }
                Ok(Err(e)) => warn!("RS485 read error: {:?}", e),
                Err(_)     => warn!("RS485 response timeout"),
            }
        }

        // ── I2C: Read BME280 temperature + pressure + humidity ──────────
        // Registers 0xF7–0xFE: press_msb, press_lsb, press_xlsb,
        //                       temp_msb,  temp_lsb,  temp_xlsb,
        //                       hum_msb,   hum_lsb
        let mut raw = [0u8; 8];
        if i2c.write_read(BME280_ADDR, &[0xF7], &mut raw).await.is_ok() {
            let adc_t = ((raw[3] as u32) << 12) | ((raw[4] as u32) << 4) | ((raw[5] as u32) >> 4);
            let adc_p = ((raw[0] as u32) << 12) | ((raw[1] as u32) << 4) | ((raw[2] as u32) >> 4);
            let adc_h = ((raw[6] as u16) << 8) | raw[7] as u16;
            // NOTE: Proper BME280 compensation requires reading 18 bytes of factory
            // trim registers from 0x88–0xA1 and 0xE1–0xE7 and applying Bosch's
            // signed 64-bit compensation formula.  The linear approximations below
            // give ±5 °C / ±20 hPa accuracy and are suitable for early integration
            // testing; replace with the full compensation when trim data is available.
            env.air_temp = (adc_t as i32 * 100 / 5120) as i16; // °C × 100
            env.pressure = adc_p / 256;                          // Pa (approx)
            env.humidity = adc_h / 512;                          // % (approx)
        }

        // ── I2C: Read BH1750 light intensity ────────────────────────────
        let mut lux_buf = [0u8; 2];
        if i2c.read(BH1750_ADDR, &mut lux_buf).await.is_ok() {
            let raw_lux = ((lux_buf[0] as u16) << 8) | lux_buf[1] as u16;
            env.light_lux = (raw_lux as u32 * 10 / 12) as u16; // count / 1.2 = lux
        }

        // Read all sensors at 1 Hz
        Timer::after(Duration::from_secs(1)).await;
    }
}

// ─── Navigation Task ─────────────────────────────────────────────────
// Waits for fresh GNSS fixes, updates the EKF, and in autonomous mode
// runs the pure-pursuit path planner to generate drive commands.

#[embassy_executor::task]
async fn navigation_task() {
    use navigation::ekf::Ekf;

    info!("Navigation task started (EKF sensor fusion + path planning)");

    // Scale factor: rad/s → (deg/s × 100).  = (180/π) × 100 ≈ 5729.58
    const RAD_TO_DEG_SCALED: f32 = 5729.58;

    let mut ekf = Ekf::new(0.02); // 50 Hz nominal prediction step
    let mut planner = PathPlanner::new();
    let mut mode = RobotMode::Idle;

    loop {
        // Block until the GNSS task delivers a new fix
        let nav = NAV_STATE.wait().await;

        // Update EKF with GPS measurement when fix is valid
        if nav.fix_quality >= 1 {
            ekf.update_gps(nav.x, nav.y);
        }

        // Check for mode change signal (non-blocking)
        if MODE_SIGNAL.signaled() {
            mode = MODE_SIGNAL.wait().await;
            info!("Navigation: mode → {:?}", mode);
            if mode == RobotMode::Autonomous {
                planner.start();
            }
        }

        // In autonomous / deseeding modes run the path planner
        if mode == RobotMode::Autonomous || mode == RobotMode::Deseeding {
            let [x, y, theta, ..] = ekf.state;
            let drive_cmd = planner.compute(x, y, theta);

            if drive_cmd.stop && planner.state == navigation::path_planner::MissionState::Completed {
                info!("Navigation: mission complete");
                MODE_SIGNAL.signal(RobotMode::Idle);
            } else if !drive_cmd.stop {
                // Convert to protocol RobotCommand (linear in mm/s, angular in deg/s × 100)
                let cmd = RobotCommand {
                    cmd_type:    CommandType::Drive,
                    linear_vel:  (drive_cmd.linear * 1000.0) as i16,
                    angular_vel: (drive_cmd.angular * RAD_TO_DEG_SCALED) as i16,
                    deseeder_on: drive_cmd.deseed,
                    probe_deploy: false,
                };
                let _ = COMMAND_CHANNEL.try_send(cmd);
            }
        }

        // Return-to-home mode: add single waypoint to home and navigate
        if mode == RobotMode::ReturnToHome {
            planner.return_to_home();
            let [x, y, theta, ..] = ekf.state;
            let drive_cmd = planner.compute(x, y, theta);
            if !drive_cmd.stop {
                let cmd = RobotCommand {
                    cmd_type:    CommandType::Drive,
                    linear_vel:  (drive_cmd.linear * 1000.0) as i16,
                    angular_vel: (drive_cmd.angular * RAD_TO_DEG_SCALED) as i16,
                    deseeder_on: false,
                    probe_deploy: false,
                };
                let _ = COMMAND_CHANNEL.try_send(cmd);
            }
        }

        Timer::after(Duration::from_millis(20)).await;
    }
}

// ─── LoRa Communication Task ─────────────────────────────────────────
// Handles all radio communication with the handheld controller:
// - Receives commands and routes them to COMMAND_CHANNEL / MODE_SIGNAL
// - Transmits pending telemetry and sensor packets
// - Sends periodic heartbeat at 1 Hz

#[embassy_executor::task]
async fn comms_task(mut radio: LoraRadio) {
    info!("Comms task started (LoRa SX1276 @ 866 MHz)");

    // Initialize the radio (resets chip, programs modem registers)
    radio.init(LoRaConfig::default()).await;
    radio.start_receive();

    let mut mgr = CommsManager::new();
    mgr.lora_status = LinkStatus::Active;

    let mut rx_buf = [0u8; 256];
    let mut tx_buf = [0u8; 256];
    let mut last_heartbeat_ms: u64 = 0;
    const HEARTBEAT_INTERVAL_MS: u64 = 1_000;

    loop {
        let now_ms = Instant::now().as_millis();

        // ── Receive (non-blocking poll) ───────────────────────────────
        if let Some(rx_len) = radio.receive(&mut rx_buf) {
            if let Some(pkt) = Packet::deserialize(&rx_buf[..rx_len]) {
                mgr.adr.report_success(radio.last_packet_rssi());
                mgr.lora_metrics.rx_count += 1;
                mgr.lora_metrics.last_rx_ms = 0;

                match pkt.msg_id {
                    MessageId::ManualCommand if pkt.payload.len() >= 4 => {
                        let linear  = i16::from_le_bytes([pkt.payload[0], pkt.payload[1]]);
                        let angular = i16::from_le_bytes([pkt.payload[2], pkt.payload[3]]);
                        let cmd = RobotCommand {
                            cmd_type:    CommandType::Drive,
                            linear_vel:  linear,
                            angular_vel: angular,
                            deseeder_on: false,
                            probe_deploy: false,
                        };
                        let _ = COMMAND_CHANNEL.try_send(cmd);
                    }
                    MessageId::ModeCommand if !pkt.payload.is_empty() => {
                        let mode_byte = pkt.payload[0];
                        let new_mode = match mode_byte {
                            0 => RobotMode::Idle,
                            1 => RobotMode::Manual,
                            2 => RobotMode::Autonomous,
                            3 => RobotMode::Deseeding,
                            4 => RobotMode::ReturnToHome,
                            5 => {
                                ESTOP_SIGNAL.signal(true);
                                RobotMode::EmergencyStop
                            }
                            6 => RobotMode::SoilSampling,
                            _ => RobotMode::Idle,
                        };
                        CURRENT_MODE_ATOMIC.store(mode_byte, Ordering::Relaxed);
                        MODE_SIGNAL.signal(new_mode);
                    }
                    MessageId::Emergency => {
                        error!("Emergency stop received via LoRa");
                        ESTOP_SIGNAL.signal(true);
                    }
                    MessageId::Ping => {
                        // Respond with a heartbeat carrying the current mode and SoC
                        let mode_val = CURRENT_MODE_ATOMIC.load(Ordering::Relaxed);
                        let soc_val  = BATTERY_SOC_ATOMIC.load(Ordering::Relaxed);
                        let ack = mgr.build_heartbeat(mode_val, soc_val);
                        let ack_len = ack.serialize(&mut tx_buf);
                        radio.transmit(&tx_buf[..ack_len]);
                        radio.start_receive();
                    }
                    _ => {}
                }
            } else {
                mgr.adr.report_failure();
                mgr.lora_metrics.rx_errors += 1;
            }
        }

        // ── Transmit: pending telemetry ───────────────────────────────
        if let Ok(tel) = TELEMETRY_CHANNEL.try_receive() {
            let seq = mgr.next_seq();
            // 20-byte telemetry payload (big-endian lat/lon, little-endian rest)
            let payload = [
                (tel.lat >> 24) as u8, (tel.lat >> 16) as u8,
                (tel.lat >>  8) as u8,  tel.lat as u8,
                (tel.lon >> 24) as u8, (tel.lon >> 16) as u8,
                (tel.lon >>  8) as u8,  tel.lon as u8,
                (tel.speed   >> 8) as u8,   tel.speed as u8,
                (tel.heading >> 8) as u8, tel.heading as u8,
                (tel.battery_mv >> 8) as u8, tel.battery_mv as u8,
                tel.battery_soc, tel.mode, tel.sat_count,
                (tel.hdop >> 8) as u8, tel.hdop as u8,
                tel.rssi as u8,
            ];
            let pkt = Packet::new(ADDR_ROBOT, ADDR_CONTROLLER, MessageId::Telemetry, &payload, seq);
            let pkt_len = pkt.serialize(&mut tx_buf);
            if radio.transmit(&tx_buf[..pkt_len]) {
                mgr.lora_metrics.tx_count += 1;
            } else {
                mgr.lora_metrics.tx_errors += 1;
            }
            radio.start_receive();
        }

        // ── Transmit: pending sensor data ─────────────────────────────
        if let Ok(sensor) = SENSOR_CHANNEL.try_receive() {
            let seq = mgr.next_seq();
            // 14-byte soil payload (see docs/07-protocol-alignment.md §5.1)
            let payload = [
                (sensor.soil_moisture >> 8) as u8, sensor.soil_moisture as u8,
                (sensor.soil_temp    >> 8) as u8,  sensor.soil_temp     as u8,
                (sensor.soil_n  >> 8) as u8, sensor.soil_n  as u8,
                (sensor.soil_p  >> 8) as u8, sensor.soil_p  as u8,
                (sensor.soil_k  >> 8) as u8, sensor.soil_k  as u8,
                (sensor.soil_ph >> 8) as u8, sensor.soil_ph as u8,
                (sensor.soil_ec >> 8) as u8, sensor.soil_ec as u8,
            ];
            let pkt = Packet::new(ADDR_ROBOT, ADDR_CONTROLLER, MessageId::SoilData, &payload, seq);
            let pkt_len = pkt.serialize(&mut tx_buf);
            if radio.transmit(&tx_buf[..pkt_len]) {
                mgr.lora_metrics.tx_count += 1;
            }
            radio.start_receive();
        }

        // ── Periodic: Heartbeat at 1 Hz ───────────────────────────────
        if now_ms.saturating_sub(last_heartbeat_ms) >= HEARTBEAT_INTERVAL_MS {
            last_heartbeat_ms = now_ms;
            let mode_val = CURRENT_MODE_ATOMIC.load(Ordering::Relaxed);
            let soc_val  = BATTERY_SOC_ATOMIC.load(Ordering::Relaxed);
            let hb = mgr.build_heartbeat(mode_val, soc_val);
            let hb_len = hb.serialize(&mut tx_buf);
            radio.transmit(&tx_buf[..hb_len]);
            radio.start_receive();
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}

// ─── Motor Control Task ───────────────────────────────────────────────
// Reads RobotCommand from COMMAND_CHANNEL, converts to wheel RPMs via
// differential-drive kinematics, and issues CAN frames to the four
// BLDC motor controllers via FDCAN1.  Controls the deseeder BTS7960
// H-bridge via TIM3 PWM and soil-probe servos via TIM4 PWM.

/// Build a classic CAN data frame.  Returns None only when `id > 0x7FF`
/// (impossible with the fixed CAN IDs used here) or `data.len() > 8`.
fn make_can_frame(id: u16, data: &[u8]) -> Option<ClassicFrame> {
    let sid = StandardId::new(id)?;
    let header = Header::new(Id::Standard(sid), data.len() as u8, false);
    ClassicFrame::new(header, data).ok()
}

#[embassy_executor::task]
async fn motor_control_task(
    mut can_tx:       CanTx,
    mut deseeder_pwm: DeseedPwm,
    mut servo_pwm:    ServoPwm,
) {
    use control::can_messages::{
        build_speed_command,
        MOTOR_FL_CMD, MOTOR_FR_CMD, MOTOR_RL_CMD, MOTOR_RR_CMD,
        DESEEDER_CMD, ESTOP_BROADCAST,
    };

    info!("Motor control task started (4× BLDC via FDCAN1, BTS7960 deseeder, 2× servo)");

    let drive = DifferentialDrive::new(
        0.55,   // track width  550 mm
        0.15,   // wheel diam   150 mm
        3000,   // max RPM
        20.0,   // gear ratio   20:1
    );

    let mut state        = DriveState::new();
    let mut estop_active = false;
    let mut deseed_on    = false;
    let mut probe_out    = false;

    // Per-motor maximum current — conservative for 15 A BLDC controllers
    const MAX_DRIVE_MA:  u16 = 15_000;
    const MAX_DESEED_MA: u16 = 20_000;
    // Deseeder operating RPM (forward, one-directional)
    const DESEED_RPM: i16 = 1_200;

    // ── Servo pulse-width duty cycles at 50 Hz (20 ms period) ────────
    // Standard RC servo:  1 ms → deployed (probe down),  2 ms → retracted
    // duty = pulse_ms / 20_ms × max_duty
    let servo_max = servo_pwm.get_max_duty() as u32;
    let duty_deployed  = ((servo_max * 1_000) / 20_000) as u16; // 5 %
    let duty_retracted = ((servo_max * 2_000) / 20_000) as u16; // 10 %

    // Initialise servos to retracted (safe, clear of soil surface)
    servo_pwm.set_duty(Channel::Ch1, duty_retracted);
    servo_pwm.set_duty(Channel::Ch2, duty_retracted);
    servo_pwm.enable(Channel::Ch1);
    servo_pwm.enable(Channel::Ch2);

    // Deseeder starts idle — both BTS7960 inputs low, channels disabled
    deseeder_pwm.set_duty(Channel::Ch1, 0);
    deseeder_pwm.set_duty(Channel::Ch2, 0);

    loop {
        // ── E-STOP (non-blocking) ─────────────────────────────────────
        if ESTOP_SIGNAL.signaled() {
            estop_active = ESTOP_SIGNAL.wait().await;
            if estop_active {
                error!("Motor control: E-STOP activated — broadcasting CAN ESTOP");
                // DLC-0 broadcast on 0x7FF; every motor controller enters safe-stop
                if let Some(frame) = make_can_frame(ESTOP_BROADCAST, &[]) {
                    can_tx.write(&frame).await;
                }
                // Zero drive and deseeder state
                state.front_left.target_rpm  = 0;
                state.front_right.target_rpm = 0;
                state.rear_left.target_rpm   = 0;
                state.rear_right.target_rpm  = 0;
                state.deseeder.target_rpm    = 0;
                // Disable deseeder PWM immediately
                deseeder_pwm.disable(Channel::Ch1);
                deseeder_pwm.disable(Channel::Ch2);
                deseed_on = false;
                CURRENT_MODE_ATOMIC.store(RobotMode::EmergencyStop as u8, Ordering::Relaxed);
            } else {
                info!("Motor control: E-STOP cleared");
                estop_active = false;
            }
        }

        if estop_active {
            Timer::after(Duration::from_millis(50)).await;
            continue;
        }

        // ── Process next drive/actuator command (non-blocking) ────────
        if let Ok(cmd) = COMMAND_CHANNEL.try_receive() {
            match cmd.cmd_type {

                CommandType::Drive => {
                    // Convert protocol units → SI
                    let linear_m_s    = cmd.linear_vel  as f32 / 1_000.0; // mm/s → m/s
                    let angular_rad_s = cmd.angular_vel as f32 / 5_729.6; // (deg/s × 100) → rad/s
                    let (left_rpm, right_rpm) =
                        drive.velocity_to_wheel_rpm(linear_m_s, angular_rad_s);

                    state.front_left.target_rpm  = left_rpm;
                    state.rear_left.target_rpm   = left_rpm;
                    state.front_right.target_rpm = right_rpm;
                    state.rear_right.target_rpm  = right_rpm;

                    debug!(
                        "Drive cmd: lin={}mm/s ang={} L={}rpm R={}rpm",
                        cmd.linear_vel, cmd.angular_vel, left_rpm, right_rpm
                    );

                    // Send speed commands to all four BLDC controllers via FDCAN1
                    let fl = build_speed_command(left_rpm,  MAX_DRIVE_MA);
                    let fr = build_speed_command(right_rpm, MAX_DRIVE_MA);
                    for (id, data) in [
                        (MOTOR_FL_CMD, fl),
                        (MOTOR_RL_CMD, fl),
                        (MOTOR_FR_CMD, fr),
                        (MOTOR_RR_CMD, fr),
                    ] {
                        if let Some(frame) = make_can_frame(id, &data) {
                            can_tx.write(&frame).await;
                        }
                    }
                }

                CommandType::EmergencyStop => {
                    ESTOP_SIGNAL.signal(true);
                }

                CommandType::StartDeseeding => {
                    if !deseed_on {
                        deseed_on = true;
                        info!("Deseeder: starting at {} rpm", DESEED_RPM);

                        // Notify the deseeder CAN node (brushless deseeder controller)
                        let data = build_speed_command(DESEED_RPM, MAX_DESEED_MA);
                        if let Some(frame) = make_can_frame(DESEEDER_CMD, &data) {
                            can_tx.write(&frame).await;
                        }

                        // BTS7960: R_PWM = 70 % forward, L_PWM = 0
                        let max = deseeder_pwm.get_max_duty() as u32;
                        let fwd_duty = ((max * 70) / 100) as u16;
                        deseeder_pwm.set_duty(Channel::Ch1, fwd_duty);
                        deseeder_pwm.set_duty(Channel::Ch2, 0);
                        deseeder_pwm.enable(Channel::Ch1);
                        deseeder_pwm.enable(Channel::Ch2);
                        state.deseeder.target_rpm = DESEED_RPM;
                    }
                }

                CommandType::StopDeseeding => {
                    if deseed_on {
                        deseed_on = false;
                        info!("Deseeder: stopping");

                        // Ramp to zero via CAN, then kill PWM
                        let data = build_speed_command(0, MAX_DESEED_MA);
                        if let Some(frame) = make_can_frame(DESEEDER_CMD, &data) {
                            can_tx.write(&frame).await;
                        }
                        deseeder_pwm.set_duty(Channel::Ch1, 0);
                        deseeder_pwm.set_duty(Channel::Ch2, 0);
                        deseeder_pwm.disable(Channel::Ch1);
                        deseeder_pwm.disable(Channel::Ch2);
                        state.deseeder.target_rpm = 0;
                    }
                }

                CommandType::DeployProbe => {
                    if !probe_out {
                        probe_out = true;
                        info!("Probe: deploying (servo → 1 ms pulse)");
                        // Drive both servos to deployed angle (1 ms = 5 % duty)
                        servo_pwm.set_duty(Channel::Ch1, duty_deployed);
                        servo_pwm.set_duty(Channel::Ch2, duty_deployed);
                        // Allow 1.5 s for mechanical travel
                        Timer::after(Duration::from_millis(1_500)).await;
                        info!("Probe: deployed");
                    }
                }

                CommandType::RetractProbe => {
                    if probe_out {
                        probe_out = false;
                        info!("Probe: retracting (servo → 2 ms pulse)");
                        // Return servos to retracted angle (2 ms = 10 % duty)
                        servo_pwm.set_duty(Channel::Ch1, duty_retracted);
                        servo_pwm.set_duty(Channel::Ch2, duty_retracted);
                        Timer::after(Duration::from_millis(1_500)).await;
                        info!("Probe: retracted");
                    }
                }

                _ => {}
            }
        }

        // Control loop: 50 Hz
        Timer::after(Duration::from_millis(20)).await;
    }
}

// ─── CAN RX Task ─────────────────────────────────────────────────────
// Continuously receives frames from FDCAN1 and processes:
//   - BMS status (0x300) → updates BATTERY_SOC_ATOMIC
//   - Motor fault feedback (0x180–0x183) → logged as warnings

#[embassy_executor::task]
async fn can_rx_task(mut can_rx: CanRx) {
    use control::can_messages::{BMS_STATUS, MOTOR_FL_STATUS, MOTOR_FR_STATUS, MOTOR_RL_STATUS, MOTOR_RR_STATUS};

    info!("CAN RX task started (BMS + motor feedback monitoring)");

    loop {
        match can_rx.read().await {
            Ok(frame) => {
                let raw_id = match frame.header().id() {
                    Id::Standard(sid) => sid.as_raw() as u32,
                    Id::Extended(eid) => eid.as_raw(),
                };
                let data = frame.data();

                match raw_id as u16 {
                    id if id == BMS_STATUS && data.len() >= 5 => {
                        // BMS payload: [volt_hi, volt_lo, curr_hi, curr_lo, soc, temp, fault, 0]
                        let soc = data[4];
                        BATTERY_SOC_ATOMIC.store(soc, Ordering::Relaxed);
                        if soc < 15 {
                            warn!("Battery low: {}%", soc);
                        }
                    }
                    id if (id == MOTOR_FL_STATUS
                        || id == MOTOR_FR_STATUS
                        || id == MOTOR_RL_STATUS
                        || id == MOTOR_RR_STATUS)
                        && data.len() >= 6 =>
                    {
                        use control::can_messages::parse_motor_status;
                        let mut buf = [0u8; 8];
                        let copy_len = data.len().min(8);
                        buf[..copy_len].copy_from_slice(&data[..copy_len]);
                        let (actual_rpm, _current, _temp, fault) = parse_motor_status(&buf);
                        if fault != 0 {
                            error!("Motor CAN id=0x{:03X} fault=0x{:02X} rpm={}", id, fault, actual_rpm);
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                warn!("CAN RX error: {:?}", e);
                Timer::after(Duration::from_millis(10)).await;
            }
        }
    }
}

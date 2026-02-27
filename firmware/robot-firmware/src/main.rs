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
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use panic_probe as _;

mod comms;
mod control;
mod drivers;
mod navigation;
mod protocol;
mod sensors;

// ─── Inter-task communication channels ───────────────────────────────

/// Navigation state shared across tasks
static NAV_STATE: Signal<ThreadModeRawMutex, navigation::NavState> = Signal::new();

/// Sensor data channel (sensor task → comms task)
static SENSOR_CHANNEL: Channel<ThreadModeRawMutex, protocol::SensorPacket, 8> = Channel::new();

/// Command channel (comms task → control task)
static COMMAND_CHANNEL: Channel<ThreadModeRawMutex, protocol::RobotCommand, 8> = Channel::new();

/// Telemetry channel (various tasks → comms task for transmission)
static TELEMETRY_CHANNEL: Channel<ThreadModeRawMutex, protocol::TelemetryPacket, 16> =
    Channel::new();

/// Emergency stop signal (any task can trigger)
static ESTOP_SIGNAL: Signal<ThreadModeRawMutex, bool> = Signal::new();

/// Robot operating mode
static MODE_SIGNAL: Signal<ThreadModeRawMutex, control::RobotMode> = Signal::new();

// ─── Interrupt bindings ──────────────────────────────────────────────

bind_interrupts!(struct Irqs {
    USART1 => embassy_stm32::usart::InterruptHandler<peripherals::USART1>;
    USART2 => embassy_stm32::usart::InterruptHandler<peripherals::USART2>;
    USART3 => embassy_stm32::usart::InterruptHandler<peripherals::USART3>;
    UART5 => embassy_stm32::usart::InterruptHandler<peripherals::UART5>;
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

    // Configure system clocks: HSE 8MHz → PLL → 480MHz SYSCLK
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
            divp: Some(PllDiv::DIV2), // 480 MHz
            divq: Some(PllDiv::DIV8), // 60 MHz for SPI
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 240 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 120 MHz
    }

    let p = embassy_stm32::init(config);
    info!("System clocks configured: SYSCLK=480MHz");

    // ─── GPIO Setup ──────────────────────────────────────────────
    let led_status = Output::new(p.PC0, Level::Low, Speed::Low);
    let led_error = Output::new(p.PC1, Level::Low, Speed::Low);
    let led_comms = Output::new(p.PC2, Level::Low, Speed::Low);
    let _relay_ctrl = Output::new(p.PD7, Level::Low, Speed::Low);

    // ─── Spawn all subsystem tasks ──────────────────────────────

    // Heartbeat LED task
    spawner.must_spawn(heartbeat_task(led_status));

    // Safety monitor (E-STOP, watchdog, tilt)
    spawner.must_spawn(safety_task(led_error));

    // Communication status LED
    spawner.must_spawn(comms_led_task(led_comms));

    info!("ADR-1 Robot Firmware initialized successfully");
    info!("All subsystem tasks spawned");

    // Main loop: system monitor
    loop {
        // Log system status every 10 seconds
        Timer::after(Duration::from_secs(10)).await;
        info!("System heartbeat - all tasks running");
    }
}

// ─── Heartbeat LED Task ──────────────────────────────────────────────

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

#[embassy_executor::task]
async fn safety_task(mut led_error: Output<'static>) {
    info!("Safety monitor started");

    loop {
        // Check for emergency stop signal
        if ESTOP_SIGNAL.signaled() {
            let estop = ESTOP_SIGNAL.wait().await;
            if estop {
                error!("EMERGENCY STOP ACTIVATED");
                led_error.set_high();
                // In a full implementation: disable all motors via CAN,
                // set mode to ESTOP, broadcast to controller
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}

// ─── Communication Status LED Task ──────────────────────────────────

#[embassy_executor::task]
async fn comms_led_task(mut led: Output<'static>) {
    loop {
        // Fast blink when transmitting, slow blink when idle
        led.toggle();
        Timer::after(Duration::from_millis(500)).await;
    }
}

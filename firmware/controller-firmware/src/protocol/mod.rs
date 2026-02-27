//! Controller Protocol Types
//!
//! Shared data structures for controller-robot communication.
//! Uses the same wire protocol as the robot firmware.

use heapless::Vec;
use serde::{Deserialize, Serialize};

/// Maximum payload size for LoRa packets
pub const MAX_PAYLOAD_SIZE: usize = 200;

/// Sync word for packet framing
pub const SYNC_WORD: u16 = 0xAD01;

/// Device addresses
pub const ADDR_ROBOT: u8 = 0x01;
pub const ADDR_CONTROLLER: u8 = 0x02;

// ─── Input from controller hardware ─────────────────────────────────

/// Raw controller input state
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct ControllerInput {
    /// Left joystick X axis (-1000 to 1000, left negative)
    pub joy_left_x: i16,
    /// Left joystick Y axis (-1000 to 1000, backward negative)
    pub joy_left_y: i16,
    /// Right joystick X axis (-1000 to 1000)
    pub joy_right_x: i16,
    /// Right joystick Y axis (-1000 to 1000)
    pub joy_right_y: i16,
    /// Button state bitmap
    pub buttons: u16,
    /// Button press events (edge-triggered)
    pub button_events: u16,
}

/// Button bit positions
pub mod buttons {
    pub const MODE: u16 = 1 << 0;
    pub const HOME: u16 = 1 << 1;
    pub const MENU: u16 = 1 << 2;
    pub const L1: u16 = 1 << 3;
    pub const L2: u16 = 1 << 4;
    pub const R1: u16 = 1 << 5;
    pub const R2: u16 = 1 << 6;
    pub const JOY_L_BTN: u16 = 1 << 7;
    pub const JOY_R_BTN: u16 = 1 << 8;
    pub const ESTOP: u16 = 1 << 15; // Highest bit for E-STOP
}

impl ControllerInput {
    /// Convert joystick input to robot drive command
    /// Left stick Y = forward/backward, Right stick X = turn
    pub fn to_drive_command(&self) -> DriveCommand {
        // Scale joystick to velocity
        // Left Y → linear velocity (mm/s)
        // Right X → angular velocity (deg/s × 100)
        let max_linear_vel: i16 = 1500; // 1.5 m/s in mm/s
        let max_angular_vel: i16 = 9000; // 90 deg/s × 100

        let linear = (self.joy_left_y as i32 * max_linear_vel as i32 / 1000) as i16;
        let angular = (self.joy_right_x as i32 * max_angular_vel as i32 / 1000) as i16;

        DriveCommand { linear, angular }
    }

    /// Check if a button was just pressed (edge detection)
    pub fn button_pressed(&self, button: u16) -> bool {
        self.button_events & button != 0
    }

    /// Check if a button is currently held
    pub fn button_held(&self, button: u16) -> bool {
        self.buttons & button != 0
    }
}

/// Drive command to send to robot
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct DriveCommand {
    /// Linear velocity (mm/s, positive = forward)
    pub linear: i16,
    /// Angular velocity (deg/s × 100, positive = left turn)
    pub angular: i16,
}

// ─── Telemetry received from robot ──────────────────────────────────

/// Complete telemetry state from robot
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct RobotTelemetry {
    // Navigation
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub speed_kmh: f32,
    pub heading: f32,
    pub sat_count: u8,
    pub hdop: f32,

    // Battery
    pub battery_mv: u16,
    pub battery_soc: u8,

    // Operating state
    pub mode: u8,
    pub link_rssi: i16,

    // Sensors
    pub soil_moisture: f32,
    pub soil_temp: f32,
    pub soil_n: u16,
    pub soil_p: u16,
    pub soil_k: u16,
    pub soil_ph: f32,
    pub soil_ec: u16,

    // Environment
    pub air_temp: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub light_lux: u16,
    pub wind_speed: f32,
    pub rainfall: f32,

    // Plant health
    pub ndvi: f32,
    pub leaf_temp: f32,
    pub plant_height: u16,
    pub health_score: u16,
    pub disease_alert: bool,

    // Connection
    pub connected: bool,
    pub last_update_ms: u64,
}

impl RobotTelemetry {
    /// Mode as string
    pub fn mode_str(&self) -> &'static str {
        match self.mode {
            0 => "IDLE",
            1 => "MANUAL",
            2 => "AUTO",
            3 => "DESEED",
            4 => "RTH",
            5 => "ESTOP",
            6 => "SAMPLE",
            _ => "UNKNOWN",
        }
    }

    /// Battery level category
    pub fn battery_level(&self) -> BatteryLevel {
        match self.battery_soc {
            0..=10 => BatteryLevel::Critical,
            11..=25 => BatteryLevel::Low,
            26..=75 => BatteryLevel::Medium,
            76..=100 => BatteryLevel::Full,
            _ => BatteryLevel::Unknown,
        }
    }

    /// Health score as category
    pub fn health_category(&self) -> &'static str {
        match self.health_score {
            0..=200 => "CRITICAL",
            201..=400 => "POOR",
            401..=600 => "FAIR",
            601..=800 => "GOOD",
            801..=1000 => "EXCELLENT",
            _ => "N/A",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum BatteryLevel {
    Critical,
    Low,
    Medium,
    Full,
    Unknown,
}

// ─── Packet parsing (same as robot protocol) ────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum MessageId {
    Heartbeat = 0x01,
    Telemetry = 0x02,
    PlantHealth = 0x03,
    SoilData = 0x04,
    NavStatus = 0x05,
    ManualCommand = 0x10,
    Waypoint = 0x11,
    ModeCommand = 0x12,
    Config = 0x13,
    Ack = 0x20,
    Nack = 0x21,
    Emergency = 0xFE,
    OtaData = 0xFF,
}

/// Build a manual drive command packet
pub fn build_drive_packet(cmd: &DriveCommand, seq: u16) -> Vec<u8, 16> {
    let mut payload: Vec<u8, 16> = Vec::new();
    let _ = payload.extend_from_slice(&cmd.linear.to_le_bytes());
    let _ = payload.extend_from_slice(&cmd.angular.to_le_bytes());
    payload
}

/// Build a mode command packet
pub fn build_mode_packet(mode: u8, seq: u16) -> Vec<u8, 4> {
    let mut payload: Vec<u8, 4> = Vec::new();
    let _ = payload.push(mode);
    payload
}

/// Build emergency stop packet
pub fn build_estop_packet(seq: u16) -> Vec<u8, 4> {
    let payload: Vec<u8, 4> = Vec::new();
    payload
}

/// CRC-16/CCITT (same as robot)
pub fn compute_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

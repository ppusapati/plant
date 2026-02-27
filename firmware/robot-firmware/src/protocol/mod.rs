//! ADR-1 Communication Protocol
//!
//! Defines all message types, packet formats, and serialization
//! for LoRa/BLE/WiFi communication between Robot and Controller.

use heapless::Vec;
use serde::{Deserialize, Serialize};

/// Maximum payload size for LoRa packets (SX1276 FIFO = 256 bytes)
pub const MAX_PAYLOAD_SIZE: usize = 200;

/// Sync word for packet framing
pub const SYNC_WORD: u16 = 0xAD01;

/// Device addresses
pub const ADDR_ROBOT: u8 = 0x01;
pub const ADDR_CONTROLLER: u8 = 0x02;
pub const ADDR_BROADCAST: u8 = 0xFF;

// ─── Message IDs ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, defmt::Format)]
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

// ─── Packet Frame ────────────────────────────────────────────────────

/// Wire-format packet structure
/// Layout: [SYNC:2][SRC:1][DST:1][MSG_ID:1][LEN:1][PAYLOAD:0-200][SEQ:2][CRC:2]
#[derive(Debug, Clone, defmt::Format)]
pub struct Packet {
    pub src: u8,
    pub dst: u8,
    pub msg_id: MessageId,
    pub payload: Vec<u8, MAX_PAYLOAD_SIZE>,
    pub seq: u16,
}

impl Packet {
    pub fn new(src: u8, dst: u8, msg_id: MessageId, payload: &[u8], seq: u16) -> Self {
        let mut p = Vec::new();
        let _ = p.extend_from_slice(payload);
        Self {
            src,
            dst,
            msg_id,
            payload: p,
            seq,
        }
    }

    /// Serialize packet to wire format bytes
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let sync_bytes = SYNC_WORD.to_be_bytes();
        buf[0] = sync_bytes[0];
        buf[1] = sync_bytes[1];
        buf[2] = self.src;
        buf[3] = self.dst;
        buf[4] = self.msg_id as u8;
        buf[5] = self.payload.len() as u8;

        let payload_len = self.payload.len();
        buf[6..6 + payload_len].copy_from_slice(&self.payload);

        let seq_offset = 6 + payload_len;
        let seq_bytes = self.seq.to_le_bytes();
        buf[seq_offset] = seq_bytes[0];
        buf[seq_offset + 1] = seq_bytes[1];

        // CRC-16/CCITT over entire frame (excluding CRC field)
        let crc = compute_crc16(&buf[..seq_offset + 2]);
        let crc_bytes = crc.to_le_bytes();
        buf[seq_offset + 2] = crc_bytes[0];
        buf[seq_offset + 3] = crc_bytes[1];

        seq_offset + 4 // Total packet length
    }

    /// Deserialize packet from wire format bytes
    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        if buf.len() < 10 {
            return None;
        }

        // Check sync word
        let sync = u16::from_be_bytes([buf[0], buf[1]]);
        if sync != SYNC_WORD {
            return None;
        }

        let src = buf[2];
        let dst = buf[3];
        let msg_id_raw = buf[4];
        let payload_len = buf[5] as usize;

        if buf.len() < 6 + payload_len + 4 {
            return None;
        }

        // Verify CRC
        let crc_offset = 6 + payload_len + 2;
        let expected_crc = u16::from_le_bytes([buf[crc_offset], buf[crc_offset + 1]]);
        let computed_crc = compute_crc16(&buf[..crc_offset]);
        if expected_crc != computed_crc {
            return None;
        }

        let msg_id = match msg_id_raw {
            0x01 => MessageId::Heartbeat,
            0x02 => MessageId::Telemetry,
            0x03 => MessageId::PlantHealth,
            0x04 => MessageId::SoilData,
            0x05 => MessageId::NavStatus,
            0x10 => MessageId::ManualCommand,
            0x11 => MessageId::Waypoint,
            0x12 => MessageId::ModeCommand,
            0x13 => MessageId::Config,
            0x20 => MessageId::Ack,
            0x21 => MessageId::Nack,
            0xFE => MessageId::Emergency,
            0xFF => MessageId::OtaData,
            _ => return None,
        };

        let mut payload = Vec::new();
        let _ = payload.extend_from_slice(&buf[6..6 + payload_len]);

        let seq_offset = 6 + payload_len;
        let seq = u16::from_le_bytes([buf[seq_offset], buf[seq_offset + 1]]);

        Some(Self {
            src,
            dst,
            msg_id,
            payload,
            seq,
        })
    }
}

// ─── Telemetry Data Structures ───────────────────────────────────────

/// Full telemetry packet sent to controller at 5Hz
#[derive(Debug, Clone, Serialize, Deserialize, defmt::Format)]
pub struct TelemetryPacket {
    /// Latitude in degrees × 1e7 (integer for precision)
    pub lat: i32,
    /// Longitude in degrees × 1e7
    pub lon: i32,
    /// Altitude in mm
    pub alt: i32,
    /// Speed in mm/s
    pub speed: u16,
    /// Heading in degrees × 100
    pub heading: u16,
    /// Battery voltage in mV
    pub battery_mv: u16,
    /// Battery state of charge (0-100%)
    pub battery_soc: u8,
    /// Robot operating mode
    pub mode: u8,
    /// Number of satellites in fix
    pub sat_count: u8,
    /// HDOP × 100
    pub hdop: u16,
    /// RSSI of last received LoRa packet
    pub rssi: i16,
}

/// Sensor data packet
#[derive(Debug, Clone, Serialize, Deserialize, defmt::Format)]
pub struct SensorPacket {
    /// Soil moisture (% × 10)
    pub soil_moisture: u16,
    /// Soil temperature (°C × 100)
    pub soil_temp: i16,
    /// Soil nitrogen (mg/kg)
    pub soil_n: u16,
    /// Soil phosphorus (mg/kg)
    pub soil_p: u16,
    /// Soil potassium (mg/kg)
    pub soil_k: u16,
    /// Soil pH (× 100)
    pub soil_ph: u16,
    /// Soil EC (µS/cm)
    pub soil_ec: u16,
    /// Air temperature (°C × 100)
    pub air_temp: i16,
    /// Relative humidity (% × 100)
    pub humidity: u16,
    /// Barometric pressure (Pa)
    pub pressure: u32,
    /// Light intensity (lux)
    pub light_lux: u16,
    /// UV index (× 100)
    pub uv_index: u16,
    /// Wind speed (m/s × 100)
    pub wind_speed: u16,
    /// Rainfall (mm × 10, cumulative)
    pub rainfall: u16,
}

/// Plant health data packet
#[derive(Debug, Clone, Serialize, Deserialize, defmt::Format)]
pub struct PlantHealthPacket {
    /// NDVI value (× 1000, range -1000 to 1000)
    pub ndvi: i16,
    /// Leaf temperature (°C × 100)
    pub leaf_temp: i16,
    /// Plant height (mm)
    pub plant_height: u16,
    /// Canopy coverage (% × 10)
    pub canopy_coverage: u16,
    /// Composite health score (0-1000)
    pub health_score: u16,
    /// Disease detection flag
    pub disease_alert: bool,
}

/// Robot command from controller
#[derive(Debug, Clone, Serialize, Deserialize, defmt::Format)]
pub struct RobotCommand {
    /// Command type
    pub cmd_type: CommandType,
    /// Linear velocity (mm/s, signed: positive=forward)
    pub linear_vel: i16,
    /// Angular velocity (deg/s × 100, signed: positive=left)
    pub angular_vel: i16,
    /// Deseeding active flag
    pub deseeder_on: bool,
    /// Probe deployment flag
    pub probe_deploy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, defmt::Format)]
#[repr(u8)]
pub enum CommandType {
    Drive = 0x01,
    SetMode = 0x02,
    SetWaypoint = 0x03,
    EmergencyStop = 0x04,
    CalibrateIMU = 0x05,
    CalibrateSoil = 0x06,
    StartDeseeding = 0x07,
    StopDeseeding = 0x08,
    DeployProbe = 0x09,
    RetractProbe = 0x0A,
    ReturnToHome = 0x0B,
}

/// Navigation waypoint
#[derive(Debug, Clone, Serialize, Deserialize, defmt::Format)]
pub struct Waypoint {
    /// Latitude (degrees × 1e7)
    pub lat: i32,
    /// Longitude (degrees × 1e7)
    pub lon: i32,
    /// Action at waypoint
    pub action: WaypointAction,
    /// Speed to waypoint (mm/s)
    pub speed: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, defmt::Format)]
#[repr(u8)]
pub enum WaypointAction {
    PassThrough = 0x00,
    Stop = 0x01,
    Deseed = 0x02,
    SampleSoil = 0x03,
    PhotoCapture = 0x04,
}

// ─── CRC-16/CCITT ────────────────────────────────────────────────────

/// Compute CRC-16/CCITT (polynomial 0x1021, init 0xFFFF)
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

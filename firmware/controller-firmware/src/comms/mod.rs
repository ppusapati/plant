//! Controller Communication Manager
//!
//! Handles LoRa communication with the robot.
//! Manages command transmission and telemetry reception.
//! Implements the same protocol as the robot firmware.

use defmt::*;
use crate::protocol::{self, ControllerInput, DriveCommand, MessageId, RobotTelemetry};
use heapless::Vec;

/// Controller communication state
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ControllerCommsState {
    /// Searching for robot
    Scanning,
    /// Connected, exchanging heartbeats
    Connected,
    /// Connected, actively controlling
    ActiveControl,
    /// Link degraded (high latency/loss)
    Degraded,
    /// Link lost
    Disconnected,
}

/// Controller communication manager
pub struct ControllerComms {
    pub state: ControllerCommsState,
    /// Sequence counter for outgoing packets
    seq_counter: u16,
    /// Last received sequence from robot
    last_rx_seq: u16,
    /// Time since last heartbeat from robot (ms)
    pub heartbeat_timeout_ms: u32,
    /// Telemetry receive rate (packets/sec)
    pub rx_rate: f32,
    /// Command transmit rate (packets/sec)
    pub tx_rate: f32,
    /// Packets sent
    pub tx_count: u32,
    /// Packets received
    pub rx_count: u32,
    /// Retransmit count
    pub retx_count: u32,
    /// Duplicate packets dropped
    pub dup_count: u32,
    /// Pending ACK sequence (-1 if none)
    pending_ack_seq: i32,
    /// Retransmit buffer (last sent packet for retry)
    retx_buffer: Vec<u8, 256>,
    /// Retransmit attempts remaining
    retx_remaining: u8,
}

impl ControllerComms {
    pub fn new() -> Self {
        Self {
            state: ControllerCommsState::Scanning,
            seq_counter: 0,
            last_rx_seq: 0,
            heartbeat_timeout_ms: 0,
            rx_rate: 0.0,
            tx_rate: 0.0,
            tx_count: 0,
            rx_count: 0,
            retx_count: 0,
            dup_count: 0,
            pending_ack_seq: -1,
            retx_buffer: Vec::new(),
            retx_remaining: 0,
        }
    }

    /// Get next sequence number
    pub fn next_seq(&mut self) -> u16 {
        let seq = self.seq_counter;
        self.seq_counter = self.seq_counter.wrapping_add(1);
        seq
    }

    /// Build drive command packet from controller input
    pub fn build_drive_command(&mut self, input: &ControllerInput) -> Vec<u8, 256> {
        let cmd = input.to_drive_command();
        let seq = self.next_seq();

        let mut payload: Vec<u8, 8> = Vec::new();
        let _ = payload.extend_from_slice(&cmd.linear.to_le_bytes());
        let _ = payload.extend_from_slice(&cmd.angular.to_le_bytes());

        self.build_packet(MessageId::ManualCommand, &payload, seq)
    }

    /// Build mode change command
    pub fn build_mode_command(&mut self, mode: u8) -> Vec<u8, 256> {
        let seq = self.next_seq();
        let payload = [mode];
        self.build_packet(MessageId::ModeCommand, &payload, seq)
    }

    /// Build emergency stop packet
    pub fn build_estop_packet(&mut self) -> Vec<u8, 256> {
        let seq = self.next_seq();
        self.build_packet(MessageId::Emergency, &[], seq)
    }

    /// Build a complete wire-format packet
    fn build_packet(&self, msg_id: MessageId, payload: &[u8], seq: u16) -> Vec<u8, 256> {
        let mut buf: Vec<u8, 256> = Vec::new();
        let sync = protocol::SYNC_WORD.to_be_bytes();
        let _ = buf.push(sync[0]);
        let _ = buf.push(sync[1]);
        let _ = buf.push(protocol::ADDR_CONTROLLER);
        let _ = buf.push(protocol::ADDR_ROBOT);
        let _ = buf.push(msg_id as u8);
        let _ = buf.push(payload.len() as u8);
        let _ = buf.extend_from_slice(payload);
        let seq_bytes = seq.to_le_bytes();
        let _ = buf.push(seq_bytes[0]);
        let _ = buf.push(seq_bytes[1]);

        let crc = protocol::compute_crc16(&buf);
        let crc_bytes = crc.to_le_bytes();
        let _ = buf.push(crc_bytes[0]);
        let _ = buf.push(crc_bytes[1]);

        buf
    }

    /// Process received packet and update telemetry
    pub fn process_received(
        &mut self,
        data: &[u8],
        telemetry: &mut RobotTelemetry,
    ) -> Option<MessageId> {
        if data.len() < 10 {
            return None;
        }

        // Verify sync word
        let sync = u16::from_be_bytes([data[0], data[1]]);
        if sync != protocol::SYNC_WORD {
            return None;
        }

        let src = data[2];
        if src != protocol::ADDR_ROBOT {
            return None; // Not from robot
        }

        let msg_id_raw = data[4];
        let payload_len = data[5] as usize;

        if data.len() < 6 + payload_len + 4 {
            return None;
        }

        // Verify CRC
        let crc_offset = 6 + payload_len + 2;
        let expected_crc = u16::from_le_bytes([data[crc_offset], data[crc_offset + 1]]);
        let computed_crc = protocol::compute_crc16(&data[..crc_offset]);
        if expected_crc != computed_crc {
            return None;
        }

        // Check sequence for duplicates
        let seq_offset = 6 + payload_len;
        let seq = u16::from_le_bytes([data[seq_offset], data[seq_offset + 1]]);
        if seq == self.last_rx_seq {
            self.dup_count += 1;
            return None;
        }
        self.last_rx_seq = seq;
        self.rx_count += 1;

        let payload = &data[6..6 + payload_len];
        let msg_id = msg_id_raw;

        // Parse based on message type
        match msg_id {
            0x01 => {
                // Heartbeat
                if payload.len() >= 2 {
                    telemetry.mode = payload[0];
                    telemetry.battery_soc = payload[1];
                }
                telemetry.connected = true;
                self.heartbeat_timeout_ms = 0;
                self.state = ControllerCommsState::Connected;
                Some(MessageId::Heartbeat)
            }
            0x02 => {
                // Telemetry
                self.parse_telemetry(payload, telemetry);
                Some(MessageId::Telemetry)
            }
            0x03 => {
                // Plant health
                self.parse_plant_health(payload, telemetry);
                Some(MessageId::PlantHealth)
            }
            0x04 => {
                // Soil data
                self.parse_soil_data(payload, telemetry);
                Some(MessageId::SoilData)
            }
            0x05 => {
                // Nav status
                self.parse_nav_status(payload, telemetry);
                Some(MessageId::NavStatus)
            }
            0x20 => {
                // ACK
                if payload.len() >= 2 {
                    let acked_seq = u16::from_le_bytes([payload[0], payload[1]]);
                    if self.pending_ack_seq == acked_seq as i32 {
                        self.pending_ack_seq = -1;
                        self.retx_remaining = 0;
                    }
                }
                Some(MessageId::Ack)
            }
            0xFE => {
                // Emergency from robot
                telemetry.mode = 5; // ESTOP
                Some(MessageId::Emergency)
            }
            _ => None,
        }
    }

    fn parse_telemetry(&self, payload: &[u8], telem: &mut RobotTelemetry) {
        if payload.len() < 20 {
            return;
        }
        telem.latitude = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
            as f64 / 1e7;
        telem.longitude = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]])
            as f64 / 1e7;
        telem.altitude = i32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]])
            as f32 / 1000.0;
        telem.speed_kmh = u16::from_le_bytes([payload[12], payload[13]]) as f32 / 1000.0 * 3.6;
        telem.heading = u16::from_le_bytes([payload[14], payload[15]]) as f32 / 100.0;
        telem.battery_mv = u16::from_le_bytes([payload[16], payload[17]]);
        telem.battery_soc = payload[18];
        telem.mode = payload[19];
    }

    fn parse_plant_health(&self, payload: &[u8], telem: &mut RobotTelemetry) {
        if payload.len() < 10 {
            return;
        }
        telem.ndvi = i16::from_le_bytes([payload[0], payload[1]]) as f32 / 1000.0;
        telem.leaf_temp = i16::from_le_bytes([payload[2], payload[3]]) as f32 / 100.0;
        telem.plant_height = u16::from_le_bytes([payload[4], payload[5]]);
        telem.health_score = u16::from_le_bytes([payload[6], payload[7]]);
        telem.disease_alert = payload[8] != 0;
    }

    fn parse_soil_data(&self, payload: &[u8], telem: &mut RobotTelemetry) {
        if payload.len() < 14 {
            return;
        }
        telem.soil_moisture = u16::from_le_bytes([payload[0], payload[1]]) as f32 / 10.0;
        telem.soil_temp = i16::from_le_bytes([payload[2], payload[3]]) as f32 / 100.0;
        telem.soil_n = u16::from_le_bytes([payload[4], payload[5]]);
        telem.soil_p = u16::from_le_bytes([payload[6], payload[7]]);
        telem.soil_k = u16::from_le_bytes([payload[8], payload[9]]);
        telem.soil_ph = u16::from_le_bytes([payload[10], payload[11]]) as f32 / 100.0;
        telem.soil_ec = u16::from_le_bytes([payload[12], payload[13]]);
    }

    fn parse_nav_status(&self, payload: &[u8], telem: &mut RobotTelemetry) {
        if payload.len() < 5 {
            return;
        }
        telem.sat_count = payload[0];
        telem.hdop = u16::from_le_bytes([payload[1], payload[2]]) as f32 / 100.0;
    }

    /// Update timeout counter, called every tick
    pub fn tick(&mut self, elapsed_ms: u32) {
        self.heartbeat_timeout_ms += elapsed_ms;

        if self.heartbeat_timeout_ms > 3000 {
            self.state = ControllerCommsState::Degraded;
        }
        if self.heartbeat_timeout_ms > 10_000 {
            self.state = ControllerCommsState::Disconnected;
        }
    }
}

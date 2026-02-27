//! Communication Manager
//!
//! Manages LoRa, BLE, and WiFi communication links.
//! Implements link priority, failover, and adaptive data rate.

use defmt::*;
use crate::protocol::{MessageId, Packet, ADDR_CONTROLLER, ADDR_ROBOT};

/// Communication link status
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LinkStatus {
    /// Link is active and healthy
    Active,
    /// Link is degraded (high packet loss or weak signal)
    Degraded,
    /// Link is disconnected
    Disconnected,
    /// Link is not available (hardware not present)
    Unavailable,
}

/// Communication link type
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LinkType {
    LoRa,
    Ble,
    WiFi,
}

/// Link health metrics
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct LinkMetrics {
    /// RSSI of last received packet (dBm)
    pub rssi: i16,
    /// Signal-to-noise ratio (dB)
    pub snr: i8,
    /// Packets sent
    pub tx_count: u32,
    /// Packets received
    pub rx_count: u32,
    /// Failed transmissions
    pub tx_errors: u32,
    /// CRC errors on receive
    pub rx_errors: u32,
    /// Time since last received packet (ms)
    pub last_rx_ms: u32,
    /// Round-trip time (ms)
    pub rtt_ms: u16,
}

impl LinkMetrics {
    /// Packet loss rate (0.0 - 1.0)
    pub fn packet_loss_rate(&self) -> f32 {
        if self.tx_count == 0 {
            return 0.0;
        }
        self.tx_errors as f32 / self.tx_count as f32
    }

    /// Check if link is healthy
    pub fn is_healthy(&self) -> bool {
        self.last_rx_ms < 3000 && self.packet_loss_rate() < 0.3
    }
}

/// LoRa data rate profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LoRaProfile {
    Fast,
    Normal,
    LongRange,
}

/// Adaptive data rate controller
pub struct AdaptiveRate {
    pub current_profile: LoRaProfile,
    consecutive_failures: u8,
    consecutive_successes: u8,
    rssi_history: [i16; 8],
    history_idx: usize,
}

impl AdaptiveRate {
    pub fn new() -> Self {
        Self {
            current_profile: LoRaProfile::Normal,
            consecutive_failures: 0,
            consecutive_successes: 0,
            rssi_history: [-80; 8],
            history_idx: 0,
        }
    }

    /// Report a successful transmission/reception
    pub fn report_success(&mut self, rssi: i16) {
        self.consecutive_failures = 0;
        self.consecutive_successes = self.consecutive_successes.saturating_add(1);

        // Update RSSI history
        self.rssi_history[self.history_idx] = rssi;
        self.history_idx = (self.history_idx + 1) % 8;

        // Try to upgrade if signal is strong
        if self.consecutive_successes > 10 && self.avg_rssi() > -100 {
            self.try_upgrade();
        }
    }

    /// Report a failed transmission
    pub fn report_failure(&mut self) {
        self.consecutive_successes = 0;
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);

        // Downgrade after 3 consecutive failures
        if self.consecutive_failures >= 3 {
            self.downgrade();
            self.consecutive_failures = 0;
        }
    }

    /// Average RSSI over recent history
    fn avg_rssi(&self) -> i16 {
        let sum: i32 = self.rssi_history.iter().map(|&r| r as i32).sum();
        (sum / 8) as i16
    }

    /// Try to upgrade to faster profile
    fn try_upgrade(&mut self) {
        self.current_profile = match self.current_profile {
            LoRaProfile::LongRange => {
                info!("ADR: Upgrading to Normal profile");
                LoRaProfile::Normal
            }
            LoRaProfile::Normal => {
                if self.avg_rssi() > -80 {
                    info!("ADR: Upgrading to Fast profile");
                    LoRaProfile::Fast
                } else {
                    LoRaProfile::Normal
                }
            }
            LoRaProfile::Fast => LoRaProfile::Fast,
        };
        self.consecutive_successes = 0;
    }

    /// Downgrade to more robust profile
    fn downgrade(&mut self) {
        self.current_profile = match self.current_profile {
            LoRaProfile::Fast => {
                warn!("ADR: Downgrading to Normal profile");
                LoRaProfile::Normal
            }
            LoRaProfile::Normal => {
                warn!("ADR: Downgrading to Long Range profile");
                LoRaProfile::LongRange
            }
            LoRaProfile::LongRange => {
                error!("ADR: Already at Long Range, link very weak");
                LoRaProfile::LongRange
            }
        };
    }
}

/// Communication manager handles all links and failover
pub struct CommsManager {
    /// LoRa link status
    pub lora_status: LinkStatus,
    /// BLE link status
    pub ble_status: LinkStatus,
    /// WiFi link status
    pub wifi_status: LinkStatus,
    /// LoRa metrics
    pub lora_metrics: LinkMetrics,
    /// BLE metrics
    pub ble_metrics: LinkMetrics,
    /// Adaptive rate controller
    pub adr: AdaptiveRate,
    /// Sequence counter for outgoing packets
    seq_counter: u16,
    /// Active primary link
    pub primary_link: LinkType,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u32,
    /// Last heartbeat sent (ms timestamp)
    pub last_heartbeat_ms: u64,
}

impl CommsManager {
    pub fn new() -> Self {
        Self {
            lora_status: LinkStatus::Disconnected,
            ble_status: LinkStatus::Unavailable,
            wifi_status: LinkStatus::Unavailable,
            lora_metrics: LinkMetrics::default(),
            ble_metrics: LinkMetrics::default(),
            adr: AdaptiveRate::new(),
            seq_counter: 0,
            primary_link: LinkType::LoRa,
            heartbeat_interval_ms: 1000,
            last_heartbeat_ms: 0,
        }
    }

    /// Get next sequence number
    pub fn next_seq(&mut self) -> u16 {
        let seq = self.seq_counter;
        self.seq_counter = self.seq_counter.wrapping_add(1);
        seq
    }

    /// Build a heartbeat packet
    pub fn build_heartbeat(&mut self, mode: u8, battery_soc: u8) -> Packet {
        let payload = [mode, battery_soc];
        let seq = self.next_seq();
        Packet::new(ADDR_ROBOT, ADDR_CONTROLLER, MessageId::Heartbeat, &payload, seq)
    }

    /// Build a telemetry packet from raw telemetry data bytes
    pub fn build_telemetry(&mut self, telemetry_bytes: &[u8]) -> Packet {
        let seq = self.next_seq();
        Packet::new(
            ADDR_ROBOT,
            ADDR_CONTROLLER,
            MessageId::Telemetry,
            telemetry_bytes,
            seq,
        )
    }

    /// Build ACK packet
    pub fn build_ack(&mut self, ack_seq: u16) -> Packet {
        let payload = ack_seq.to_le_bytes();
        let seq = self.next_seq();
        Packet::new(ADDR_ROBOT, ADDR_CONTROLLER, MessageId::Ack, &payload, seq)
    }

    /// Update link status based on metrics
    pub fn update_link_status(&mut self, now_ms: u64) {
        // LoRa status
        self.lora_status = if self.lora_metrics.last_rx_ms > 10_000 {
            LinkStatus::Disconnected
        } else if self.lora_metrics.last_rx_ms > 3_000
            || self.lora_metrics.packet_loss_rate() > 0.3
        {
            LinkStatus::Degraded
        } else {
            LinkStatus::Active
        };

        // Failover logic
        self.primary_link = match self.lora_status {
            LinkStatus::Active => LinkType::LoRa,
            LinkStatus::Degraded => {
                if self.ble_status == LinkStatus::Active {
                    LinkType::Ble
                } else {
                    LinkType::LoRa // Stay on LoRa even if degraded
                }
            }
            LinkStatus::Disconnected => {
                if self.ble_status == LinkStatus::Active {
                    LinkType::Ble
                } else if self.wifi_status == LinkStatus::Active {
                    LinkType::WiFi
                } else {
                    LinkType::LoRa // Keep trying
                }
            }
            LinkStatus::Unavailable => {
                if self.ble_status == LinkStatus::Active {
                    LinkType::Ble
                } else {
                    LinkType::WiFi
                }
            }
        };
    }

    /// Check if communication is completely lost
    pub fn is_comms_lost(&self) -> bool {
        self.lora_status == LinkStatus::Disconnected
            && self.ble_status != LinkStatus::Active
            && self.wifi_status != LinkStatus::Active
    }
}

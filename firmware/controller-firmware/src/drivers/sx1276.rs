//! SX1276 LoRa driver for ESP32-S3 controller
//! Same radio as robot, connected via SPI2 on ESP32-S3.
//! Uses PCB trace antenna (meander IFA) instead of external SMA.

// Re-export the radio driver from the shared protocol
// In production, this would be a shared crate. For now, we duplicate
// the essential parts with ESP32-S3 specific SPI interface.

use defmt::*;

/// SX1276 register addresses (same as robot driver)
pub mod reg {
    pub const FIFO: u8 = 0x00;
    pub const OP_MODE: u8 = 0x01;
    pub const FR_MSB: u8 = 0x06;
    pub const FR_MID: u8 = 0x07;
    pub const FR_LSB: u8 = 0x08;
    pub const PA_CONFIG: u8 = 0x09;
    pub const FIFO_ADDR_PTR: u8 = 0x0D;
    pub const FIFO_TX_BASE: u8 = 0x0E;
    pub const FIFO_RX_BASE: u8 = 0x0F;
    pub const FIFO_RX_CURRENT: u8 = 0x10;
    pub const IRQ_FLAGS: u8 = 0x12;
    pub const RX_NB_BYTES: u8 = 0x13;
    pub const PKT_SNR: u8 = 0x19;
    pub const PKT_RSSI: u8 = 0x1A;
    pub const MODEM_CONFIG_1: u8 = 0x1D;
    pub const MODEM_CONFIG_2: u8 = 0x1E;
    pub const PAYLOAD_LENGTH: u8 = 0x22;
    pub const MODEM_CONFIG_3: u8 = 0x26;
    pub const SYNC_WORD: u8 = 0x39;
    pub const DIO_MAPPING_1: u8 = 0x40;
    pub const VERSION: u8 = 0x42;
}

/// Controller LoRa configuration
/// Lower TX power than robot (PCB antenna, shorter range expected)
#[derive(Debug, Clone)]
pub struct ControllerLoRaConfig {
    pub frequency_hz: u32,
    pub tx_power_dbm: i8,
    pub spreading_factor: u8,
    pub bandwidth: u8,
    pub coding_rate: u8,
}

impl Default for ControllerLoRaConfig {
    fn default() -> Self {
        Self {
            frequency_hz: 866_000_000,
            tx_power_dbm: 14, // Lower than robot (PCB antenna)
            spreading_factor: 9,
            bandwidth: 8, // 250kHz
            coding_rate: 2, // 4/6
        }
    }
}

/// Controller-side LoRa radio abstraction
pub struct ControllerRadio {
    config: ControllerLoRaConfig,
    seq_counter: u16,
    last_rssi: i16,
    last_snr: i8,
    packets_sent: u32,
    packets_received: u32,
}

impl ControllerRadio {
    pub fn new() -> Self {
        Self {
            config: ControllerLoRaConfig::default(),
            seq_counter: 0,
            last_rssi: -120,
            last_snr: 0,
            packets_sent: 0,
            packets_received: 0,
        }
    }

    pub fn next_seq(&mut self) -> u16 {
        let seq = self.seq_counter;
        self.seq_counter = self.seq_counter.wrapping_add(1);
        seq
    }

    pub fn last_rssi(&self) -> i16 {
        self.last_rssi
    }

    pub fn last_snr(&self) -> i8 {
        self.last_snr
    }

    pub fn update_rx_stats(&mut self, rssi: i16, snr: i8) {
        self.last_rssi = rssi;
        self.last_snr = snr;
        self.packets_received += 1;
    }

    pub fn record_tx(&mut self) {
        self.packets_sent += 1;
    }
}

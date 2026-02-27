//! 1-Wire Protocol Driver (Software Bit-Bang)
//!
//! Used for DS18B20 soil temperature sensors.
//! Connected via PB1 with 4.7kΩ external pull-up to 3.3V.

use defmt::*;

/// DS18B20 ROM commands
const CMD_SKIP_ROM: u8 = 0xCC;
const CMD_CONVERT_T: u8 = 0x44;
const CMD_READ_SCRATCHPAD: u8 = 0xBE;
const CMD_MATCH_ROM: u8 = 0x55;
const CMD_SEARCH_ROM: u8 = 0xF0;
const CMD_READ_ROM: u8 = 0x33;

/// DS18B20 configuration
const DS18B20_FAMILY_CODE: u8 = 0x28;
const CONVERSION_TIME_MS: u32 = 750; // 12-bit resolution

/// Temperature reading from DS18B20
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Temperature {
    /// Temperature in °C × 100 (e.g., 2550 = 25.50°C)
    pub value_centidegrees: i16,
    /// Raw 16-bit value from sensor
    pub raw: u16,
    /// Reading is valid
    pub valid: bool,
}

impl Temperature {
    /// Get temperature as floating point
    pub fn as_f32(&self) -> f32 {
        self.value_centidegrees as f32 / 100.0
    }

    /// Parse temperature from DS18B20 scratchpad bytes
    pub fn from_scratchpad(lsb: u8, msb: u8) -> Self {
        let raw = (msb as u16) << 8 | lsb as u16;
        let signed = raw as i16;

        // DS18B20: 12-bit resolution, 0.0625°C per LSB
        // Multiply by 6.25 to get centidegrees
        let centidegrees = ((signed as i32) * 625) / 100;

        Self {
            value_centidegrees: centidegrees as i16,
            raw,
            valid: true,
        }
    }
}

/// 1-Wire bus timing constants (microseconds)
pub mod timing {
    /// Reset pulse duration
    pub const RESET_PULSE_US: u32 = 480;
    /// Presence detect wait
    pub const PRESENCE_WAIT_US: u32 = 70;
    /// Presence detect window
    pub const PRESENCE_WINDOW_US: u32 = 410;
    /// Write 1: release within this time
    pub const WRITE1_LOW_US: u32 = 6;
    /// Write 0: hold low for this time
    pub const WRITE0_LOW_US: u32 = 60;
    /// Slot total time
    pub const SLOT_US: u32 = 70;
    /// Read: sample after this time
    pub const READ_SAMPLE_US: u32 = 9;
    /// Recovery time between slots
    pub const RECOVERY_US: u32 = 2;
}

/// Software 1-Wire bus state machine
/// Actual GPIO operations must be performed by the caller
/// since this is no_std and we can't hold pin references generically.
pub struct OneWireProtocol;

impl OneWireProtocol {
    /// Calculate CRC-8 (Dallas/Maxim polynomial 0x31)
    pub fn crc8(data: &[u8]) -> u8 {
        let mut crc: u8 = 0;
        for &byte in data {
            let mut b = byte;
            for _ in 0..8 {
                let mix = (crc ^ b) & 0x01;
                crc >>= 1;
                if mix != 0 {
                    crc ^= 0x8C;
                }
                b >>= 1;
            }
        }
        crc
    }

    /// Verify DS18B20 scratchpad CRC (9 bytes, last byte is CRC)
    pub fn verify_scratchpad(data: &[u8; 9]) -> bool {
        Self::crc8(&data[..8]) == data[8]
    }

    /// Parse temperature from scratchpad
    pub fn parse_temperature(scratchpad: &[u8; 9]) -> Option<Temperature> {
        if !Self::verify_scratchpad(scratchpad) {
            return None;
        }
        Some(Temperature::from_scratchpad(scratchpad[0], scratchpad[1]))
    }
}

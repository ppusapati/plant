//! Modbus RTU Master Driver
//!
//! Communicates with RS485 soil sensors via Modbus RTU protocol.
//! Connected via UART2 through MAX3485 transceiver.
//! DE/RE pin (PD4) controls transmit/receive direction.

use defmt::*;
use heapless::Vec;

/// Modbus function codes
pub const FC_READ_HOLDING_REGISTERS: u8 = 0x03;
pub const FC_READ_INPUT_REGISTERS: u8 = 0x04;
pub const FC_WRITE_SINGLE_REGISTER: u8 = 0x06;
pub const FC_WRITE_MULTIPLE_REGISTERS: u8 = 0x10;

/// Modbus RTU frame builder and parser
pub struct ModbusRtu {
    /// Response timeout in milliseconds
    pub timeout_ms: u32,
}

/// Modbus request frame
#[derive(Debug, Clone)]
pub struct ModbusRequest {
    pub slave_addr: u8,
    pub function_code: u8,
    pub start_register: u16,
    pub register_count: u16,
}

/// Modbus response data
#[derive(Debug, Clone, defmt::Format)]
pub struct ModbusResponse {
    pub slave_addr: u8,
    pub function_code: u8,
    pub data: Vec<u16, 32>,
    pub valid: bool,
}

/// Soil sensor register addresses (standard RS485 soil sensors)
pub mod soil_registers {
    /// Soil moisture register
    pub const MOISTURE: u16 = 0x0000;
    /// Soil temperature register
    pub const TEMPERATURE: u16 = 0x0001;
    /// Soil EC (electrical conductivity) register
    pub const EC: u16 = 0x0002;
    /// Soil pH register
    pub const PH: u16 = 0x0003;
    /// Soil Nitrogen register
    pub const NITROGEN: u16 = 0x0004;
    /// Soil Phosphorus register
    pub const PHOSPHORUS: u16 = 0x0005;
    /// Soil Potassium register
    pub const POTASSIUM: u16 = 0x0006;
}

/// Modbus slave addresses
pub mod slave_addr {
    pub const NPK_SENSOR: u8 = 0x01;
    pub const PH_SENSOR: u8 = 0x02;
    pub const EC_SENSOR: u8 = 0x03;
    pub const MOISTURE_SENSOR: u8 = 0x04;
}

impl ModbusRtu {
    pub fn new(timeout_ms: u32) -> Self {
        Self { timeout_ms }
    }

    /// Build a Modbus RTU request frame for reading holding registers
    pub fn build_read_request(
        &self,
        slave_addr: u8,
        start_register: u16,
        count: u16,
    ) -> Vec<u8, 16> {
        let mut frame: Vec<u8, 16> = Vec::new();
        let _ = frame.push(slave_addr);
        let _ = frame.push(FC_READ_HOLDING_REGISTERS);
        let _ = frame.push((start_register >> 8) as u8);
        let _ = frame.push(start_register as u8);
        let _ = frame.push((count >> 8) as u8);
        let _ = frame.push(count as u8);

        let crc = compute_modbus_crc(&frame);
        let _ = frame.push(crc as u8); // CRC low byte first
        let _ = frame.push((crc >> 8) as u8); // CRC high byte

        frame
    }

    /// Parse a Modbus RTU response frame
    pub fn parse_response(&self, data: &[u8]) -> Option<ModbusResponse> {
        if data.len() < 5 {
            return None;
        }

        // Verify CRC
        let msg_len = data.len();
        let received_crc = (data[msg_len - 1] as u16) << 8 | data[msg_len - 2] as u16;
        let computed_crc = compute_modbus_crc(&data[..msg_len - 2]);

        if received_crc != computed_crc {
            warn!("Modbus CRC error: expected {:04X}, got {:04X}", computed_crc, received_crc);
            return None;
        }

        let slave_addr = data[0];
        let function_code = data[1];

        // Check for exception response
        if function_code & 0x80 != 0 {
            warn!("Modbus exception: slave={}, code={:02X}", slave_addr, data[2]);
            return None;
        }

        let byte_count = data[2] as usize;
        if data.len() < 3 + byte_count + 2 {
            return None;
        }

        // Parse register values (big-endian u16)
        let mut registers: Vec<u16, 32> = Vec::new();
        for i in (0..byte_count).step_by(2) {
            let value = (data[3 + i] as u16) << 8 | data[3 + i + 1] as u16;
            let _ = registers.push(value);
        }

        Some(ModbusResponse {
            slave_addr,
            function_code,
            data: registers,
            valid: true,
        })
    }

    /// Read all soil sensor data from NPK sensor (registers 0x0000-0x0006)
    pub fn build_npk_read_request(&self) -> Vec<u8, 16> {
        self.build_read_request(slave_addr::NPK_SENSOR, soil_registers::MOISTURE, 7)
    }

    /// Read pH from dedicated pH sensor
    pub fn build_ph_read_request(&self) -> Vec<u8, 16> {
        self.build_read_request(slave_addr::PH_SENSOR, soil_registers::PH, 1)
    }

    /// Read EC from dedicated EC sensor
    pub fn build_ec_read_request(&self) -> Vec<u8, 16> {
        self.build_read_request(slave_addr::EC_SENSOR, soil_registers::EC, 1)
    }
}

/// Parsed soil sensor data
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct SoilData {
    /// Soil moisture in % (scaled by 0.1)
    pub moisture_pct: f32,
    /// Soil temperature in °C (scaled by 0.1)
    pub temperature_c: f32,
    /// Electrical conductivity in µS/cm
    pub ec_us_cm: u16,
    /// Soil pH (scaled by 0.01)
    pub ph: f32,
    /// Nitrogen in mg/kg
    pub nitrogen_mg_kg: u16,
    /// Phosphorus in mg/kg
    pub phosphorus_mg_kg: u16,
    /// Potassium in mg/kg
    pub potassium_mg_kg: u16,
    /// Data is valid (successful read)
    pub valid: bool,
}

impl SoilData {
    /// Parse soil data from NPK sensor Modbus response (7 registers)
    pub fn from_npk_response(response: &ModbusResponse) -> Self {
        if response.data.len() < 7 {
            return Self::default();
        }

        Self {
            moisture_pct: response.data[0] as f32 * 0.1,
            temperature_c: response.data[1] as f32 * 0.1,
            ec_us_cm: response.data[2],
            ph: response.data[3] as f32 * 0.01,
            nitrogen_mg_kg: response.data[4],
            phosphorus_mg_kg: response.data[5],
            potassium_mg_kg: response.data[6],
            valid: true,
        }
    }
}

/// Compute Modbus CRC-16 (polynomial 0xA001)
fn compute_modbus_crc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }

    crc
}

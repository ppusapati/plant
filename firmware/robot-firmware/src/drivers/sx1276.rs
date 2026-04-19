//! SX1276 LoRa Radio Driver
//!
//! Semtech SX1276 driver for 868MHz ISM band communication.
//! Supports configurable spreading factor, bandwidth, and coding rate.
//! Connected via SPI1 with GPIO interrupts for DIO0/DIO1.

use defmt::*;
use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_time::{Duration, Timer};
use embedded_hal::spi::Operation;

/// SX1276 Register addresses
mod reg {
    pub const FIFO: u8 = 0x00;
    pub const OP_MODE: u8 = 0x01;
    pub const FR_MSB: u8 = 0x06;
    pub const FR_MID: u8 = 0x07;
    pub const FR_LSB: u8 = 0x08;
    pub const PA_CONFIG: u8 = 0x09;
    pub const PA_RAMP: u8 = 0x0A;
    pub const OCP: u8 = 0x0B;
    pub const LNA: u8 = 0x0C;
    pub const FIFO_ADDR_PTR: u8 = 0x0D;
    pub const FIFO_TX_BASE: u8 = 0x0E;
    pub const FIFO_RX_BASE: u8 = 0x0F;
    pub const FIFO_RX_CURRENT: u8 = 0x10;
    pub const IRQ_FLAGS_MASK: u8 = 0x11;
    pub const IRQ_FLAGS: u8 = 0x12;
    pub const RX_NB_BYTES: u8 = 0x13;
    pub const PKT_SNR: u8 = 0x19;
    pub const PKT_RSSI: u8 = 0x1A;
    pub const RSSI: u8 = 0x1B;
    pub const MODEM_CONFIG_1: u8 = 0x1D;
    pub const MODEM_CONFIG_2: u8 = 0x1E;
    pub const SYMB_TIMEOUT_LSB: u8 = 0x1F;
    pub const PREAMBLE_MSB: u8 = 0x20;
    pub const PREAMBLE_LSB: u8 = 0x21;
    pub const PAYLOAD_LENGTH: u8 = 0x22;
    pub const MAX_PAYLOAD_LENGTH: u8 = 0x23;
    pub const MODEM_CONFIG_3: u8 = 0x26;
    pub const FREQ_ERROR_MSB: u8 = 0x28;
    pub const DETECT_OPTIMIZE: u8 = 0x31;
    pub const INVERT_IQ: u8 = 0x33;
    pub const DETECTION_THRESHOLD: u8 = 0x37;
    pub const SYNC_WORD: u8 = 0x39;
    pub const DIO_MAPPING_1: u8 = 0x40;
    pub const VERSION: u8 = 0x42;
    pub const PA_DAC: u8 = 0x4D;
}

/// LoRa spreading factor
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum SpreadingFactor {
    SF7 = 7,
    SF8 = 8,
    SF9 = 9,
    SF10 = 10,
    SF11 = 11,
    SF12 = 12,
}

/// LoRa bandwidth
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum Bandwidth {
    Bw125kHz = 0x07,
    Bw250kHz = 0x08,
    Bw500kHz = 0x09,
}

/// LoRa coding rate
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum CodingRate {
    Cr4_5 = 1,
    Cr4_6 = 2,
    Cr4_7 = 3,
    Cr4_8 = 4,
}

/// LoRa radio configuration
#[derive(Debug, Clone, defmt::Format)]
pub struct LoRaConfig {
    pub frequency_hz: u32,
    pub spreading_factor: SpreadingFactor,
    pub bandwidth: Bandwidth,
    pub coding_rate: CodingRate,
    pub tx_power_dbm: i8,
    pub sync_word: u8,
    pub preamble_length: u16,
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self {
            frequency_hz: 866_000_000, // 866 MHz India ISM
            spreading_factor: SpreadingFactor::SF9,
            bandwidth: Bandwidth::Bw250kHz,
            coding_rate: CodingRate::Cr4_6,
            tx_power_dbm: 17,
            sync_word: 0xAD,
            preamble_length: 8,
        }
    }
}

/// Communication profiles for adaptive rate
impl LoRaConfig {
    pub fn fast() -> Self {
        Self {
            spreading_factor: SpreadingFactor::SF7,
            bandwidth: Bandwidth::Bw500kHz,
            coding_rate: CodingRate::Cr4_5,
            ..Default::default()
        }
    }

    pub fn normal() -> Self {
        Self::default()
    }

    pub fn long_range() -> Self {
        Self {
            spreading_factor: SpreadingFactor::SF12,
            bandwidth: Bandwidth::Bw125kHz,
            coding_rate: CodingRate::Cr4_8,
            tx_power_dbm: 20,
            ..Default::default()
        }
    }
}

/// SX1276 LoRa radio driver
pub struct Sx1276<SPI> {
    spi: SPI,
    cs: Output<'static>,
    reset: Output<'static>,
    config: LoRaConfig,
    seq_counter: u16,
}

/// The SX1276 driver uses `SpiBus` (not `SpiDevice`) because it controls
/// the CS pin directly as a separate GPIO output.  This is necessary to
/// perform back-to-back single-byte FIFO reads without de-asserting CS
/// between each byte.  A `SpiDevice` wrapper would toggle CS on every
/// transaction, which does not match the SX1276's FIFO access protocol.
impl<SPI> Sx1276<SPI>
where
    SPI: embedded_hal::spi::SpiBus,
{
    pub fn new(spi: SPI, cs: Output<'static>, reset: Output<'static>) -> Self {
        Self {
            spi,
            cs,
            reset,
            config: LoRaConfig::default(),
            seq_counter: 0,
        }
    }

    /// Initialize the SX1276 with the given configuration.
    ///
    /// Performs a hardware reset, verifies the chip version register
    /// (expects 0x12), then programs all modem parameters.
    pub async fn init(&mut self, config: LoRaConfig) {
        // Hardware reset: pull RESET low for 1 ms, then release and wait 5 ms
        self.reset.set_low();
        Timer::after(Duration::from_millis(1)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(5)).await;

        // Verify chip version
        let version = self.read_register(reg::VERSION);
        if version != 0x12 {
            error!("SX1276: unexpected version 0x{:02X} (expected 0x12)", version);
        } else {
            info!("SX1276: chip version OK (0x12)");
        }

        // Enter sleep mode to allow changing LoRa/FSK mode bit
        self.write_register(reg::OP_MODE, 0x00); // Sleep, FSK/OOK
        Timer::after(Duration::from_millis(1)).await;
        // Switch to LoRa mode, remain in sleep
        self.write_register(reg::OP_MODE, 0x80); // Sleep, LoRa
        Timer::after(Duration::from_millis(1)).await;

        // Set frequency
        self.config = config;
        let frf = ((self.config.frequency_hz as u64) << 19) / 32_000_000;
        self.write_register(reg::FR_MSB, (frf >> 16) as u8);
        self.write_register(reg::FR_MID, (frf >> 8) as u8);
        self.write_register(reg::FR_LSB, frf as u8);

        // PA config: PA_BOOST, max power, output power
        let pa_config = 0x80 | ((self.config.tx_power_dbm as u8).saturating_sub(2) & 0x0F);
        self.write_register(reg::PA_CONFIG, pa_config);

        // Over-current protection: enabled, 140 mA
        self.write_register(reg::OCP, 0x3B);

        // LNA: max gain, boost on
        self.write_register(reg::LNA, 0x23);

        // Modem config 1: bandwidth, coding rate, implicit header off
        let bw_cr = ((self.config.bandwidth as u8) << 4)
            | ((self.config.coding_rate as u8) << 1);
        self.write_register(reg::MODEM_CONFIG_1, bw_cr);

        // Modem config 2: spreading factor, TX continuous off, CRC on
        let mc2 = ((self.config.spreading_factor as u8) << 4) | 0x04;
        self.write_register(reg::MODEM_CONFIG_2, mc2);

        // Modem config 3: LDRO auto, AGC enabled
        self.write_register(reg::MODEM_CONFIG_3, 0x04);

        // Preamble length
        self.write_register(reg::PREAMBLE_MSB, (self.config.preamble_length >> 8) as u8);
        self.write_register(reg::PREAMBLE_LSB, self.config.preamble_length as u8);

        // Sync word (0xAD for ADR-1 network)
        self.write_register(reg::SYNC_WORD, self.config.sync_word);

        // FIFO base addresses
        self.write_register(reg::FIFO_TX_BASE, 0x00);
        self.write_register(reg::FIFO_RX_BASE, 0x00);

        // PA DAC for +20 dBm mode
        if self.config.tx_power_dbm >= 20 {
            self.write_register(reg::PA_DAC, 0x87);
        }

        // Enter standby
        self.set_mode(0x01);
        info!(
            "SX1276: initialized @ {} Hz, SF{}, BW={} kHz",
            self.config.frequency_hz,
            self.config.spreading_factor as u8,
            match self.config.bandwidth {
                Bandwidth::Bw125kHz => 125,
                Bandwidth::Bw250kHz => 250,
                Bandwidth::Bw500kHz => 500,
            }
        );
    }

    /// Read a single register
    fn read_register(&mut self, addr: u8) -> u8 {
        let mut buf = [addr & 0x7F, 0x00];
        self.cs.set_low();
        let _ = self.spi.transfer_in_place(&mut buf);
        self.cs.set_high();
        buf[1]
    }

    /// Write a single register
    fn write_register(&mut self, addr: u8, value: u8) {
        let buf = [addr | 0x80, value];
        self.cs.set_low();
        let _ = self.spi.write(&buf);
        self.cs.set_high();
    }

    /// Set operating mode
    fn set_mode(&mut self, mode: u8) {
        self.write_register(reg::OP_MODE, 0x80 | mode); // 0x80 = LoRa mode bit
    }

    /// Get next sequence number
    pub fn next_seq(&mut self) -> u16 {
        let seq = self.seq_counter;
        self.seq_counter = self.seq_counter.wrapping_add(1);
        seq
    }

    /// Get RSSI of last received packet
    pub fn last_packet_rssi(&mut self) -> i16 {
        let raw = self.read_register(reg::PKT_RSSI) as i16;
        // For HF port (>868MHz): RSSI = -157 + raw
        -157 + raw
    }

    /// Get SNR of last received packet
    pub fn last_packet_snr(&mut self) -> i8 {
        let raw = self.read_register(reg::PKT_SNR) as i8;
        raw / 4
    }

    /// Write data to FIFO and transmit
    pub fn transmit(&mut self, data: &[u8]) -> bool {
        if data.len() > 255 {
            return false;
        }

        // Set standby mode
        self.set_mode(0x01);

        // Reset FIFO pointer to TX base
        self.write_register(reg::FIFO_ADDR_PTR, 0x00);

        // Write payload to FIFO
        for &byte in data {
            self.write_register(reg::FIFO, byte);
        }

        // Set payload length
        self.write_register(reg::PAYLOAD_LENGTH, data.len() as u8);

        // Set DIO0 to TxDone
        self.write_register(reg::DIO_MAPPING_1, 0x40);

        // Enter TX mode
        self.set_mode(0x03);

        // Wait for TxDone (poll IRQ flags)
        for _ in 0..1000 {
            let irq = self.read_register(reg::IRQ_FLAGS);
            if irq & 0x08 != 0 {
                // TxDone
                self.write_register(reg::IRQ_FLAGS, 0x08); // Clear flag
                return true;
            }
        }

        false
    }

    /// Set radio to continuous receive mode
    pub fn start_receive(&mut self) {
        // Set FIFO RX base address
        self.write_register(reg::FIFO_RX_BASE, 0x00);
        self.write_register(reg::FIFO_ADDR_PTR, 0x00);

        // Set DIO0 to RxDone
        self.write_register(reg::DIO_MAPPING_1, 0x00);

        // Clear IRQ flags
        self.write_register(reg::IRQ_FLAGS, 0xFF);

        // Enter continuous RX mode
        self.set_mode(0x05);
    }

    /// Check if packet received and read it
    pub fn receive(&mut self, buf: &mut [u8]) -> Option<usize> {
        let irq = self.read_register(reg::IRQ_FLAGS);

        if irq & 0x40 != 0 {
            // RxDone
            // Check for CRC error
            if irq & 0x20 != 0 {
                self.write_register(reg::IRQ_FLAGS, 0xFF);
                return None;
            }

            let len = self.read_register(reg::RX_NB_BYTES) as usize;
            let current_addr = self.read_register(reg::FIFO_RX_CURRENT);
            self.write_register(reg::FIFO_ADDR_PTR, current_addr);

            let read_len = len.min(buf.len());
            for i in 0..read_len {
                buf[i] = self.read_register(reg::FIFO);
            }

            // Clear IRQ flags
            self.write_register(reg::IRQ_FLAGS, 0xFF);

            // Restart receive
            self.start_receive();

            Some(read_len)
        } else {
            None
        }
    }
}

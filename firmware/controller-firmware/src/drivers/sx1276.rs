//! SX1276 LoRa driver for ESP32-S3 controller
//! Same radio as robot, connected via SPI2 on ESP32-S3.
//! Uses PCB trace antenna (meander IFA) instead of external SMA.

use defmt::*;
use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;

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
            bandwidth: 8, // 250 kHz
            coding_rate: 2, // 4/6
        }
    }
}

/// Controller-side SX1276 radio driver.
///
/// Owns the SPI bus and CS/RESET GPIO pins so register access can be
/// encapsulated cleanly without closures.  The `SPI` type parameter must
/// implement `embedded_hal::spi::SpiBus` (blocking variant, which is what
/// esp-hal's `Spi<Blocking>` provides).
pub struct ControllerRadio<SPI, CS, RST> {
    spi: SPI,
    cs: CS,
    reset: RST,
    config: ControllerLoRaConfig,
    seq_counter: u16,
    last_rssi: i16,
    last_snr: i8,
    packets_sent: u32,
    packets_received: u32,
}

impl<SPI, CS, RST> ControllerRadio<SPI, CS, RST>
where
    SPI: SpiBus,
    CS:  OutputPin,
    RST: OutputPin,
{
    /// Create a new `ControllerRadio` from a SPI bus and two GPIO pins.
    pub fn new(spi: SPI, cs: CS, reset: RST) -> Self {
        Self {
            spi,
            cs,
            reset,
            config: ControllerLoRaConfig::default(),
            seq_counter: 0,
            last_rssi: -120,
            last_snr: 0,
            packets_sent: 0,
            packets_received: 0,
        }
    }

    // ── Private SPI helpers ───────────────────────────────────────────

    pub fn read_register(&mut self, addr: u8) -> u8 {
        let mut buf = [addr & 0x7F, 0x00];
        let _ = self.cs.set_low();
        let _ = self.spi.transfer_in_place(&mut buf);
        let _ = self.cs.set_high();
        buf[1]
    }

    pub fn write_register(&mut self, addr: u8, val: u8) {
        let buf = [addr | 0x80, val];
        let _ = self.cs.set_low();
        let _ = self.spi.write(&buf);
        let _ = self.cs.set_high();
    }

    // ── Initialization ────────────────────────────────────────────────

    /// Reset the SX1276 and program all modem registers.
    /// Must be called with a blocking delay available (uses spin-wait because
    /// the controller firmware uses the esp-hal blocking SPI).
    pub fn init(&mut self, config: ControllerLoRaConfig) {
        // Hardware reset: RESET low ≥ 100 µs, then release and wait ≥ 5 ms
        let _ = self.reset.set_low();
        spin_delay_ms(1);
        let _ = self.reset.set_high();
        spin_delay_ms(5);

        let version = self.read_register(reg::VERSION);
        if version != 0x12 {
            error!("SX1276: unexpected version 0x{:02X} (expected 0x12)", version);
        } else {
            info!("SX1276: chip version OK (0x12)");
        }

        // Sleep → LoRa mode
        self.write_register(reg::OP_MODE, 0x00);
        spin_delay_ms(1);
        self.write_register(reg::OP_MODE, 0x80);
        spin_delay_ms(1);

        self.config = config;

        // Frequency
        let frf = ((self.config.frequency_hz as u64) << 19) / 32_000_000;
        self.write_register(reg::FR_MSB, (frf >> 16) as u8);
        self.write_register(reg::FR_MID, (frf >> 8) as u8);
        self.write_register(reg::FR_LSB, frf as u8);

        // PA: PA_BOOST
        self.write_register(reg::PA_CONFIG, 0x80 | ((self.config.tx_power_dbm as u8).saturating_sub(2) & 0x0F));

        // Modem config
        self.write_register(reg::MODEM_CONFIG_1, (self.config.bandwidth << 4) | (self.config.coding_rate << 1));
        self.write_register(reg::MODEM_CONFIG_2, (self.config.spreading_factor << 4) | 0x04);
        self.write_register(reg::MODEM_CONFIG_3, 0x04); // AGC enabled

        // FIFO base addresses and sync word
        self.write_register(reg::FIFO_TX_BASE, 0x00);
        self.write_register(reg::FIFO_RX_BASE, 0x00);
        self.write_register(reg::SYNC_WORD, 0xAD); // ADR-1 network ID

        // Enter standby, then continuous RX
        self.enter_rx();

        info!("SX1276: initialized @ {} Hz", self.config.frequency_hz);
    }

    /// Put the radio into continuous receive mode.
    pub fn enter_rx(&mut self) {
        self.write_register(reg::OP_MODE, 0x81); // standby (LoRa)
        self.write_register(reg::FIFO_ADDR_PTR, 0x00);
        self.write_register(reg::IRQ_FLAGS, 0xFF); // clear flags
        self.write_register(reg::OP_MODE, 0x85); // continuous RX (LoRa)
    }

    /// Poll for a received packet.  Returns the number of bytes received, or 0.
    pub fn poll_receive(&mut self, buf: &mut [u8]) -> usize {
        let irq = self.read_register(reg::IRQ_FLAGS);
        if irq & 0x40 == 0 {
            return 0; // RxDone not set
        }

        self.write_register(reg::IRQ_FLAGS, 0xFF); // clear all flags

        if irq & 0x20 != 0 {
            // CRC error — restart RX and bail
            self.enter_rx();
            return 0;
        }

        let len = self.read_register(reg::RX_NB_BYTES) as usize;
        let cur_addr = self.read_register(reg::FIFO_RX_CURRENT);
        self.write_register(reg::FIFO_ADDR_PTR, cur_addr);

        let read_len = len.min(buf.len());
        for b in buf[..read_len].iter_mut() {
            *b = self.read_register(reg::FIFO);
        }

        self.packets_received += 1;
        self.last_rssi = -157 + self.read_register(reg::PKT_RSSI) as i16;
        self.last_snr  = self.read_register(reg::PKT_SNR) as i8 / 4;

        self.enter_rx(); // re-arm for next packet
        read_len
    }

    /// Transmit a packet.  Returns `true` on success (TxDone IRQ within timeout).
    pub fn transmit(&mut self, data: &[u8]) -> bool {
        if data.len() > 255 {
            return false;
        }

        self.write_register(reg::OP_MODE, 0x81); // standby (LoRa)
        self.write_register(reg::FIFO_ADDR_PTR, 0x00);

        for &byte in data {
            self.write_register(reg::FIFO, byte);
        }

        self.write_register(reg::PAYLOAD_LENGTH, data.len() as u8);
        self.write_register(reg::DIO_MAPPING_1, 0x40); // DIO0 = TxDone
        self.write_register(reg::OP_MODE, 0x83); // TX (LoRa)

        for _ in 0..2000 {
            let irq = self.read_register(reg::IRQ_FLAGS);
            if irq & 0x08 != 0 {
                self.write_register(reg::IRQ_FLAGS, 0x08); // clear TxDone
                self.packets_sent += 1;
                self.enter_rx();
                return true;
            }
        }

        // Timeout: return to RX anyway
        self.enter_rx();
        false
    }

    // ── Accessors ─────────────────────────────────────────────────────

    pub fn next_seq(&mut self) -> u16 {
        let seq = self.seq_counter;
        self.seq_counter = self.seq_counter.wrapping_add(1);
        seq
    }

    pub fn last_rssi(&self) -> i16  { self.last_rssi }
    pub fn last_snr(&self)  -> i8   { self.last_snr }
    pub fn packets_sent(&self)     -> u32 { self.packets_sent }
    pub fn packets_received(&self) -> u32 { self.packets_received }
}

/// Busy-wait delay in milliseconds.
/// Used only during init (before Embassy timer is available).
fn spin_delay_ms(ms: u32) {
    // Conservative 240 MHz ESP32-S3: ~240_000 cycles/ms
    const CYCLES_PER_MS: u32 = 240_000;
    let total = ms.saturating_mul(CYCLES_PER_MS);
    for _ in 0..total {
        core::hint::spin_loop();
    }
}


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

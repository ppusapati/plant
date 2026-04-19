//! 1-Wire Protocol Driver (Software Bit-Bang)
//!
//! Used for DS18B20 soil temperature sensors.
//! Connected via PB1 with 4.7kΩ external pull-up to 3.3V.

use defmt::*;
use embassy_stm32::gpio::{Flex, Pull, Speed};
use embassy_time::{Duration, Timer};

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

/// Hardware 1-Wire bus driver using Embassy STM32 GPIO (bit-bang).
///
/// Requires a `Flex<'static>` pin connected to the 1-Wire data line with a
/// 4.7 kΩ pull-up resistor to 3.3 V (external — the internal pull-up is too
/// weak for reliable 1-Wire signalling).
///
/// Bit-level timing uses `cortex_m::asm::delay` (busy-wait) because the
/// required intervals (2–70 µs) are shorter than the Embassy async-timer
/// resolution (~30 µs at 32.768 kHz).  The 750 ms conversion wait uses
/// `Timer::after` so the executor can schedule other tasks.
pub struct OneWireBus {
    pin: Flex<'static>,
}

impl OneWireBus {
    /// Create a new 1-Wire bus driver.
    ///
    /// `pin` must be the Flex GPIO connected to the bus data line.  The driver
    /// configures it as a floating input initially; the external pull-up keeps
    /// the bus high at idle.
    pub fn new(mut pin: Flex<'static>) -> Self {
        pin.set_as_input(Pull::None);
        Self { pin }
    }

    // ── GPIO helpers ─────────────────────────────────────────────────────

    /// Drive the bus actively low (open-drain style: switch to output-low).
    #[inline]
    fn drive_low(&mut self) {
        self.pin.set_low();
        self.pin.set_as_output(Speed::Low);
    }

    /// Release the bus (switch to input; external pull-up returns the line HIGH).
    #[inline]
    fn release(&mut self) {
        self.pin.set_as_input(Pull::None);
    }

    /// Sample the current bus level.
    #[inline]
    fn read(&mut self) -> bool {
        self.pin.is_high()
    }

    // ── Timing ───────────────────────────────────────────────────────────

    /// Busy-wait for at least `us` microseconds.
    ///
    /// Uses `cortex_m::asm::delay` which executes a tight loop at roughly one
    /// iteration per CPU cycle.  At STM32H743 SYSCLK = 480 MHz this gives
    /// ~480 cycles/µs.  Being a few µs longer than requested is safe for
    /// 1-Wire because all timing constraints are "at least N µs".
    ///
    /// **Note:** This multiplier is calibrated for the 480 MHz SYSCLK
    /// configured in `main.rs`.  If the clock is changed, update the
    /// constant `CYCLES_PER_US` below accordingly.
    #[inline]
    fn delay_us(&self, us: u32) {
        const CYCLES_PER_US: u32 = 480; // STM32H743 SYSCLK = 480 MHz
        cortex_m::asm::delay(us.saturating_mul(CYCLES_PER_US));
    }

    // ── 1-Wire primitives ────────────────────────────────────────────────

    /// Issue a bus reset pulse and check for a device presence response.
    ///
    /// Returns `true` when at least one device pulls the bus low during the
    /// presence window (60–240 µs after the master releases the reset).
    pub fn reset(&mut self) -> bool {
        self.drive_low();
        self.delay_us(timing::RESET_PULSE_US);
        self.release();
        self.delay_us(timing::PRESENCE_WAIT_US);
        let present = !self.read(); // device pulls bus LOW → present
        self.delay_us(timing::PRESENCE_WINDOW_US);
        present
    }

    /// Write a single bit to the bus (LSB-first convention used by write_byte).
    pub fn write_bit(&mut self, bit: bool) {
        if bit {
            // Write-1 slot: drive low for 6 µs then release for remainder
            self.drive_low();
            self.delay_us(timing::WRITE1_LOW_US);
            self.release();
            self.delay_us(timing::SLOT_US - timing::WRITE1_LOW_US);
        } else {
            // Write-0 slot: hold low for 60 µs then release
            self.drive_low();
            self.delay_us(timing::WRITE0_LOW_US);
            self.release();
            self.delay_us(timing::SLOT_US - timing::WRITE0_LOW_US);
        }
        self.delay_us(timing::RECOVERY_US);
    }

    /// Read a single bit from the bus.
    ///
    /// The master initiates a read slot (drives low for 6 µs, then releases)
    /// and samples the bus 9 µs later (15 µs total from slot start), which is
    /// within the DS18B20's valid sampling window.
    pub fn read_bit(&mut self) -> bool {
        self.drive_low();
        self.delay_us(timing::WRITE1_LOW_US); // 6 µs — initiate read slot
        self.release();
        self.delay_us(timing::READ_SAMPLE_US); // 9 µs — device has driven by now
        let bit = self.read();
        // Complete the 70 µs slot (15 µs already elapsed)
        self.delay_us(timing::SLOT_US - timing::WRITE1_LOW_US - timing::READ_SAMPLE_US);
        self.delay_us(timing::RECOVERY_US);
        bit
    }

    /// Write a byte to the bus, LSB first.
    pub fn write_byte(&mut self, byte: u8) {
        for i in 0..8 {
            self.write_bit((byte >> i) & 1 != 0);
        }
    }

    /// Read a byte from the bus, LSB first.
    pub fn read_byte(&mut self) -> u8 {
        let mut byte = 0u8;
        for i in 0..8 {
            if self.read_bit() {
                byte |= 1 << i;
            }
        }
        byte
    }

    // ── DS18B20 high-level interface ─────────────────────────────────────

    /// Read the temperature from a single DS18B20 on the bus.
    ///
    /// Sequence (per DS18B20 datasheet):
    /// 1. Reset + Skip ROM + Convert T command
    /// 2. Await 750 ms (12-bit conversion time) — yields to executor
    /// 3. Reset + Skip ROM + Read Scratchpad (9 bytes)
    /// 4. Verify CRC-8 and parse temperature
    ///
    /// Returns `None` if no device is present or the scratchpad CRC fails.
    pub async fn read_temperature(&mut self) -> Option<Temperature> {
        // Step 1: trigger conversion
        if !self.reset() {
            warn!("1-Wire: no device present (reset)");
            return None;
        }
        self.write_byte(CMD_SKIP_ROM);
        self.write_byte(CMD_CONVERT_T);

        // Step 2: wait for 12-bit conversion — async so the executor stays alive
        Timer::after(Duration::from_millis(CONVERSION_TIME_MS as u64)).await;

        // Step 3: read 9-byte scratchpad
        if !self.reset() {
            warn!("1-Wire: no presence pulse after conversion");
            return None;
        }
        self.write_byte(CMD_SKIP_ROM);
        self.write_byte(CMD_READ_SCRATCHPAD);

        let mut scratchpad = [0u8; 9];
        for byte in &mut scratchpad {
            *byte = self.read_byte();
        }

        // Step 4: verify CRC and parse
        match OneWireProtocol::parse_temperature(&scratchpad) {
            Some(t) => {
                info!(
                    "1-Wire: {}.{:02}°C (raw=0x{:04X})",
                    t.value_centidegrees / 100,
                    (t.value_centidegrees % 100).unsigned_abs(),
                    t.raw,
                );
                Some(t)
            }
            None => {
                warn!("1-Wire: CRC error on scratchpad");
                None
            }
        }
    }
}

/// Software 1-Wire bus state machine (protocol logic, no hardware coupling).
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

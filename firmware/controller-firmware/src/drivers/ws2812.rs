//! APA102-2020 (Industrial) LED Driver
//!
//! 4× APA102-2020 LEDs daisy-chained via SPI on GPIO45 (Data) + GPIO46 (Clock).
//! Industrial grade: -40°C to +85°C operating range.
//! Used for status indication on the controller.

use defmt::*;

/// Number of LEDs in the chain
pub const NUM_LEDS: usize = 4;

/// LED positions on controller
pub const LED_STATUS: usize = 0; // Main status
pub const LED_LINK: usize = 1; // Communication link
pub const LED_BATTERY: usize = 2; // Battery indicator
pub const LED_MODE: usize = 3; // Operating mode

/// RGB color (native RGB order for APA102)
#[derive(Debug, Clone, Copy, Default)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const OFF: Self = Self::new(0, 0, 0);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const ORANGE: Self = Self::new(255, 128, 0);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const PURPLE: Self = Self::new(128, 0, 255);

    /// Dim the color by a factor (0-255, where 255 = full brightness)
    pub fn dim(&self, brightness: u8) -> Self {
        Self {
            r: ((self.r as u16 * brightness as u16) / 255) as u8,
            g: ((self.g as u16 * brightness as u16) / 255) as u8,
            b: ((self.b as u16 * brightness as u16) / 255) as u8,
        }
    }

    /// Convert to APA102 frame format (0xE0|brightness, B, G, R)
    pub fn to_apa102_frame(&self, brightness: u8) -> [u8; 4] {
        let global = 0xE0 | (brightness >> 3); // 5-bit global brightness
        [global, self.b, self.g, self.r]
    }

    /// Convert to GRB byte order (legacy compatibility)
    pub fn to_grb(&self) -> [u8; 3] {
        [self.g, self.r, self.b]
    }
}

/// LED pattern state machine
pub struct LedController {
    pub leds: [RgbColor; NUM_LEDS],
    pub brightness: u8,
    animation_frame: u16,
}

impl LedController {
    pub fn new() -> Self {
        Self {
            leds: [RgbColor::OFF; NUM_LEDS],
            brightness: 64, // Default 25% brightness
            animation_frame: 0,
        }
    }

    /// Set all LEDs off
    pub fn all_off(&mut self) {
        for led in &mut self.leds {
            *led = RgbColor::OFF;
        }
    }

    /// Set a single LED color
    pub fn set(&mut self, index: usize, color: RgbColor) {
        if index < NUM_LEDS {
            self.leds[index] = color.dim(self.brightness);
        }
    }

    /// Update LED states based on robot connection status
    pub fn update_status(&mut self, connected: bool, mode: u8, battery_soc: u8, rssi: i16) {
        // Status LED: pulsing green when connected, red when not
        if connected {
            let pulse = self.pulse_brightness(30);
            self.leds[LED_STATUS] = RgbColor::GREEN.dim(pulse);
        } else {
            let pulse = self.pulse_brightness(60);
            self.leds[LED_STATUS] = RgbColor::RED.dim(pulse);
        }

        // Link quality LED
        self.leds[LED_LINK] = if rssi > -80 {
            RgbColor::GREEN.dim(self.brightness)
        } else if rssi > -110 {
            RgbColor::YELLOW.dim(self.brightness)
        } else if rssi > -130 {
            RgbColor::ORANGE.dim(self.brightness)
        } else {
            RgbColor::RED.dim(self.brightness)
        };

        // Battery LED
        self.leds[LED_BATTERY] = match battery_soc {
            0..=10 => {
                let pulse = self.pulse_brightness(120);
                RgbColor::RED.dim(pulse) // Flashing red
            }
            11..=25 => RgbColor::ORANGE.dim(self.brightness),
            26..=75 => RgbColor::YELLOW.dim(self.brightness),
            76..=100 => RgbColor::GREEN.dim(self.brightness),
            _ => RgbColor::WHITE.dim(self.brightness),
        };

        // Mode LED
        self.leds[LED_MODE] = match mode {
            0 => RgbColor::WHITE.dim(self.brightness / 2),  // Idle: dim white
            1 => RgbColor::BLUE.dim(self.brightness),       // Manual: blue
            2 => RgbColor::CYAN.dim(self.brightness),       // Auto: cyan
            3 => RgbColor::GREEN.dim(self.brightness),      // Deseeding: green
            4 => RgbColor::PURPLE.dim(self.brightness),     // RTH: purple
            5 => RgbColor::RED.dim(255),                    // ESTOP: bright red
            _ => RgbColor::OFF,
        };

        self.animation_frame = self.animation_frame.wrapping_add(1);
    }

    /// Generate a pulsing brightness value (sawtooth wave)
    fn pulse_brightness(&self, speed: u16) -> u8 {
        let phase = (self.animation_frame % (speed * 2)) as f32 / speed as f32;
        let brightness = if phase < 1.0 {
            phase
        } else {
            2.0 - phase
        };
        (brightness * self.brightness as f32) as u8
    }

    /// Generate APA102 SPI data frame
    /// Format: [Start Frame (4×0x00)] [LED Frames (4 bytes each)] [End Frame (4×0xFF)]
    /// Transmitted via SPI at up to 8MHz clock
    pub fn generate_spi_data(&self) -> [u8; 4 + NUM_LEDS * 4 + 4] {
        let mut data = [0u8; 4 + NUM_LEDS * 4 + 4];
        // Start frame: 4 bytes of 0x00
        // (already zeroed by default)

        // LED frames
        for (i, led) in self.leds.iter().enumerate() {
            let frame = led.to_apa102_frame(self.brightness);
            let offset = 4 + i * 4;
            data[offset] = frame[0];
            data[offset + 1] = frame[1];
            data[offset + 2] = frame[2];
            data[offset + 3] = frame[3];
        }

        // End frame: 4 bytes of 0xFF
        let end_offset = 4 + NUM_LEDS * 4;
        data[end_offset] = 0xFF;
        data[end_offset + 1] = 0xFF;
        data[end_offset + 2] = 0xFF;
        data[end_offset + 3] = 0xFF;

        data
    }

    /// Generate legacy WS2812B-compatible data (for testing)
    pub fn generate_data(&self) -> [u8; NUM_LEDS * 3] {
        let mut data = [0u8; NUM_LEDS * 3];
        for (i, led) in self.leds.iter().enumerate() {
            let grb = led.to_grb();
            data[i * 3] = grb[0];
            data[i * 3 + 1] = grb[1];
            data[i * 3 + 2] = grb[2];
        }
        data
    }
}

//! NavIC GNSS driver for controller
//! u-blox MAX-M10S module via UART
//! Used for controller position (field mapping, distance to robot)

use defmt::*;
use heapless::Vec;

/// Controller GNSS position data (simplified vs robot)
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct ControllerPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub fix_valid: bool,
    pub sat_count: u8,
    pub hdop: f32,
}

/// Simple NMEA GGA parser for controller (we only need position)
pub struct SimpleNmeaParser {
    buffer: Vec<u8, 128>,
    in_sentence: bool,
    pub position: ControllerPosition,
}

impl SimpleNmeaParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            in_sentence: false,
            position: ControllerPosition::default(),
        }
    }

    /// Feed a byte, return true if position updated
    pub fn feed(&mut self, byte: u8) -> bool {
        match byte {
            b'$' => {
                self.buffer.clear();
                self.in_sentence = true;
                let _ = self.buffer.push(byte);
                false
            }
            b'\n' if self.in_sentence => {
                self.in_sentence = false;
                self.try_parse_gga()
            }
            b'\r' => false,
            _ if self.in_sentence => {
                if self.buffer.push(byte).is_err() {
                    self.in_sentence = false;
                }
                false
            }
            _ => false,
        }
    }

    fn try_parse_gga(&mut self) -> bool {
        let s = match core::str::from_utf8(&self.buffer) {
            Ok(s) => s,
            Err(_) => return false,
        };

        if !s.starts_with("$GNGGA") && !s.starts_with("$GPGGA") {
            return false;
        }

        let fields: Vec<&str, 16> = s.split(',').collect();
        if fields.len() < 10 {
            return false;
        }

        // Latitude
        if let Some(lat) = parse_coord(fields[2]) {
            self.position.latitude = if fields[3] == "S" { -lat } else { lat };
        }

        // Longitude
        if let Some(lon) = parse_coord(fields[4]) {
            self.position.longitude = if fields[5] == "W" { -lon } else { lon };
        }

        // Fix quality
        let fix_q = fields[6].parse::<u8>().unwrap_or(0);
        self.position.fix_valid = fix_q > 0;

        // Satellites
        self.position.sat_count = fields[7].parse::<u8>().unwrap_or(0);

        // HDOP
        self.position.hdop = parse_f32_simple(fields[8]).unwrap_or(99.9);

        // Altitude
        self.position.altitude = parse_f32_simple(fields[9]).unwrap_or(0.0);

        self.position.fix_valid
    }
}

/// Calculate distance between two GPS positions (Haversine formula)
pub fn distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
    let r = 6_371_000.0f64; // Earth radius in meters

    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let a = libm::sin(dlat / 2.0) * libm::sin(dlat / 2.0)
        + libm::cos(lat1.to_radians())
            * libm::cos(lat2.to_radians())
            * libm::sin(dlon / 2.0)
            * libm::sin(dlon / 2.0);

    let c = 2.0 * libm::atan2(libm::sqrt(a), libm::sqrt(1.0 - a));

    (r * c) as f32
}

/// Calculate bearing from position 1 to position 2 (degrees)
pub fn bearing_degrees(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();

    let y = libm::sin(dlon) * libm::cos(lat2_r);
    let x = libm::cos(lat1_r) * libm::sin(lat2_r)
        - libm::sin(lat1_r) * libm::cos(lat2_r) * libm::cos(dlon);

    let bearing = libm::atan2(y, x).to_degrees();
    ((bearing + 360.0) % 360.0) as f32
}

fn parse_coord(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let val = parse_f64_simple(s)?;
    let degrees = (val / 100.0) as i32;
    let minutes = val - (degrees as f64) * 100.0;
    Some(degrees as f64 + minutes / 60.0)
}

fn parse_f64_simple(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let mut result: f64 = 0.0;
    let mut decimal = false;
    let mut decimal_places: f64 = 1.0;
    let mut negative = false;
    for (i, c) in s.chars().enumerate() {
        match c {
            '-' if i == 0 => negative = true,
            '.' => decimal = true,
            '0'..='9' => {
                let digit = (c as u8 - b'0') as f64;
                if decimal {
                    decimal_places *= 10.0;
                    result += digit / decimal_places;
                } else {
                    result = result * 10.0 + digit;
                }
            }
            _ => return None,
        }
    }
    if negative {
        result = -result;
    }
    Some(result)
}

fn parse_f32_simple(s: &str) -> Option<f32> {
    parse_f64_simple(s).map(|v| v as f32)
}

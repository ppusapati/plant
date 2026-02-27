//! NavIC/GPS GNSS Driver
//!
//! Parses NMEA 0183 sentences from u-blox NEO-M9N module.
//! Supports NavIC (IRNSS), GPS, GLONASS, and Galileo constellations.
//! Connected via UART1 at 115200 baud with PPS on TIM2_CH1.

use defmt::*;
use heapless::{String, Vec};

/// Maximum NMEA sentence length
const MAX_NMEA_LEN: usize = 128;

/// GNSS fix quality
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum FixQuality {
    NoFix = 0,
    GpsFix = 1,
    DgpsFix = 2,
    PpsFix = 3,
    RtkFixed = 4,
    RtkFloat = 5,
    Estimated = 6,
}

/// Satellite constellation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Constellation {
    Gps,
    Glonass,
    Galileo,
    NavIC,
    Unknown,
}

/// Satellite info from GSV sentence
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct SatelliteInfo {
    pub prn: u16,
    pub elevation: i16,
    pub azimuth: u16,
    pub snr: u8,
    pub constellation: Constellation,
}

/// Complete GNSS fix data
#[derive(Debug, Clone, defmt::Format)]
pub struct GnssData {
    /// Latitude in degrees (positive = North)
    pub latitude: f64,
    /// Longitude in degrees (positive = East)
    pub longitude: f64,
    /// Altitude above MSL in meters
    pub altitude: f32,
    /// Speed over ground in km/h
    pub speed_kmh: f32,
    /// Course/heading in degrees (0-360)
    pub heading: f32,
    /// Fix quality
    pub fix_quality: FixQuality,
    /// Number of satellites used in fix
    pub satellites_used: u8,
    /// Horizontal dilution of precision
    pub hdop: f32,
    /// UTC time (HHMMSS.SS as integer × 100)
    pub utc_time: u32,
    /// Date (DDMMYY)
    pub date: u32,
    /// Total NavIC satellites in view
    pub navic_sats_in_view: u8,
    /// Total GPS satellites in view
    pub gps_sats_in_view: u8,
    /// Fix is valid
    pub valid: bool,
}

impl Default for GnssData {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            speed_kmh: 0.0,
            heading: 0.0,
            fix_quality: FixQuality::NoFix,
            satellites_used: 0,
            hdop: 99.99,
            utc_time: 0,
            date: 0,
            navic_sats_in_view: 0,
            gps_sats_in_view: 0,
            valid: false,
        }
    }
}

/// NMEA parser state machine
pub struct NmeaParser {
    buffer: Vec<u8, MAX_NMEA_LEN>,
    pub data: GnssData,
    in_sentence: bool,
}

impl NmeaParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            data: GnssData::default(),
            in_sentence: false,
        }
    }

    /// Feed a byte to the parser. Returns true when a complete sentence is parsed.
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
                self.parse_sentence()
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

    /// Parse a complete NMEA sentence
    fn parse_sentence(&mut self) -> bool {
        // Verify checksum
        if !self.verify_checksum() {
            return false;
        }

        let sentence = core::str::from_utf8(&self.buffer).unwrap_or("");

        if sentence.starts_with("$GNGGA") || sentence.starts_with("$GPGGA") {
            self.parse_gga(sentence);
            true
        } else if sentence.starts_with("$GNRMC") || sentence.starts_with("$GPRMC") {
            self.parse_rmc(sentence);
            true
        } else if sentence.starts_with("$GNGSA") {
            self.parse_gsa(sentence);
            false
        } else if sentence.starts_with("$GIGSV") {
            // NavIC-specific satellites in view
            self.parse_navic_gsv(sentence);
            false
        } else if sentence.starts_with("$GPGSV") {
            self.parse_gps_gsv(sentence);
            false
        } else {
            false
        }
    }

    /// Verify NMEA checksum (XOR of all chars between $ and *)
    fn verify_checksum(&self) -> bool {
        let sentence = match core::str::from_utf8(&self.buffer) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let star_pos = match sentence.rfind('*') {
            Some(p) => p,
            None => return false,
        };

        if star_pos + 3 > sentence.len() {
            return false;
        }

        let mut checksum: u8 = 0;
        for &b in &self.buffer[1..star_pos] {
            checksum ^= b;
        }

        let expected = u8::from_str_radix(&sentence[star_pos + 1..star_pos + 3], 16).unwrap_or(0);
        checksum == expected
    }

    /// Parse GGA sentence (position fix)
    /// $GNGGA,hhmmss.ss,llll.lll,N,yyyyy.yyy,E,q,ss,hdop,alt,M,geoid,M,,*cs
    fn parse_gga(&mut self, sentence: &str) {
        let fields: Vec<&str, 16> = sentence.split(',').collect();
        if fields.len() < 14 {
            return;
        }

        // UTC time
        if let Some(time) = parse_f64(fields[1]) {
            self.data.utc_time = (time * 100.0) as u32;
        }

        // Latitude
        if let Some(lat) = parse_nmea_coord(fields[2]) {
            self.data.latitude = if fields[3] == "S" { -lat } else { lat };
        }

        // Longitude
        if let Some(lon) = parse_nmea_coord(fields[4]) {
            self.data.longitude = if fields[5] == "W" { -lon } else { lon };
        }

        // Fix quality
        self.data.fix_quality = match fields[6] {
            "0" => FixQuality::NoFix,
            "1" => FixQuality::GpsFix,
            "2" => FixQuality::DgpsFix,
            "3" => FixQuality::PpsFix,
            "4" => FixQuality::RtkFixed,
            "5" => FixQuality::RtkFloat,
            "6" => FixQuality::Estimated,
            _ => FixQuality::NoFix,
        };

        // Satellites used
        if let Some(sats) = parse_u32(fields[7]) {
            self.data.satellites_used = sats as u8;
        }

        // HDOP
        if let Some(hdop) = parse_f32(fields[8]) {
            self.data.hdop = hdop;
        }

        // Altitude
        if let Some(alt) = parse_f32(fields[9]) {
            self.data.altitude = alt;
        }

        self.data.valid = self.data.fix_quality != FixQuality::NoFix;
    }

    /// Parse RMC sentence (speed, heading, date)
    /// $GNRMC,hhmmss.ss,A,llll.ll,N,yyyyy.yy,E,sog,cog,ddmmyy,mv,mvE,mode*cs
    fn parse_rmc(&mut self, sentence: &str) {
        let fields: Vec<&str, 16> = sentence.split(',').collect();
        if fields.len() < 12 {
            return;
        }

        // Speed over ground (knots → km/h)
        if let Some(sog) = parse_f32(fields[7]) {
            self.data.speed_kmh = sog * 1.852;
        }

        // Course over ground
        if let Some(cog) = parse_f32(fields[8]) {
            self.data.heading = cog;
        }

        // Date
        if let Some(date) = parse_u32(fields[9]) {
            self.data.date = date;
        }
    }

    /// Parse GSA sentence (DOP and active satellites)
    fn parse_gsa(&mut self, _sentence: &str) {
        // DOP values already parsed from GGA
    }

    /// Parse NavIC-specific GSV sentence
    fn parse_navic_gsv(&mut self, sentence: &str) {
        let fields: Vec<&str, 24> = sentence.split(',').collect();
        if fields.len() >= 4 {
            if let Some(total) = parse_u32(fields[3]) {
                self.data.navic_sats_in_view = total as u8;
            }
        }
    }

    /// Parse GPS GSV sentence
    fn parse_gps_gsv(&mut self, sentence: &str) {
        let fields: Vec<&str, 24> = sentence.split(',').collect();
        if fields.len() >= 4 {
            if let Some(total) = parse_u32(fields[3]) {
                self.data.gps_sats_in_view = total as u8;
            }
        }
    }

    /// Get latitude as integer (degrees × 1e7) for protocol packets
    pub fn lat_i32(&self) -> i32 {
        (self.data.latitude * 1e7) as i32
    }

    /// Get longitude as integer (degrees × 1e7) for protocol packets
    pub fn lon_i32(&self) -> i32 {
        (self.data.longitude * 1e7) as i32
    }
}

// ─── Helper parsing functions ────────────────────────────────────────

fn parse_f64(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    // Simple float parser for no_std
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

fn parse_f32(s: &str) -> Option<f32> {
    parse_f64(s).map(|v| v as f32)
}

fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut result: u32 = 0;
    for c in s.chars() {
        match c {
            '0'..='9' => {
                result = result * 10 + (c as u32 - '0' as u32);
            }
            _ => return None,
        }
    }
    Some(result)
}

/// Parse NMEA coordinate format (DDDMM.MMMMM) to decimal degrees
fn parse_nmea_coord(s: &str) -> Option<f64> {
    let val = parse_f64(s)?;
    let degrees = (val / 100.0) as i32;
    let minutes = val - (degrees as f64) * 100.0;
    Some(degrees as f64 + minutes / 60.0)
}

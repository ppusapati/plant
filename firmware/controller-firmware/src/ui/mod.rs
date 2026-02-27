//! UI / Display Subsystem
//!
//! Manages the 3.5" ILI9488 TFT display (480×320, 16-bit color).
//! Renders multiple screens: Dashboard, Map, Sensors, Settings.
//! Uses embedded-graphics for rendering primitives.

use defmt::*;
use embedded_graphics::prelude::*;

/// Display dimensions
pub const DISPLAY_WIDTH: u32 = 480;
pub const DISPLAY_HEIGHT: u32 = 320;

/// Color palette (RGB565)
pub mod colors {
    pub const BLACK: u16 = 0x0000;
    pub const WHITE: u16 = 0xFFFF;
    pub const RED: u16 = 0xF800;
    pub const GREEN: u16 = 0x07E0;
    pub const BLUE: u16 = 0x001F;
    pub const YELLOW: u16 = 0xFFE0;
    pub const CYAN: u16 = 0x07FF;
    pub const ORANGE: u16 = 0xFD20;
    pub const DARK_GREEN: u16 = 0x03E0;
    pub const DARK_GRAY: u16 = 0x7BEF;
    pub const LIGHT_GRAY: u16 = 0xC618;
    pub const BG_COLOR: u16 = 0x18E3; // Dark blue-gray background
    pub const PANEL_COLOR: u16 = 0x2124; // Slightly lighter panel
    pub const ACCENT: u16 = 0x2C9F; // Teal accent
}

/// Display screens
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Screen {
    /// Main dashboard with key metrics
    Dashboard,
    /// Map view with robot position and waypoints
    MapView,
    /// Detailed sensor readings
    SensorDetail,
    /// Plant health NDVI visualization
    PlantHealth,
    /// Soil health details
    SoilHealth,
    /// Robot configuration
    Settings,
    /// Mission planning
    Mission,
}

/// Display update commands sent from other tasks
#[derive(Debug, Clone, defmt::Format)]
pub enum DisplayUpdate {
    /// Switch to a different screen
    SwitchScreen(Screen),
    /// Refresh telemetry data on current screen
    RefreshTelemetry,
    /// Show alert overlay
    ShowAlert(u8),
}

/// Dashboard layout definition
/// ```text
/// ┌──────────────────────────────────────────────┐
/// │ ADR-1 Controller     🔋 85%  📶 -78dBm  MODE │
/// ├──────────┬──────────┬──────────┬─────────────┤
/// │ LAT      │ LON      │ SPD      │ HDG         │
/// │ 12.9716  │ 77.5946  │ 0.8km/h  │ 045°        │
/// ├──────────┴──────────┼──────────┴─────────────┤
/// │    NDVI: 0.72       │   HEALTH: 847/1000     │
/// │    ██████████░░░░░  │   ██████████████░░░░   │
/// ├─────────────────────┼────────────────────────┤
/// │ Soil Moisture: 34%  │ Air Temp: 28.5°C       │
/// │ Soil Temp: 24.1°C   │ Humidity: 65%          │
/// │ pH: 6.5             │ Wind: 2.3 m/s          │
/// │ N:120 P:45 K:200    │ Light: 45000 lux       │
/// ├─────────────────────┴────────────────────────┤
/// │ STATUS: DESEEDING | Seeds: 1,234 | 2.1km done│
/// └──────────────────────────────────────────────┘
/// ```
pub struct DashboardLayout;

impl DashboardLayout {
    /// Header bar Y position
    pub const HEADER_Y: i32 = 0;
    pub const HEADER_H: i32 = 30;

    /// Navigation row
    pub const NAV_Y: i32 = 32;
    pub const NAV_H: i32 = 45;

    /// Health indicators row
    pub const HEALTH_Y: i32 = 79;
    pub const HEALTH_H: i32 = 50;

    /// Sensor data area
    pub const SENSOR_Y: i32 = 131;
    pub const SENSOR_H: i32 = 140;

    /// Status bar
    pub const STATUS_Y: i32 = 293;
    pub const STATUS_H: i32 = 27;
}

/// Map view renderer
pub struct MapView {
    /// Map center latitude
    pub center_lat: f64,
    /// Map center longitude
    pub center_lon: f64,
    /// Zoom level (meters per pixel)
    pub meters_per_pixel: f32,
    /// Robot trail points (circular buffer)
    pub trail: heapless::Vec<(i16, i16), 256>,
    /// Waypoint markers
    pub waypoints: heapless::Vec<(i16, i16, u8), 64>,
}

impl MapView {
    pub fn new() -> Self {
        Self {
            center_lat: 0.0,
            center_lon: 0.0,
            meters_per_pixel: 0.5,
            trail: heapless::Vec::new(),
            waypoints: heapless::Vec::new(),
        }
    }

    /// Convert GPS coordinate to screen pixel
    pub fn gps_to_screen(&self, lat: f64, lon: f64) -> (i16, i16) {
        let dlat = (lat - self.center_lat) * 111_132.92;
        let dlon = (lon - self.center_lon) * 111_132.92 * libm::cos(self.center_lat.to_radians());

        let px = (DISPLAY_WIDTH as f64 / 2.0 + dlon / self.meters_per_pixel as f64) as i16;
        let py = (DISPLAY_HEIGHT as f64 / 2.0 - dlat / self.meters_per_pixel as f64) as i16;

        (px, py)
    }

    /// Zoom in (decrease meters per pixel)
    pub fn zoom_in(&mut self) {
        self.meters_per_pixel = (self.meters_per_pixel * 0.7).max(0.1);
    }

    /// Zoom out (increase meters per pixel)
    pub fn zoom_out(&mut self) {
        self.meters_per_pixel = (self.meters_per_pixel * 1.4).min(10.0);
    }

    /// Add trail point
    pub fn add_trail_point(&mut self, x: i16, y: i16) {
        if self.trail.is_full() {
            // Remove oldest point (shift left)
            for i in 0..self.trail.len() - 1 {
                self.trail[i] = self.trail[i + 1];
            }
            self.trail.truncate(self.trail.len() - 1);
        }
        let _ = self.trail.push((x, y));
    }
}

/// Sensor detail screen data formatting
pub struct SensorDisplay;

impl SensorDisplay {
    /// Format NDVI value with color indication
    pub fn ndvi_color(ndvi: f32) -> u16 {
        if ndvi < 0.2 {
            colors::RED
        } else if ndvi < 0.4 {
            colors::ORANGE
        } else if ndvi < 0.6 {
            colors::YELLOW
        } else if ndvi < 0.8 {
            colors::GREEN
        } else {
            colors::DARK_GREEN
        }
    }

    /// Format health score with color
    pub fn health_color(score: u16) -> u16 {
        match score {
            0..=200 => colors::RED,
            201..=400 => colors::ORANGE,
            401..=600 => colors::YELLOW,
            601..=800 => colors::GREEN,
            801..=1000 => colors::DARK_GREEN,
            _ => colors::WHITE,
        }
    }

    /// Format battery level with color
    pub fn battery_color(soc: u8) -> u16 {
        match soc {
            0..=10 => colors::RED,
            11..=25 => colors::ORANGE,
            26..=50 => colors::YELLOW,
            51..=100 => colors::GREEN,
            _ => colors::WHITE,
        }
    }

    /// Format RSSI with signal strength bars (0-4)
    pub fn rssi_bars(rssi: i16) -> u8 {
        match rssi {
            _ if rssi > -70 => 4,
            _ if rssi > -90 => 3,
            _ if rssi > -110 => 2,
            _ if rssi > -130 => 1,
            _ => 0,
        }
    }

    /// Generate a progress bar as pixel data
    /// Returns (filled_width, total_width) for rendering
    pub fn progress_bar(value: f32, min: f32, max: f32, total_width: u16) -> u16 {
        let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
        (normalized * total_width as f32) as u16
    }
}

/// Alert types for overlay display
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum AlertType {
    /// Low battery warning
    LowBattery,
    /// Communication link lost
    LinkLost,
    /// Emergency stop active
    EmergencyStop,
    /// Obstacle detected
    Obstacle,
    /// Geofence boundary reached
    GeofenceBreach,
    /// Sensor error
    SensorError,
    /// Mission complete
    MissionComplete,
    /// Disease detected
    DiseaseAlert,
}

impl AlertType {
    pub fn message(&self) -> &'static str {
        match self {
            Self::LowBattery => "LOW BATTERY",
            Self::LinkLost => "LINK LOST",
            Self::EmergencyStop => "EMERGENCY STOP",
            Self::Obstacle => "OBSTACLE DETECTED",
            Self::GeofenceBreach => "GEOFENCE BREACH",
            Self::SensorError => "SENSOR ERROR",
            Self::MissionComplete => "MISSION COMPLETE",
            Self::DiseaseAlert => "DISEASE DETECTED",
        }
    }

    pub fn color(&self) -> u16 {
        match self {
            Self::LowBattery | Self::LinkLost | Self::EmergencyStop | Self::GeofenceBreach => {
                colors::RED
            }
            Self::Obstacle | Self::SensorError => colors::ORANGE,
            Self::MissionComplete => colors::GREEN,
            Self::DiseaseAlert => colors::YELLOW,
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::EmergencyStop | Self::LinkLost | Self::GeofenceBreach
        )
    }
}

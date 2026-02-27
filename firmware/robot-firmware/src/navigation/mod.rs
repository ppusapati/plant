//! Navigation Subsystem
//!
//! Implements sensor fusion using Extended Kalman Filter (EKF):
//! - NavIC/GPS position (10 Hz)
//! - BNO055 IMU (100 Hz) - accelerometer, gyroscope, magnetometer
//! - Wheel odometry (100 Hz) - 4× encoders
//! - Output: Fused position/heading at 50 Hz

pub mod ekf;
pub mod path_planner;

use defmt::*;

/// Navigation state vector
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct NavState {
    /// X position in local frame (meters from home)
    pub x: f32,
    /// Y position in local frame (meters from home)
    pub y: f32,
    /// Heading (radians, 0 = North, CW positive)
    pub theta: f32,
    /// Forward velocity (m/s)
    pub velocity: f32,
    /// Angular velocity (rad/s)
    pub omega: f32,
    /// Latitude (degrees)
    pub latitude: f64,
    /// Longitude (degrees)
    pub longitude: f64,
    /// Altitude (meters)
    pub altitude: f32,
    /// Fix quality (0=none, 1=GPS, 4=RTK fixed)
    pub fix_quality: u8,
    /// Number of satellites
    pub sat_count: u8,
    /// HDOP
    pub hdop: f32,
    /// Timestamp (ms since boot)
    pub timestamp_ms: u64,
}

/// Home position (reference point for local frame)
#[derive(Debug, Clone, Copy)]
pub struct HomePosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub set: bool,
}

impl Default for HomePosition {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            set: false,
        }
    }
}

impl HomePosition {
    /// Convert GPS lat/lon to local X/Y (meters) relative to home
    /// Uses equirectangular approximation (valid for short distances <10km)
    pub fn gps_to_local(&self, lat: f64, lon: f64) -> (f32, f32) {
        if !self.set {
            return (0.0, 0.0);
        }

        let lat_rad = lat.to_radians();
        let home_lat_rad = self.latitude.to_radians();

        // Meters per degree at this latitude
        let m_per_deg_lat: f64 = 111_132.92;
        let m_per_deg_lon: f64 = 111_132.92 * libm::cos(home_lat_rad);

        let dx = (lon - self.longitude) * m_per_deg_lon;
        let dy = (lat - self.latitude) * m_per_deg_lat;

        (dx as f32, dy as f32)
    }

    /// Convert local X/Y back to GPS lat/lon
    pub fn local_to_gps(&self, x: f32, y: f32) -> (f64, f64) {
        if !self.set {
            return (0.0, 0.0);
        }

        let home_lat_rad = self.latitude.to_radians();
        let m_per_deg_lat: f64 = 111_132.92;
        let m_per_deg_lon: f64 = 111_132.92 * libm::cos(home_lat_rad);

        let lat = self.latitude + (y as f64) / m_per_deg_lat;
        let lon = self.longitude + (x as f64) / m_per_deg_lon;

        (lat, lon)
    }
}

/// Wheel odometry calculator
pub struct WheelOdometry {
    /// Wheel diameter in meters
    wheel_diameter: f32,
    /// Track width (distance between left and right wheels) in meters
    track_width: f32,
    /// Encoder counts per revolution
    counts_per_rev: u32,
    /// Previous encoder counts [FL, FR, RL, RR]
    prev_counts: [i32; 4],
    /// Accumulated distance (meters)
    pub total_distance: f32,
}

impl WheelOdometry {
    pub fn new(wheel_diameter: f32, track_width: f32, counts_per_rev: u32) -> Self {
        Self {
            wheel_diameter,
            track_width,
            counts_per_rev,
            prev_counts: [0; 4],
            total_distance: 0.0,
        }
    }

    /// Update odometry with new encoder counts
    /// Returns (delta_distance, delta_theta)
    pub fn update(&mut self, counts: [i32; 4]) -> (f32, f32) {
        let meters_per_count =
            core::f32::consts::PI * self.wheel_diameter / self.counts_per_rev as f32;

        // Calculate delta for each wheel
        let dl_fl = (counts[0] - self.prev_counts[0]) as f32 * meters_per_count;
        let dl_fr = (counts[1] - self.prev_counts[1]) as f32 * meters_per_count;
        let dl_rl = (counts[2] - self.prev_counts[2]) as f32 * meters_per_count;
        let dl_rr = (counts[3] - self.prev_counts[3]) as f32 * meters_per_count;

        self.prev_counts = counts;

        // Average left and right sides
        let dl_left = (dl_fl + dl_rl) / 2.0;
        let dl_right = (dl_fr + dl_rr) / 2.0;

        // Differential drive kinematics
        let delta_distance = (dl_left + dl_right) / 2.0;
        let delta_theta = (dl_right - dl_left) / self.track_width;

        self.total_distance += libm::fabsf(delta_distance);

        (delta_distance, delta_theta)
    }

    /// Get current speed estimate from last update delta and dt
    pub fn speed(&self, delta_distance: f32, dt_s: f32) -> f32 {
        if dt_s > 0.0 {
            delta_distance / dt_s
        } else {
            0.0
        }
    }
}

/// Geofence boundary checker
pub struct Geofence {
    /// Boundary polygon vertices (local X, Y in meters)
    vertices: heapless::Vec<(f32, f32), 32>,
    /// Maximum radius from home (meters)
    max_radius: f32,
    /// Geofence active flag
    active: bool,
}

impl Geofence {
    pub fn new(max_radius: f32) -> Self {
        Self {
            vertices: heapless::Vec::new(),
            max_radius,
            active: false,
        }
    }

    /// Add a vertex to the geofence polygon
    pub fn add_vertex(&mut self, x: f32, y: f32) -> bool {
        self.vertices.push((x, y)).is_ok()
    }

    /// Activate the geofence
    pub fn activate(&mut self) {
        if self.vertices.len() >= 3 || self.max_radius > 0.0 {
            self.active = true;
        }
    }

    /// Check if position is within geofence
    pub fn is_within(&self, x: f32, y: f32) -> bool {
        if !self.active {
            return true; // If not active, always "within"
        }

        // Check radius first
        let distance = libm::sqrtf(x * x + y * y);
        if distance > self.max_radius {
            return false;
        }

        // If polygon defined, check point-in-polygon
        if self.vertices.len() >= 3 {
            return self.point_in_polygon(x, y);
        }

        true
    }

    /// Ray casting algorithm for point-in-polygon test
    fn point_in_polygon(&self, x: f32, y: f32) -> bool {
        let n = self.vertices.len();
        let mut inside = false;
        let mut j = n - 1;

        for i in 0..n {
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];

            if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }
}

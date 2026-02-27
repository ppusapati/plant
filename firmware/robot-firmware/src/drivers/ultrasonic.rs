//! HC-SR04 Ultrasonic Sensor Driver
//!
//! 4× HC-SR04 sensors for obstacle detection:
//! - Front (PE0/PE1): Forward obstacle detection
//! - Right (PE2/PE3): Row tracking / lateral obstacle
//! - Rear  (PE4/PE5): Reversing safety
//! - Left  (PE6/PE7): Row tracking / lateral obstacle
//!
//! Uses GPIO trigger (10µs pulse) and timer capture for echo measurement.
//! Distance = (echo_time_us × 0.0343) / 2 cm

use defmt::*;

/// Speed of sound at 20°C in cm/µs
const SPEED_OF_SOUND_CM_US: f32 = 0.0343;

/// Maximum detection range in cm
const MAX_RANGE_CM: f32 = 400.0;

/// Minimum detection range in cm
const MIN_RANGE_CM: f32 = 2.0;

/// Timeout for echo (max range ≈ 23ms round trip)
const ECHO_TIMEOUT_US: u32 = 25_000;

/// Ultrasonic sensor position
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SensorPosition {
    Front = 0,
    Right = 1,
    Rear = 2,
    Left = 3,
}

/// Single sensor measurement
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct UltrasonicMeasurement {
    pub position: SensorPosition,
    /// Distance in millimeters
    pub distance_mm: u16,
    /// Measurement is valid
    pub valid: bool,
    /// Raw echo time in microseconds
    pub echo_us: u32,
}

/// All four ultrasonic sensor readings
#[derive(Debug, Clone, defmt::Format)]
pub struct ObstacleMap {
    pub front: UltrasonicMeasurement,
    pub right: UltrasonicMeasurement,
    pub rear: UltrasonicMeasurement,
    pub left: UltrasonicMeasurement,
    /// Minimum obstacle distance across all sensors (mm)
    pub min_distance_mm: u16,
    /// Direction of closest obstacle
    pub closest_direction: SensorPosition,
}

impl ObstacleMap {
    pub fn new() -> Self {
        let default_measurement = UltrasonicMeasurement {
            position: SensorPosition::Front,
            distance_mm: u16::MAX,
            valid: false,
            echo_us: 0,
        };

        Self {
            front: UltrasonicMeasurement {
                position: SensorPosition::Front,
                ..default_measurement
            },
            right: UltrasonicMeasurement {
                position: SensorPosition::Right,
                ..default_measurement
            },
            rear: UltrasonicMeasurement {
                position: SensorPosition::Rear,
                ..default_measurement
            },
            left: UltrasonicMeasurement {
                position: SensorPosition::Left,
                ..default_measurement
            },
            min_distance_mm: u16::MAX,
            closest_direction: SensorPosition::Front,
        }
    }

    /// Update a sensor reading
    pub fn update(&mut self, measurement: UltrasonicMeasurement) {
        match measurement.position {
            SensorPosition::Front => self.front = measurement,
            SensorPosition::Right => self.right = measurement,
            SensorPosition::Rear => self.rear = measurement,
            SensorPosition::Left => self.left = measurement,
        }
        self.recalculate_min();
    }

    /// Recalculate minimum distance and direction
    fn recalculate_min(&mut self) {
        let sensors = [&self.front, &self.right, &self.rear, &self.left];
        self.min_distance_mm = u16::MAX;

        for sensor in &sensors {
            if sensor.valid && sensor.distance_mm < self.min_distance_mm {
                self.min_distance_mm = sensor.distance_mm;
                self.closest_direction = sensor.position;
            }
        }
    }

    /// Check if any obstacle is within emergency stop distance (500mm)
    pub fn emergency_obstacle(&self) -> bool {
        self.min_distance_mm < 500
    }

    /// Check if front path is clear for given distance (mm)
    pub fn front_clear(&self, distance_mm: u16) -> bool {
        !self.front.valid || self.front.distance_mm > distance_mm
    }
}

/// Convert echo time (microseconds) to distance (millimeters)
pub fn echo_to_distance_mm(echo_us: u32) -> Option<u16> {
    if echo_us == 0 || echo_us > ECHO_TIMEOUT_US {
        return None;
    }

    let distance_cm = (echo_us as f32 * SPEED_OF_SOUND_CM_US) / 2.0;

    if distance_cm < MIN_RANGE_CM || distance_cm > MAX_RANGE_CM {
        return None;
    }

    Some((distance_cm * 10.0) as u16) // Convert cm to mm
}

/// Apply temperature compensation to speed of sound
/// Speed = 331.3 + 0.606 × T(°C) m/s
pub fn compensated_distance_mm(echo_us: u32, temperature_c: f32) -> Option<u16> {
    if echo_us == 0 || echo_us > ECHO_TIMEOUT_US {
        return None;
    }

    let speed_m_s = 331.3 + 0.606 * temperature_c;
    let speed_cm_us = speed_m_s / 10_000.0; // m/s → cm/µs
    let distance_cm = (echo_us as f32 * speed_cm_us) / 2.0;

    if distance_cm < MIN_RANGE_CM || distance_cm > MAX_RANGE_CM {
        return None;
    }

    Some((distance_cm * 10.0) as u16)
}

/// Median filter for ultrasonic measurements (removes outliers)
pub fn median_filter(samples: &mut [u16; 5]) -> u16 {
    // Simple insertion sort for 5 elements
    for i in 1..5 {
        let key = samples[i];
        let mut j = i;
        while j > 0 && samples[j - 1] > key {
            samples[j] = samples[j - 1];
            j -= 1;
        }
        samples[j] = key;
    }
    samples[2] // Return median
}

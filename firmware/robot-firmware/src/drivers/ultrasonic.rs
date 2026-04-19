//! MaxBotix MB1240 XL-MaxSonar-EZ4 Industrial Ultrasonic Driver
//!
//! 4× MB1240 sensors for obstacle detection (Industrial grade, -40°C to +85°C, IP67):
//! - Front (PE0): Forward obstacle detection
//! - Right (PE2): Row tracking / lateral obstacle
//! - Rear  (PE4): Reversing safety
//! - Left  (PE6): Row tracking / lateral obstacle
//!
//! Supports analog voltage output (Vcc/1024 per cm) and UART serial output.
//! Range: 20cm to 765cm, resolution: 1cm, accuracy: ±1cm.

use defmt::*;
use embassy_stm32::gpio::Input;
use embassy_time::{Duration, Instant, Timer};

/// Speed of sound at 20°C in cm/µs
const SPEED_OF_SOUND_CM_US: f32 = 0.0343;

/// Maximum detection range in cm (MB1240: 765cm)
const MAX_RANGE_CM: f32 = 765.0;

/// Minimum detection range in cm (MB1240: 20cm)
const MIN_RANGE_CM: f32 = 20.0;

/// Timeout for echo (max range ≈ 45ms round trip at 765cm)
const ECHO_TIMEOUT_US: u32 = 50_000;

/// ADC voltage per centimeter (Vcc/1024 per cm for MB1240)
/// At 3.3V Vcc: 3.3/1024 ≈ 3.222mV per cm
const MV_PER_CM: f32 = 3.222;

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

/// Async driver for a single MaxBotix MB1240 ultrasonic sensor.
///
/// The MB1240 continuously measures and drives its PW (pulse-width) output
/// pin HIGH for 147 µs per inch of range (≈ 57.9 µs/cm).  This driver
/// detects each pulse via an Embassy `Input` GPIO and records its width
/// using `Instant`, which has ≈ 30 µs resolution at 32.768 kHz — yielding
/// ≈ ±1 cm absolute accuracy, sufficient for obstacle detection.
///
/// Measurements are passed through a 5-sample median filter to reject
/// spurious readings caused by acoustic reflections.
///
/// Usage:
/// ```
/// let sensor = Mb1240::new(SensorPosition::Front, pw_input_pin);
/// loop {
///     let m = sensor.measure().await;
///     obstacle_map.update(m);
/// }
/// ```
pub struct Mb1240 {
    /// Physical mounting position of this sensor.
    pub position: SensorPosition,
    /// GPIO connected to the MB1240 PW output (active-high pulse).
    pw: Input<'static>,
    /// Circular buffer of recent raw distance readings (mm) for median filter.
    filter_buf: [u16; 5],
    /// Write index into `filter_buf`.
    filter_idx: usize,
    /// Air temperature used for speed-of-sound compensation (°C, default 25 °C).
    pub air_temp_c: f32,
}

impl Mb1240 {
    /// Create a new MB1240 driver.
    ///
    /// `pw` must be connected to the sensor's PW (pulse-width) output pin.
    /// The sensor drives this pin actively (no pull resistors needed).
    pub fn new(position: SensorPosition, pw: Input<'static>) -> Self {
        Self {
            position,
            pw,
            filter_buf: [u16::MAX; 5],
            filter_idx: 0,
            air_temp_c: 25.0,
        }
    }

    /// Take a single temperature-compensated, median-filtered distance
    /// measurement (async).
    ///
    /// Waits for the next complete PW pulse (rising → falling edge) and
    /// returns an [`UltrasonicMeasurement`].  Times out after 120 ms; if
    /// the sensor is not responding the returned measurement has `valid = false`
    /// and `distance_mm = u16::MAX`.
    pub async fn measure(&mut self) -> UltrasonicMeasurement {
        let echo_us = self.measure_pulse_us().await;

        let raw_distance =
            echo_us.and_then(|us| compensated_distance_mm(us, self.air_temp_c));

        let distance_mm = if let Some(d) = raw_distance {
            // Push into the sliding window and return the median
            self.filter_buf[self.filter_idx % 5] = d;
            self.filter_idx = self.filter_idx.wrapping_add(1);
            let mut buf = self.filter_buf;
            median_filter(&mut buf)
        } else {
            u16::MAX
        };

        let valid = raw_distance.is_some();

        if valid {
            debug!(
                "Ultrasonic[{}]: {} mm",
                self.position as u8,
                distance_mm,
            );
        }

        UltrasonicMeasurement {
            position: self.position,
            distance_mm,
            valid,
            echo_us: echo_us.unwrap_or(0),
        }
    }

    /// Measure the PW pulse width in microseconds by watching GPIO edges.
    ///
    /// Returns `None` if any phase of the measurement times out (120 ms limit
    /// covers the full cycle period with margin).
    async fn measure_pulse_us(&mut self) -> Option<u32> {
        const TIMEOUT: Duration = Duration::from_millis(120);
        const POLL: Duration = Duration::from_micros(50);

        let deadline = Instant::now() + TIMEOUT;

        // 1. If the pin is already HIGH, wait for it to go LOW first so we
        //    catch a clean rising edge (not the tail of a previous pulse).
        if self.pw.is_high() {
            loop {
                if self.pw.is_low() {
                    break;
                }
                if Instant::now() >= deadline {
                    warn!("Ultrasonic[{}]: timeout waiting for LOW", self.position as u8);
                    return None;
                }
                Timer::after(POLL).await;
            }
        }

        // 2. Wait for the rising edge.
        loop {
            if self.pw.is_high() {
                break;
            }
            if Instant::now() >= deadline {
                warn!("Ultrasonic[{}]: timeout waiting for rising edge", self.position as u8);
                return None;
            }
            Timer::after(POLL).await;
        }

        let start = Instant::now();

        // 3. Wait for the falling edge.
        loop {
            if self.pw.is_low() {
                break;
            }
            if Instant::now() >= deadline {
                warn!("Ultrasonic[{}]: timeout waiting for falling edge", self.position as u8);
                return None;
            }
            Timer::after(POLL).await;
        }

        Some((Instant::now() - start).as_micros() as u32)
    }
}

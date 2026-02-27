//! Motor Control & Deseeding Mechanism Subsystem
//!
//! Controls:
//! - 4× BLDC drive motors via CAN bus (differential/skid steering)
//! - 1× Deseeding rotary motor (BTS7960 H-bridge, PWM)
//! - 1× Vacuum pump motor (12V DC, relay/PWM)
//! - 2× Servo motors (soil probe deployment)
//!
//! Implements differential drive kinematics and PID speed control.

use defmt::*;

/// Robot operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum RobotMode {
    /// Robot is idle, motors disabled
    Idle = 0,
    /// Manual control from joystick
    Manual = 1,
    /// Autonomous waypoint navigation
    Autonomous = 2,
    /// Autonomous deseeding pattern
    Deseeding = 3,
    /// Returning to home position
    ReturnToHome = 4,
    /// Emergency stop (all motors disabled)
    EmergencyStop = 5,
    /// Soil sampling mode
    SoilSampling = 6,
}

/// Individual motor state
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct MotorState {
    /// Target speed (RPM)
    pub target_rpm: i16,
    /// Actual speed (RPM, from encoder/driver feedback)
    pub actual_rpm: i16,
    /// Motor current (mA)
    pub current_ma: u16,
    /// Motor temperature (°C)
    pub temperature: i8,
    /// Motor enabled flag
    pub enabled: bool,
    /// Fault flag
    pub fault: bool,
}

/// Drive system state
#[derive(Debug, Clone, defmt::Format)]
pub struct DriveState {
    pub front_left: MotorState,
    pub front_right: MotorState,
    pub rear_left: MotorState,
    pub rear_right: MotorState,
    pub deseeder: MotorState,
    pub vacuum_on: bool,
}

impl DriveState {
    pub fn new() -> Self {
        Self {
            front_left: MotorState::default(),
            front_right: MotorState::default(),
            rear_left: MotorState::default(),
            rear_right: MotorState::default(),
            deseeder: MotorState::default(),
            vacuum_on: false,
        }
    }

    /// Check if any motor has a fault
    pub fn has_fault(&self) -> bool {
        self.front_left.fault
            || self.front_right.fault
            || self.rear_left.fault
            || self.rear_right.fault
            || self.deseeder.fault
    }

    /// Get total drive current (mA)
    pub fn total_current_ma(&self) -> u32 {
        self.front_left.current_ma as u32
            + self.front_right.current_ma as u32
            + self.rear_left.current_ma as u32
            + self.rear_right.current_ma as u32
            + self.deseeder.current_ma as u32
    }
}

/// Differential drive kinematics
pub struct DifferentialDrive {
    /// Track width (distance between left and right wheels) in meters
    track_width: f32,
    /// Wheel diameter in meters
    wheel_diameter: f32,
    /// Maximum wheel RPM
    max_rpm: i16,
    /// Gear ratio (motor RPM / wheel RPM)
    gear_ratio: f32,
}

impl DifferentialDrive {
    pub fn new(track_width: f32, wheel_diameter: f32, max_rpm: i16, gear_ratio: f32) -> Self {
        Self {
            track_width,
            wheel_diameter,
            max_rpm,
            gear_ratio,
        }
    }

    /// Convert linear + angular velocity to individual wheel RPMs
    /// Returns (left_rpm, right_rpm)
    pub fn velocity_to_wheel_rpm(&self, linear_vel: f32, angular_vel: f32) -> (i16, i16) {
        // v_left = v - ω × L/2
        // v_right = v + ω × L/2
        let v_left = linear_vel - angular_vel * self.track_width / 2.0;
        let v_right = linear_vel + angular_vel * self.track_width / 2.0;

        // Convert m/s to wheel RPM
        // RPM = (v × 60) / (π × d)
        let wheel_circumference = core::f32::consts::PI * self.wheel_diameter;
        let rpm_left = (v_left * 60.0 / wheel_circumference * self.gear_ratio) as i16;
        let rpm_right = (v_right * 60.0 / wheel_circumference * self.gear_ratio) as i16;

        // Clamp to maximum RPM
        let rpm_left = rpm_left.clamp(-self.max_rpm, self.max_rpm);
        let rpm_right = rpm_right.clamp(-self.max_rpm, self.max_rpm);

        (rpm_left, rpm_right)
    }

    /// Convert wheel RPMs to linear + angular velocity
    pub fn wheel_rpm_to_velocity(&self, left_rpm: i16, right_rpm: i16) -> (f32, f32) {
        let wheel_circumference = core::f32::consts::PI * self.wheel_diameter;

        let v_left = left_rpm as f32 / self.gear_ratio * wheel_circumference / 60.0;
        let v_right = right_rpm as f32 / self.gear_ratio * wheel_circumference / 60.0;

        let linear = (v_left + v_right) / 2.0;
        let angular = (v_right - v_left) / self.track_width;

        (linear, angular)
    }
}

/// PID controller for motor speed regulation
pub struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    prev_error: f32,
    integral_limit: f32,
    output_min: f32,
    output_max: f32,
}

impl PidController {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            integral_limit: 1000.0,
            output_min: -1.0,
            output_max: 1.0,
        }
    }

    pub fn set_limits(&mut self, min: f32, max: f32) {
        self.output_min = min;
        self.output_max = max;
    }

    pub fn set_integral_limit(&mut self, limit: f32) {
        self.integral_limit = limit;
    }

    /// Compute PID output
    pub fn compute(&mut self, setpoint: f32, measurement: f32, dt: f32) -> f32 {
        let error = setpoint - measurement;

        // Proportional
        let p_term = self.kp * error;

        // Integral with anti-windup
        self.integral += error * dt;
        self.integral = self
            .integral
            .clamp(-self.integral_limit, self.integral_limit);
        let i_term = self.ki * self.integral;

        // Derivative (on measurement to avoid derivative kick)
        let derivative = if dt > 0.0 {
            (error - self.prev_error) / dt
        } else {
            0.0
        };
        let d_term = self.kd * derivative;
        self.prev_error = error;

        // Output with clamping
        let output = p_term + i_term + d_term;
        output.clamp(self.output_min, self.output_max)
    }

    /// Reset PID state
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

/// CAN message builder for motor controllers
pub mod can_messages {
    /// Build CAN speed command message for motor driver
    /// Format: [target_rpm:i16, max_current:u16, mode:u8, reserved:3B]
    pub fn build_speed_command(target_rpm: i16, max_current_ma: u16) -> [u8; 8] {
        let rpm_bytes = target_rpm.to_le_bytes();
        let current_bytes = max_current_ma.to_le_bytes();

        [
            rpm_bytes[0],
            rpm_bytes[1],
            current_bytes[0],
            current_bytes[1],
            0x01, // mode: velocity
            0x00,
            0x00,
            0x00,
        ]
    }

    /// Parse motor status from CAN message
    /// Format: [actual_rpm:i16, current_ma:u16, temp:i8, fault:u8, reserved:2B]
    pub fn parse_motor_status(data: &[u8; 8]) -> (i16, u16, i8, u8) {
        let rpm = i16::from_le_bytes([data[0], data[1]]);
        let current = u16::from_le_bytes([data[2], data[3]]);
        let temp = data[4] as i8;
        let fault = data[5];
        (rpm, current, temp, fault)
    }

    /// Build emergency stop broadcast message (CAN ID 0x7FF)
    pub fn build_estop() -> [u8; 0] {
        [] // DLC=0, just the ID triggers stop
    }

    /// CAN IDs for motor communication
    pub const MOTOR_FL_CMD: u16 = 0x100;
    pub const MOTOR_FR_CMD: u16 = 0x101;
    pub const MOTOR_RL_CMD: u16 = 0x102;
    pub const MOTOR_RR_CMD: u16 = 0x103;
    pub const MOTOR_FL_STATUS: u16 = 0x180;
    pub const MOTOR_FR_STATUS: u16 = 0x181;
    pub const MOTOR_RL_STATUS: u16 = 0x182;
    pub const MOTOR_RR_STATUS: u16 = 0x183;
    pub const DESEEDER_CMD: u16 = 0x200;
    pub const DESEEDER_STATUS: u16 = 0x280;
    pub const BMS_STATUS: u16 = 0x300;
    pub const BMS_CELLS: u16 = 0x301;
    pub const ESTOP_BROADCAST: u16 = 0x7FF;
}

/// Deseeding mechanism controller
pub struct DeseedingController {
    /// Rotary hoe speed (RPM)
    pub hoe_rpm: u16,
    /// Hoe active flag
    pub hoe_active: bool,
    /// Vacuum pump active
    pub vacuum_active: bool,
    /// Working depth (mm)
    pub depth_mm: u16,
    /// Seed count (estimated from vacuum flow)
    pub seed_count: u32,
}

impl DeseedingController {
    pub fn new() -> Self {
        Self {
            hoe_rpm: 0,
            hoe_active: false,
            vacuum_active: false,
            depth_mm: 30, // Default 30mm depth
            seed_count: 0,
        }
    }

    /// Start deseeding with specified parameters
    pub fn start(&mut self, rpm: u16, depth_mm: u16) {
        self.hoe_rpm = rpm;
        self.depth_mm = depth_mm;
        self.hoe_active = true;
        self.vacuum_active = true;
        info!("Deseeding started: {}RPM, {}mm depth", rpm, depth_mm);
    }

    /// Stop deseeding
    pub fn stop(&mut self) {
        self.hoe_active = false;
        self.vacuum_active = false;
        self.hoe_rpm = 0;
        info!("Deseeding stopped. Seeds collected: {}", self.seed_count);
    }
}

/// Soil probe deployment controller
pub struct ProbeController {
    /// Probe is deployed (lowered into soil)
    pub deployed: bool,
    /// Current depth (mm)
    pub depth_mm: u16,
    /// Target depth (mm)
    pub target_depth_mm: u16,
    /// Deployment in progress
    pub moving: bool,
}

impl ProbeController {
    pub fn new() -> Self {
        Self {
            deployed: false,
            depth_mm: 0,
            target_depth_mm: 70, // Default 70mm
            moving: false,
        }
    }

    /// Deploy probe to target depth
    pub fn deploy(&mut self) {
        self.moving = true;
        info!("Deploying soil probe to {}mm", self.target_depth_mm);
    }

    /// Retract probe
    pub fn retract(&mut self) {
        self.moving = true;
        info!("Retracting soil probe");
    }

    /// Update probe state (call from servo control loop)
    pub fn update(&mut self, current_depth: u16) {
        self.depth_mm = current_depth;

        if self.moving {
            if self.deployed {
                // Retracting
                if current_depth == 0 {
                    self.deployed = false;
                    self.moving = false;
                }
            } else {
                // Deploying
                if current_depth >= self.target_depth_mm {
                    self.deployed = true;
                    self.moving = false;
                }
            }
        }
    }
}

//! Path Planner for Autonomous Navigation
//!
//! Implements:
//! - Pure pursuit path following
//! - Waypoint-based mission planning
//! - Row-following for crop deseeding patterns

use defmt::*;
use heapless::Vec;
use libm::{atan2f, cosf, sinf, sqrtf};

use crate::protocol::Waypoint;

/// Maximum number of waypoints in a mission
const MAX_WAYPOINTS: usize = 64;

/// Pure pursuit lookahead distance (meters)
const LOOKAHEAD_DISTANCE: f32 = 1.0;

/// Waypoint arrival threshold (meters)
const WAYPOINT_THRESHOLD: f32 = 0.5;

/// Maximum linear velocity (m/s)
const MAX_LINEAR_VEL: f32 = 1.5 / 3.6; // 1.5 km/h → m/s

/// Maximum angular velocity (rad/s)
const MAX_ANGULAR_VEL: f32 = 1.0;

/// Mission state
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum MissionState {
    Idle,
    Running,
    Paused,
    ReturningHome,
    Completed,
    Error,
}

/// Path planner for autonomous navigation
pub struct PathPlanner {
    /// Mission waypoints
    waypoints: Vec<PlannerWaypoint, MAX_WAYPOINTS>,
    /// Current waypoint index
    current_wp: usize,
    /// Mission state
    pub state: MissionState,
    /// Home position (first waypoint or initial position)
    home_x: f32,
    home_y: f32,
    /// Pure pursuit lookahead distance
    lookahead: f32,
}

/// Internal waypoint with local coordinates
#[derive(Debug, Clone)]
struct PlannerWaypoint {
    /// X position in local frame (meters)
    x: f32,
    /// Y position in local frame (meters)
    y: f32,
    /// Action at this waypoint
    action: WaypointActionLocal,
    /// Target speed to this waypoint (m/s)
    speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaypointActionLocal {
    PassThrough,
    Stop,
    Deseed,
    SampleSoil,
    PhotoCapture,
}

/// Drive command output from path planner
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct DriveCommand {
    /// Linear velocity (m/s, positive = forward)
    pub linear: f32,
    /// Angular velocity (rad/s, positive = left)
    pub angular: f32,
    /// Deseeder should be active
    pub deseed: bool,
    /// Should stop at current position
    pub stop: bool,
}

impl PathPlanner {
    pub fn new() -> Self {
        Self {
            waypoints: Vec::new(),
            current_wp: 0,
            state: MissionState::Idle,
            home_x: 0.0,
            home_y: 0.0,
            lookahead: LOOKAHEAD_DISTANCE,
        }
    }

    /// Set home position
    pub fn set_home(&mut self, x: f32, y: f32) {
        self.home_x = x;
        self.home_y = y;
    }

    /// Add a waypoint to the mission
    pub fn add_waypoint(&mut self, x: f32, y: f32, action: u8, speed: f32) -> bool {
        let wp_action = match action {
            0 => WaypointActionLocal::PassThrough,
            1 => WaypointActionLocal::Stop,
            2 => WaypointActionLocal::Deseed,
            3 => WaypointActionLocal::SampleSoil,
            4 => WaypointActionLocal::PhotoCapture,
            _ => WaypointActionLocal::PassThrough,
        };

        self.waypoints
            .push(PlannerWaypoint {
                x,
                y,
                action: wp_action,
                speed: speed.min(MAX_LINEAR_VEL),
            })
            .is_ok()
    }

    /// Clear all waypoints
    pub fn clear_mission(&mut self) {
        self.waypoints.clear();
        self.current_wp = 0;
        self.state = MissionState::Idle;
    }

    /// Start the mission
    pub fn start(&mut self) {
        if !self.waypoints.is_empty() {
            self.current_wp = 0;
            self.state = MissionState::Running;
        }
    }

    /// Pause the mission
    pub fn pause(&mut self) {
        if self.state == MissionState::Running {
            self.state = MissionState::Paused;
        }
    }

    /// Resume the mission
    pub fn resume(&mut self) {
        if self.state == MissionState::Paused {
            self.state = MissionState::Running;
        }
    }

    /// Initiate return to home
    pub fn return_to_home(&mut self) {
        self.state = MissionState::ReturningHome;
    }

    /// Compute drive command based on current position and heading
    /// Uses Pure Pursuit path following algorithm
    pub fn compute(&self, robot_x: f32, robot_y: f32, robot_theta: f32) -> DriveCommand {
        match self.state {
            MissionState::Idle | MissionState::Completed | MissionState::Error => {
                return DriveCommand {
                    linear: 0.0,
                    angular: 0.0,
                    deseed: false,
                    stop: true,
                };
            }
            MissionState::Paused => {
                return DriveCommand {
                    linear: 0.0,
                    angular: 0.0,
                    deseed: false,
                    stop: true,
                };
            }
            _ => {}
        }

        // Target waypoint
        let (target_x, target_y, target_speed, deseed) = if self.state == MissionState::ReturningHome
        {
            (self.home_x, self.home_y, MAX_LINEAR_VEL, false)
        } else if self.current_wp < self.waypoints.len() {
            let wp = &self.waypoints[self.current_wp];
            (
                wp.x,
                wp.y,
                wp.speed,
                wp.action == WaypointActionLocal::Deseed,
            )
        } else {
            return DriveCommand {
                linear: 0.0,
                angular: 0.0,
                deseed: false,
                stop: true,
            };
        };

        // Distance to target
        let dx = target_x - robot_x;
        let dy = target_y - robot_y;
        let distance = sqrtf(dx * dx + dy * dy);

        // Check if we've arrived at waypoint
        if distance < WAYPOINT_THRESHOLD {
            return DriveCommand {
                linear: 0.0,
                angular: 0.0,
                deseed,
                stop: true,
            };
        }

        // Pure pursuit: compute curvature to lookahead point
        let target_angle = atan2f(dy, dx);
        let alpha = normalize_angle(target_angle - robot_theta);

        // Curvature = 2 × sin(α) / lookahead_distance
        let ld = if distance < self.lookahead {
            distance
        } else {
            self.lookahead
        };
        let curvature = 2.0 * sinf(alpha) / ld;

        // Convert curvature to angular velocity
        let angular = (curvature * target_speed).clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);

        // Reduce speed when turning sharply
        let turn_factor = 1.0 - libm::fabsf(alpha) / core::f32::consts::PI;
        let linear = (target_speed * turn_factor).clamp(0.0, MAX_LINEAR_VEL);

        DriveCommand {
            linear,
            angular,
            deseed,
            stop: false,
        }
    }

    /// Advance to next waypoint (called when current waypoint action is complete)
    pub fn advance_waypoint(&mut self) {
        if self.state == MissionState::ReturningHome {
            self.state = MissionState::Completed;
            return;
        }

        self.current_wp += 1;
        if self.current_wp >= self.waypoints.len() {
            self.state = MissionState::Completed;
            info!("Mission complete: all waypoints visited");
        }
    }

    /// Generate a lawn-mower deseeding pattern
    pub fn generate_deseed_pattern(
        &mut self,
        start_x: f32,
        start_y: f32,
        width: f32,
        length: f32,
        row_spacing: f32,
    ) {
        self.clear_mission();

        let num_rows = (width / row_spacing) as usize + 1;
        let speed = 0.3 / 3.6; // 0.3 km/h deseeding speed

        for i in 0..num_rows {
            let x = start_x + i as f32 * row_spacing;

            if i % 2 == 0 {
                // Forward pass
                self.add_waypoint(x, start_y, 2, speed); // Deseed action
                self.add_waypoint(x, start_y + length, 1, speed); // Stop at end
            } else {
                // Return pass
                self.add_waypoint(x, start_y + length, 2, speed);
                self.add_waypoint(x, start_y, 1, speed);
            }
        }

        info!(
            "Generated deseed pattern: {} rows, {}m × {}m",
            num_rows, width, length
        );
    }
}

/// Normalize angle to [-π, π]
fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle;
    while a > core::f32::consts::PI {
        a -= 2.0 * core::f32::consts::PI;
    }
    while a < -core::f32::consts::PI {
        a += 2.0 * core::f32::consts::PI;
    }
    a
}

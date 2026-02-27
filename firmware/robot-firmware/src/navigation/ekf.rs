//! Extended Kalman Filter for Navigation Sensor Fusion
//!
//! State vector: [x, y, θ, v, ω]
//! - x, y: position in local frame (meters)
//! - θ: heading (radians)
//! - v: forward velocity (m/s)
//! - ω: angular velocity (rad/s)
//!
//! Prediction: IMU + Wheel odometry (100 Hz)
//! Update: NavIC/GPS position (10 Hz)

use libm::{cosf, sinf, sqrtf};

/// State vector dimension
const N: usize = 5;

/// Simple matrix type for EKF computations (stack-allocated)
#[derive(Clone)]
pub struct Matrix<const ROWS: usize, const COLS: usize> {
    pub data: [[f32; COLS]; ROWS],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    pub fn zeros() -> Self {
        Self {
            data: [[0.0; C]; R],
        }
    }

    pub fn identity() -> Self
    where
        [(); R]: Sized,
        [(); C]: Sized,
    {
        let mut m = Self::zeros();
        let n = if R < C { R } else { C };
        for i in 0..n {
            m.data[i][i] = 1.0;
        }
        m
    }
}

/// Extended Kalman Filter
pub struct Ekf {
    /// State estimate: [x, y, θ, v, ω]
    pub state: [f32; N],
    /// Error covariance matrix (5×5)
    pub p: Matrix<N, N>,
    /// Process noise covariance
    pub q: Matrix<N, N>,
    /// GPS measurement noise covariance (2×2 for x, y)
    pub r_gps: Matrix<2, 2>,
    /// IMU measurement noise
    pub r_imu: Matrix<2, 2>,
    /// Time step
    dt: f32,
    /// Filter initialized flag
    initialized: bool,
}

impl Ekf {
    pub fn new(dt: f32) -> Self {
        // Initial error covariance (high uncertainty)
        let mut p = Matrix::<N, N>::identity();
        p.data[0][0] = 100.0; // x uncertainty: 10m
        p.data[1][1] = 100.0; // y uncertainty: 10m
        p.data[2][2] = 1.0; // θ uncertainty: 1 rad
        p.data[3][3] = 1.0; // v uncertainty: 1 m/s
        p.data[4][4] = 0.1; // ω uncertainty: 0.32 rad/s

        // Process noise (tuned for agricultural robot)
        let mut q = Matrix::<N, N>::zeros();
        q.data[0][0] = 0.01; // x process noise
        q.data[1][1] = 0.01; // y process noise
        q.data[2][2] = 0.001; // θ process noise
        q.data[3][3] = 0.1; // v process noise
        q.data[4][4] = 0.05; // ω process noise

        // GPS measurement noise
        let mut r_gps = Matrix::<2, 2>::zeros();
        r_gps.data[0][0] = 2.5 * 2.5; // GPS x noise: 2.5m σ
        r_gps.data[1][1] = 2.5 * 2.5; // GPS y noise: 2.5m σ

        // IMU measurement noise
        let mut r_imu = Matrix::<2, 2>::zeros();
        r_imu.data[0][0] = 0.1; // heading noise
        r_imu.data[1][1] = 0.01; // angular rate noise

        Self {
            state: [0.0; N],
            p,
            q,
            r_gps,
            r_imu,
            dt,
            initialized: false,
        }
    }

    /// Initialize filter with first GPS fix
    pub fn initialize(&mut self, x: f32, y: f32, theta: f32) {
        self.state[0] = x;
        self.state[1] = y;
        self.state[2] = theta;
        self.state[3] = 0.0;
        self.state[4] = 0.0;
        self.initialized = true;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Update GPS noise based on HDOP
    pub fn set_gps_hdop(&mut self, hdop: f32) {
        let noise = hdop * 1.5; // Scale factor for position noise
        self.r_gps.data[0][0] = noise * noise;
        self.r_gps.data[1][1] = noise * noise;
    }

    /// Prediction step using motion model
    /// Inputs: velocity from odometry, angular rate from IMU
    pub fn predict(&mut self, velocity: f32, omega: f32) {
        if !self.initialized {
            return;
        }

        let theta = self.state[2];
        let dt = self.dt;

        // State prediction (constant velocity model)
        // x' = x + v × cos(θ) × dt
        // y' = y + v × sin(θ) × dt
        // θ' = θ + ω × dt
        // v' = v (from odometry)
        // ω' = ω (from IMU)
        self.state[0] += velocity * cosf(theta) * dt;
        self.state[1] += velocity * sinf(theta) * dt;
        self.state[2] += omega * dt;
        self.state[3] = velocity;
        self.state[4] = omega;

        // Normalize heading to [-π, π]
        self.state[2] = normalize_angle(self.state[2]);

        // Jacobian of state transition (F matrix)
        // F = ∂f/∂x evaluated at current state
        let mut f = Matrix::<N, N>::identity();
        f.data[0][2] = -velocity * sinf(theta) * dt; // ∂x/∂θ
        f.data[0][3] = cosf(theta) * dt; // ∂x/∂v
        f.data[1][2] = velocity * cosf(theta) * dt; // ∂y/∂θ
        f.data[1][3] = sinf(theta) * dt; // ∂y/∂v
        f.data[2][4] = dt; // ∂θ/∂ω

        // P' = F × P × F^T + Q
        let fp = mat_mul_5x5(&f, &self.p);
        let ft = transpose_5x5(&f);
        let fpft = mat_mul_5x5(&fp, &ft);

        for i in 0..N {
            for j in 0..N {
                self.p.data[i][j] = fpft.data[i][j] + self.q.data[i][j];
            }
        }
    }

    /// GPS update step
    /// Inputs: measured x, y in local frame from NavIC/GPS
    pub fn update_gps(&mut self, gps_x: f32, gps_y: f32) {
        if !self.initialized {
            return;
        }

        // Measurement matrix H (2×5): we observe x and y directly
        // H = [[1, 0, 0, 0, 0],
        //      [0, 1, 0, 0, 0]]

        // Innovation: z - H×x
        let innovation_x = gps_x - self.state[0];
        let innovation_y = gps_y - self.state[1];

        // Innovation covariance: S = H×P×H^T + R
        let s00 = self.p.data[0][0] + self.r_gps.data[0][0];
        let s01 = self.p.data[0][1];
        let s10 = self.p.data[1][0];
        let s11 = self.p.data[1][1] + self.r_gps.data[1][1];

        // Kalman gain: K = P×H^T×S^-1
        // S^-1 for 2×2 matrix
        let det = s00 * s11 - s01 * s10;
        if libm::fabsf(det) < 1e-10 {
            return; // Singular matrix, skip update
        }
        let inv_det = 1.0 / det;
        let si00 = s11 * inv_det;
        let si01 = -s01 * inv_det;
        let si10 = -s10 * inv_det;
        let si11 = s00 * inv_det;

        // K = P × H^T × S^-1 (5×2 matrix)
        // Since H only selects first two rows, K simplifies to:
        let mut k = [[0.0f32; 2]; N];
        for i in 0..N {
            k[i][0] = self.p.data[i][0] * si00 + self.p.data[i][1] * si10;
            k[i][1] = self.p.data[i][0] * si01 + self.p.data[i][1] * si11;
        }

        // State update: x = x + K × innovation
        for i in 0..N {
            self.state[i] += k[i][0] * innovation_x + k[i][1] * innovation_y;
        }

        // Normalize heading
        self.state[2] = normalize_angle(self.state[2]);

        // Covariance update: P = (I - K×H) × P
        let mut kh = Matrix::<N, N>::zeros();
        for i in 0..N {
            kh.data[i][0] = k[i][0]; // K × H only affects first two columns
            kh.data[i][1] = k[i][1];
        }

        let ikh = {
            let mut m = Matrix::<N, N>::identity();
            for i in 0..N {
                for j in 0..N {
                    m.data[i][j] -= kh.data[i][j];
                }
            }
            m
        };

        let new_p = mat_mul_5x5(&ikh, &self.p);
        self.p = new_p;
    }

    /// Get current position estimate
    pub fn position(&self) -> (f32, f32) {
        (self.state[0], self.state[1])
    }

    /// Get current heading estimate (radians)
    pub fn heading(&self) -> f32 {
        self.state[2]
    }

    /// Get current velocity estimate (m/s)
    pub fn velocity(&self) -> f32 {
        self.state[3]
    }

    /// Get position uncertainty (1σ in meters)
    pub fn position_uncertainty(&self) -> f32 {
        sqrtf(self.p.data[0][0] + self.p.data[1][1])
    }
}

// ─── Matrix helper functions ─────────────────────────────────────────

fn mat_mul_5x5(a: &Matrix<N, N>, b: &Matrix<N, N>) -> Matrix<N, N> {
    let mut result = Matrix::<N, N>::zeros();
    for i in 0..N {
        for j in 0..N {
            let mut sum = 0.0f32;
            for k in 0..N {
                sum += a.data[i][k] * b.data[k][j];
            }
            result.data[i][j] = sum;
        }
    }
    result
}

fn transpose_5x5(m: &Matrix<N, N>) -> Matrix<N, N> {
    let mut result = Matrix::<N, N>::zeros();
    for i in 0..N {
        for j in 0..N {
            result.data[i][j] = m.data[j][i];
        }
    }
    result
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

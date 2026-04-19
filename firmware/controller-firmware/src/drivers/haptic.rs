//! DRV2605L Haptic Motor Driver
//!
//! TI DRV2605L via I2C for vibration feedback on controller.
//! Supports LRA (Linear Resonant Actuator) mode with built-in effects library.

use defmt::*;
use embedded_hal::i2c::I2c;

/// DRV2605L I2C address
pub const DRV2605L_ADDR: u8 = 0x5A;

/// DRV2605L registers
pub mod reg {
    pub const STATUS: u8 = 0x00;
    pub const MODE: u8 = 0x01;
    pub const RTP_INPUT: u8 = 0x02;
    pub const LIBRARY_SEL: u8 = 0x03;
    pub const WAVEFORM_SEQ: u8 = 0x04; // 0x04-0x0B (8 slots)
    pub const GO: u8 = 0x0C;
    pub const OVERDRIVE_OFFSET: u8 = 0x0D;
    pub const SUSTAIN_POS_OFFSET: u8 = 0x0E;
    pub const SUSTAIN_NEG_OFFSET: u8 = 0x0F;
    pub const BRAKE_OFFSET: u8 = 0x10;
    pub const AUDIO_CTRL: u8 = 0x11;
    pub const AUDIO_MIN_INPUT: u8 = 0x12;
    pub const AUDIO_MAX_INPUT: u8 = 0x13;
    pub const AUDIO_MIN_OUTPUT: u8 = 0x14;
    pub const AUDIO_MAX_OUTPUT: u8 = 0x15;
    pub const RATED_VOLTAGE: u8 = 0x16;
    pub const OD_CLAMP: u8 = 0x17;
    pub const A_CAL_COMP: u8 = 0x18;
    pub const A_CAL_BEMF: u8 = 0x19;
    pub const FEEDBACK_CTRL: u8 = 0x1A;
    pub const CTRL1: u8 = 0x1B;
    pub const CTRL2: u8 = 0x1C;
    pub const CTRL3: u8 = 0x1D;
    pub const CTRL4: u8 = 0x1E;
    pub const CTRL5: u8 = 0x1F;
    pub const LRA_OPEN_LOOP: u8 = 0x20;
    pub const VBAT: u8 = 0x21;
    pub const LRA_PERIOD: u8 = 0x22;
}

/// Haptic effect library (DRV2605L built-in effects)
/// Selected effects useful for controller feedback
#[derive(Debug, Clone, Copy, defmt::Format)]
#[repr(u8)]
pub enum HapticEffect {
    /// Strong click - button press confirmation
    StrongClick100 = 1,
    /// Strong click - 60% - lighter button feedback
    StrongClick60 = 2,
    /// Sharp click - mode change
    SharpClick100 = 4,
    /// Soft bump - waypoint reached
    SoftBump100 = 7,
    /// Double click - acknowledgment
    DoubleClick100 = 10,
    /// Triple click - error/warning
    TripleClick100 = 12,
    /// Soft fuzz - continuous feedback
    SoftFuzz60 = 14,
    /// Strong buzz - alarm
    StrongBuzz100 = 15,
    /// Alert 750ms - E-STOP feedback
    Alert750ms = 16,
    /// Alert 1000ms - critical warning
    Alert1000ms = 17,
    /// Strong pulse - obstacle detected
    StrongPulse100 = 19,
    /// Medium pulse - connection event
    MediumPulse100 = 20,
    /// Sharp transition click up
    SharpTransitionUp = 28,
    /// Sharp transition click down
    SharpTransitionDown = 32,
    /// Long double tap - mission complete
    LongDoubleSharpTick = 37,
    /// Buzz (1-5 are different intensities)
    Buzz1 = 47,
    /// Pulsing strong
    PulsingStrong1 = 52,
    /// Pulsing medium
    PulsingMedium1 = 58,
    /// Transition hum
    TransitionHum1 = 64,
    /// Smooth hum - joystick haptic guide
    SmoothHum1 = 70,
}

/// Haptic feedback patterns for controller events
#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum HapticPattern {
    /// Button press acknowledgment
    ButtonPress,
    /// Mode switch confirmation
    ModeSwitch,
    /// Waypoint reached
    WaypointReached,
    /// Obstacle warning (repeated)
    ObstacleWarning,
    /// Emergency stop activated
    EmergencyStop,
    /// Link lost warning
    LinkLost,
    /// Mission complete celebration
    MissionComplete,
    /// Error notification
    Error,
    /// Geofence warning
    GeofenceWarning,
    /// Low battery alert
    LowBattery,
}

impl HapticPattern {
    /// Get the effect sequence for this pattern
    /// Returns up to 4 effects (DRV2605L supports 8 slots)
    pub fn effects(&self) -> &[HapticEffect] {
        match self {
            Self::ButtonPress => &[HapticEffect::SharpClick100],
            Self::ModeSwitch => &[HapticEffect::DoubleClick100],
            Self::WaypointReached => &[HapticEffect::SoftBump100, HapticEffect::SoftBump100],
            Self::ObstacleWarning => &[
                HapticEffect::StrongBuzz100,
                HapticEffect::StrongBuzz100,
            ],
            Self::EmergencyStop => &[HapticEffect::Alert1000ms],
            Self::LinkLost => &[
                HapticEffect::TripleClick100,
                HapticEffect::Alert750ms,
            ],
            Self::MissionComplete => &[
                HapticEffect::LongDoubleSharpTick,
                HapticEffect::SoftBump100,
            ],
            Self::Error => &[HapticEffect::TripleClick100],
            Self::GeofenceWarning => &[
                HapticEffect::StrongPulse100,
                HapticEffect::StrongPulse100,
            ],
            Self::LowBattery => &[HapticEffect::PulsingMedium1],
        }
    }
}

/// I2C driver for the TI DRV2605L haptic motor controller.
///
/// Configured for **LRA (Linear Resonant Actuator) open-loop internal-trigger**
/// mode, which does not require auto-calibration and works immediately after
/// `init()`.  Call `play_pattern()` with any [`HapticPattern`] to trigger
/// tactile feedback.
///
/// The generic `I2C` parameter must implement `embedded_hal::i2c::I2c`
/// (the blocking variant).  On the ESP32-S3 controller this will typically
/// be `esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>`.
///
/// # Example
/// ```rust
/// let mut haptic = Drv2605l::new(i2c_bus);
/// haptic.init().ok();
/// haptic.play_pattern(HapticPattern::ButtonPress).ok();
/// ```
pub struct Drv2605l<I2C> {
    i2c: I2C,
}

impl<I2C: I2c> Drv2605l<I2C> {
    /// Create a new driver wrapping `i2c`.
    ///
    /// `I2C` must be configured with the DRV2605L I2C address already
    /// set (address 0x5A).  Call `init()` before using any other method.
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    // ── Private I2C helpers ───────────────────────────────────────────

    fn write_reg(&mut self, register: u8, value: u8) -> Result<(), I2C::Error> {
        self.i2c.write(DRV2605L_ADDR, &[register, value])
    }

    fn read_reg(&mut self, register: u8) -> Result<u8, I2C::Error> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(DRV2605L_ADDR, &[register], &mut buf)?;
        Ok(buf[0])
    }

    // ── Public API ────────────────────────────────────────────────────

    /// Initialize the DRV2605L for LRA open-loop internal-trigger mode.
    ///
    /// Programs all necessary control registers.  No auto-calibration is
    /// performed, so the call returns quickly (no waiting required).
    ///
    /// Must be called once before `play_pattern()` or `stop()`.
    pub fn init(&mut self) -> Result<(), I2C::Error> {
        // Clear any fault state and enter standby
        self.write_reg(reg::MODE, 0x00)?;

        // Select LRA waveform library (library 6 = LRA)
        self.write_reg(reg::LIBRARY_SEL, 0x06)?;

        // Feedback control: N_ERM_LRA=1 (LRA mode), FB_BRAKE_FACTOR=3, LOOP_GAIN=1
        // 0xB6 = 1011_0110
        self.write_reg(reg::FEEDBACK_CTRL, 0xB6)?;

        // CTRL1: STARTUP_BOOST=1, drive time ≈ 2 ms for a ~150 Hz LRA
        // 0x13 = 0001_0011
        self.write_reg(reg::CTRL1, 0x13)?;

        // CTRL2: BIDIR_INPUT=1, BRAKE_STABILIZER=1, SAMPLE_TIME=3,
        //        BLANKING_TIME=1, IDISS_TIME=1
        // 0xF5 = 1111_0101
        self.write_reg(reg::CTRL2, 0xF5)?;

        // CTRL3: LRA_OPEN_LOOP=1 (skip auto-cal), NG_THRESH=1
        // 0xA1 = 1010_0001
        self.write_reg(reg::CTRL3, 0xA1)?;

        // CTRL4: AUTO_CAL_TIME=3 (1000 ms window — unused in open-loop)
        // 0x20 = 0010_0000
        self.write_reg(reg::CTRL4, 0x20)?;

        // CTRL5: LRA_AUTO_OPEN_LOOP=1, PLAYBACK_INTERVAL=1
        // 0x80 = 1000_0000
        self.write_reg(reg::CTRL5, 0x80)?;

        // Rated voltage: ~2.0 V RMS for a typical 150 Hz LRA
        self.write_reg(reg::RATED_VOLTAGE, 0x3E)?;

        // Overdrive clamp: ~3.2 V
        self.write_reg(reg::OD_CLAMP, 0x8C)?;

        // Internal-trigger playback mode (MODE = 0x00)
        self.write_reg(reg::MODE, 0x00)?;

        info!("DRV2605L: initialized (LRA open-loop internal-trigger)");
        Ok(())
    }

    /// Play a haptic feedback pattern.
    ///
    /// Loads the effect IDs returned by [`HapticPattern::effects()`] into
    /// waveform sequencer slots 0x04–0x0B (up to 8 effects) and sets the
    /// GO bit to trigger immediate playback.
    ///
    /// The DRV2605L plays through the sequence autonomously; you do not
    /// need to wait for completion before calling again.
    pub fn play_pattern(&mut self, pattern: HapticPattern) -> Result<(), I2C::Error> {
        let effects = pattern.effects();

        // Ensure internal-trigger mode is active
        self.write_reg(reg::MODE, 0x00)?;

        // Load effect IDs into waveform sequencer slots (max 8)
        let n = effects.len().min(8);
        for (i, effect) in effects[..n].iter().enumerate() {
            self.write_reg(reg::WAVEFORM_SEQ + i as u8, *effect as u8)?;
        }
        // Terminate the sequence with 0x00 if fewer than 8 slots are used
        if n < 8 {
            self.write_reg(reg::WAVEFORM_SEQ + n as u8, 0x00)?;
        }

        // Trigger playback
        self.write_reg(reg::GO, 0x01)?;

        Ok(())
    }

    /// Stop any currently playing haptic pattern immediately.
    pub fn stop(&mut self) -> Result<(), I2C::Error> {
        self.write_reg(reg::GO, 0x00)
    }

    /// Read the STATUS register.  Bit 3 (DIAG_RESULT) is `1` on a device fault.
    pub fn status(&mut self) -> Result<u8, I2C::Error> {
        self.read_reg(reg::STATUS)
    }
}

//! DRV2605L Haptic Motor Driver
//!
//! TI DRV2605L via I2C for vibration feedback on controller.
//! Supports LRA (Linear Resonant Actuator) mode with built-in effects library.

use defmt::*;

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

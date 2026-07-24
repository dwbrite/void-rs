//! Device-level feel tuning. One global set: a tap should feel like a tap
//! everywhere in the game. Per-action knobs (buffer windows, thresholds)
//! live on bindings and the ActionMap instead.

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct DetectorConfig {
    /// Radial deadzone applied to sticks before anything else.
    pub stick_deadzone: f32,
    /// Stick must reach this magnitude for a deflection to count as a tap.
    pub tap_min_displacement: f32,
    /// Deflections held longer than this are holds, not taps (seconds).
    pub tap_max_hold: f32,
    /// Stick must return inside this radius to complete the tap.
    pub tap_release_radius: f32,
    /// Rolls only register while the stick is outside this radius.
    pub roll_min_radius: f32,
    /// Angular speed that maps to a roll signal value of 1.0.
    pub roll_max_rpm: f32,
    /// Time constant (seconds) for smoothing the roll velocity estimate.
    pub roll_smoothing: f32,

    pub flick_rest_radius: f32,        // 0.15 — "the stick has started moving"; just above deadzone
    pub flick_rise_time: f32,          // 0.05 — ≈3 ticks; rise faster than this = "fast"
    pub spring_min_hold: f32,          // = tap_max_hold; shorter holds can't be a deflect-release
    pub spring_max_release_time: f32,  // 0.066 — edge → center within ~4 ticks counts as a "spring"

    /// Ticks after edge-arrival during which a TapDeflect can be cancelled by
    /// rotation. 0.0 restores zero-latency taps (and roll-starts fire attacks).
    pub tap_roll_grace: f32,        // 0.03 — ~4 ticks @128Hz
    /// Rotation accumulated at the edge within the grace window that reclassifies
    /// the deflection as a roll-start.
    pub tap_cancel_rotation: f32,   // TAU / 12.0 — 30°
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            stick_deadzone: 0.15,
            tap_min_displacement: 0.70,
            tap_max_hold: 0.20,
            tap_release_radius: 0.3,
            roll_min_radius: 0.9,
            roll_max_rpm: 1000.0,
            roll_smoothing: 0.05,
            flick_rest_radius: 0.25,
            flick_rise_time: 0.025,
            spring_min_hold: 0.5, // TODO: rename this shit
            spring_max_release_time: 0.066,
            tap_roll_grace: 0.03,
            tap_cancel_rotation: std::f32::consts::TAU / 9.0,  // 360/6 = 40
        }
    }
}

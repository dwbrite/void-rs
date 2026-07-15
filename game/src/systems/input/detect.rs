//! The hardware-facing layer. Reads Bevy's `Gamepad`, deadzones sticks once,
//! runs the stateful tap/roll detectors, and publishes everything as
//! `Signal`s keyed by `SignalId`. Nothing downstream touches hardware.

use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

use super::config::DetectorConfig;
use super::signal::{RollDir, Signal, SignalId, Stick};

const BUTTONS: &[GamepadButton] = &[
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::North,
    GamepadButton::West,
    GamepadButton::LeftTrigger,
    GamepadButton::LeftTrigger2,
    GamepadButton::RightTrigger,
    GamepadButton::RightTrigger2,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::Mode,
    GamepadButton::LeftThumb,
    GamepadButton::RightThumb,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];

// ---------------------------------------------------------------------------
// Per-pad state
// ---------------------------------------------------------------------------

#[derive(Default)]
enum TapState {
    #[default]
    Idle,
    Deflected {
        start: f32,
        octant: u8,
    },
}

#[derive(Default)]
struct StickDetector {
    tap: TapState,
    last_angle: Option<f32>,
    /// Smoothed angular velocity in rad/s. Positive = CCW.
    omega: f32,
}

#[derive(Default)]
pub struct PadSignals {
    pub signals: HashMap<SignalId, Signal>,
    left: StickDetector,
    right: StickDetector,
}

impl PadSignals {
    fn push(&mut self, id: SignalId, v: f32) {
        self.signals.entry(id).or_default().push(v);
    }
    pub fn get(&self, id: SignalId) -> Signal {
        self.signals.get(&id).copied().unwrap_or_default()
    }
}

/// All signals, per connected gamepad entity.
#[derive(Resource, Default)]
pub struct InputSignals {
    pub pads: HashMap<Entity, PadSignals>,
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

fn radial_deadzone(v: Vec2, dz: f32) -> Vec2 {
    let mag = v.length();
    if mag <= dz {
        Vec2::ZERO
    } else {
        // Renormalize so output magnitude spans 0..1 smoothly past the deadzone.
        v / mag * ((mag - dz) / (1.0 - dz)).min(1.0)
    }
}

fn octant_of(angle: f32) -> u8 {
    ((angle / (TAU / 8.0)).round() as i32).rem_euclid(8) as u8
}

fn wrap_angle(a: f32) -> f32 {
    let mut a = a % TAU;
    if a > TAU / 2.0 {
        a -= TAU;
    } else if a < -TAU / 2.0 {
        a += TAU;
    }
    a
}

// ---------------------------------------------------------------------------
// Stick detectors (taps + rolls)
// ---------------------------------------------------------------------------

fn tick_stick(
    det: &mut StickDetector,
    stick: Stick,
    v: Vec2,
    now: f32,
    dt: f32,
    cfg: &DetectorConfig,
    out: &mut Vec<(SignalId, f32)>,
) {
    let mag = v.length();
    let angle = v.y.atan2(v.x);

    // --- Tap: default all 8 octant pulses to 0, override on fire ---
    let mut fired: Option<u8> = None;
    det.tap = match std::mem::take(&mut det.tap) {
        TapState::Idle => {
            if mag >= cfg.tap_min_displacement {
                // Octant locks at first crossing: free hysteresis.
                TapState::Deflected { start: now, octant: octant_of(angle) }
            } else {
                TapState::Idle
            }
        }
        TapState::Deflected { start, octant } => {
            if now - start > cfg.tap_max_hold {
                TapState::Idle // held too long; it's a hold, not a tap
            } else if mag <= cfg.tap_release_radius {
                fired = Some(octant);
                TapState::Idle
            } else {
                TapState::Deflected { start, octant }
            }
        }
    };
    for oct in 0..8u8 {
        let v = if fired == Some(oct) { 1.0 } else { 0.0 };
        out.push((SignalId::OctantTap { stick, octant: oct }, v));
    }

    // --- Roll: smoothed angular velocity, normalized to max_rpm ---
    if mag >= cfg.roll_min_radius {
        if let Some(last) = det.last_angle {
            let raw_omega = if dt > 0.0 { wrap_angle(angle - last) / dt } else { 0.0 };
            let alpha = dt / (dt + cfg.roll_smoothing);
            det.omega += (raw_omega - det.omega) * alpha;
        }
        det.last_angle = Some(angle);
    } else {
        det.last_angle = None;
        let alpha = dt / (dt + cfg.roll_smoothing);
        det.omega += (0.0 - det.omega) * alpha;
    }
    let rpm = det.omega / TAU * 60.0; // +CCW, -CW
    let max = cfg.roll_max_rpm.max(1.0);
    out.push((SignalId::Roll { stick, dir: RollDir::Ccw }, (rpm / max).clamp(0.0, 1.0)));
    out.push((SignalId::Roll { stick, dir: RollDir::Cw }, (-rpm / max).clamp(0.0, 1.0)));
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

pub(crate) fn tick_signals(
    time: Res<Time>,
    cfg: Res<DetectorConfig>,
    pads: Query<(Entity, &Gamepad)>,
    mut sig: ResMut<InputSignals>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();

    // Drop state for disconnected pads.
    sig.pads.retain(|e, _| pads.contains(*e));

    for (entity, pad) in &pads {
        let ps = sig.pads.entry(entity).or_default();

        // Buttons (analog where available: triggers report 0..1).
        for &b in BUTTONS {
            let v = pad.get(b).unwrap_or(if pad.pressed(b) { 1.0 } else { 0.0 });
            ps.push(SignalId::Button(b), v);
        }

        // Sticks: deadzone once, publish axes + magnitude, run detectors.
        let mut staged: Vec<(SignalId, f32)> = Vec::with_capacity(24);
        for (stick, raw, ax, ay) in [
            (Stick::Left, pad.left_stick(), GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
            (Stick::Right, pad.right_stick(), GamepadAxis::RightStickX, GamepadAxis::RightStickY),
        ] {
            let v = radial_deadzone(raw, cfg.stick_deadzone);
            staged.push((SignalId::Axis(ax), v.x));
            staged.push((SignalId::Axis(ay), v.y));
            staged.push((SignalId::StickMagnitude(stick), v.length().min(1.0)));
            let det = match stick {
                Stick::Left => &mut ps.left,
                Stick::Right => &mut ps.right,
            };
            tick_stick(det, stick, v, now, dt, &cfg, &mut staged);
        }
        for (id, v) in staged {
            ps.push(id, v);
        }
    }
}

//! The hardware-facing layer. Reads Bevy's `Gamepad`, deadzones sticks once,
//! runs the stateful tap/roll detectors, and publishes everything as
//! `Signal`s keyed by `SignalId`. Nothing downstream touches hardware.

use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

use super::config::DetectorConfig;
use super::signal::{Octant, RollDir, Signal, SignalId, Stick};

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

// ---------------------------------------------------------------------------
// Flick detection
// ---------------------------------------------------------------------------

#[derive(Default)]
enum FlickState {
    #[default]
    Idle,
    /// Left rest, hasn't hit full deflection yet. Timing the rise.
    Rising { start: f32 },
    Deflected {
        /// When we crossed `tap_min_displacement`.
        crossed: f32,
        /// Octant locked at edge-arrival — hysteresis for tap gestures.
        locked_octant: u8,
        /// Octant tracked during the hold — long holds may rotate, and
        /// deflect-release should report where the stick *was*, not where
        /// it entered.
        cur_octant: u8,
        /// Rise time was <= flick_rise_time.
        fast: bool,
        /// When mag last dipped below the edge (springback timing).
        /// None while pinned at the edge.
        left_edge: Option<f32>,
    },
}

#[derive(Default)]
struct StickDetector {
    flick: FlickState,
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

    // --- Flicks: three gestures from one state machine ---
    //
    // NOTE for binding-capture UI: one physical gesture can fire multiple
    // signals (fast flick+release fires TapDeflect *and* TapRelease). Bindings
    // filter to one signal so this is fine in play, but a "press input to
    // bind" listener should collect candidates for ~tap_max_hold after the
    // first fire and let the player disambiguate. Don't "fix" it here.
    let mut tap_deflect: Option<u8> = None;
    let mut tap_release: Option<u8> = None;
    let mut deflect_release: Option<u8> = None;

    det.flick = match std::mem::take(&mut det.flick) {
        FlickState::Idle => {
            if mag >= cfg.tap_min_displacement {
                // 0 -> edge in a single tick: trivially fast.
                let oct = octant_of(angle);
                tap_deflect = Some(oct); // (1) fires on arrival, zero latency
                FlickState::Deflected {
                    crossed: now,
                    locked_octant: oct,
                    cur_octant: oct,
                    fast: true,
                    left_edge: None,
                }
            } else if mag > cfg.flick_rest_radius {
                FlickState::Rising { start: now }
            } else {
                FlickState::Idle
            }
        }

        FlickState::Rising { start } => {
            if mag >= cfg.tap_min_displacement {
                let fast = now - start <= cfg.flick_rise_time;
                let oct = octant_of(angle);
                if fast {
                    tap_deflect = Some(oct); // (1) fires on arrival
                }
                FlickState::Deflected {
                    crossed: now,
                    locked_octant: oct,
                    cur_octant: oct,
                    fast,
                    left_edge: None,
                }
            } else if mag <= cfg.flick_rest_radius {
                FlickState::Idle // wiggled and gave up; never reached the edge
            } else {
                FlickState::Rising { start }
            }
        }

        FlickState::Deflected { crossed, locked_octant, mut cur_octant, fast, mut left_edge } => {
            let held = now - crossed;

            if mag <= cfg.tap_release_radius {
                // Back at center: classify the exit.
                if fast && held <= cfg.tap_max_hold {
                    tap_release = Some(locked_octant); // (2)
                } else if held >= cfg.spring_min_hold
                    && left_edge.map_or(true, |t| now - t <= cfg.spring_max_release_time)
                {
                    deflect_release = Some(cur_octant); // (3)
                }
                // else: slow ease back to neutral — no gesture, on purpose.
                FlickState::Idle
            } else {
                if mag >= cfg.tap_min_displacement {
                    left_edge = None;               // (re)pinned to the edge
                    cur_octant = octant_of(angle);  // track rotation during hold
                } else if left_edge.is_none() {
                    left_edge = Some(now);          // started coming off the edge
                }

                FlickState::Deflected { crossed, locked_octant, cur_octant, fast, left_edge }
            }
        }
    };

    for oct in 0..8u8 {
        let o = Octant::from_oct(oct);
        out.push((SignalId::TapDeflect     { stick, octant: o }, (tap_deflect     == Some(oct)) as u8 as f32));
        out.push((SignalId::TapRelease     { stick, octant: o }, (tap_release     == Some(oct)) as u8 as f32));
        out.push((SignalId::DeflectRelease { stick, octant: o }, (deflect_release == Some(oct)) as u8 as f32));
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

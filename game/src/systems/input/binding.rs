//! The "anything to anything" glue: a binding points at one signal on one
//! device and pipes it through composable transforms. This is the only place
//! where hardware-side signals and game-side actions meet.

use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;

use super::detect::{InputSignals, PadSignals};
use super::signal::{Octant, RollDir, SignalId, Stick};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DeviceFilter {
    /// Read from every connected pad, take the strongest value.
    /// Use for menu navigation; don't expose to gameplay rebinding.
    #[default]
    Any,
    /// Read from one specific gamepad entity.
    Pad(Entity),
}

#[derive(Clone, Copy, Debug)]
pub enum Processor {
    Abs,
    Negate,
    /// 0.0 below the threshold, 1.0 at or above it.
    Threshold(f32),
    /// Zero out small values (per-binding, on top of the stick deadzone).
    Deadzone(f32),
    Remap { from: (f32, f32), to: (f32, f32) },
}

impl Processor {
    fn apply(self, v: f32) -> f32 {
        match self {
            Processor::Abs => v.abs(),
            Processor::Negate => -v,
            Processor::Threshold(t) => {
                if v >= t {
                    1.0
                } else {
                    0.0
                }
            }
            Processor::Deadzone(d) => {
                if v.abs() <= d {
                    0.0
                } else {
                    v
                }
            }
            Processor::Remap { from, to } => {
                let t = if (from.1 - from.0).abs() > f32::EPSILON {
                    (v - from.0) / (from.1 - from.0)
                } else {
                    0.0
                };
                to.0 + t.clamp(0.0, 1.0) * (to.1 - to.0)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub device: DeviceFilter,
    pub signal: SignalId,
    pub processors: Vec<Processor>,
}

impl Binding {
    pub fn new(signal: SignalId) -> Self {
        Self { device: DeviceFilter::Any, signal, processors: Vec::new() }
    }
    pub fn button(b: GamepadButton) -> Self {
        Self::new(SignalId::Button(b))
    }
    pub fn axis(a: GamepadAxis) -> Self {
        Self::new(SignalId::Axis(a))
    }
    pub fn tap(stick: Stick, octant: Octant) -> Self {
        Self::new(SignalId::OctantTap { stick, octant })
    }
    pub fn roll(stick: Stick, dir: RollDir) -> Self {
        Self::new(SignalId::Roll { stick, dir })
    }
    pub fn on(mut self, device: DeviceFilter) -> Self {
        self.device = device;
        self
    }
    pub fn with(mut self, p: Processor) -> Self {
        self.processors.push(p);
        self
    }

    pub(crate) fn eval(&self, sig: &InputSignals) -> f32 {
        let read = |ps: &PadSignals| ps.get(self.signal).value;
        let raw = match self.device {
            DeviceFilter::Pad(e) => sig.pads.get(&e).map(read).unwrap_or(0.0),
            DeviceFilter::Any => sig
                .pads
                .values()
                .map(read)
                .fold(0.0, |acc: f32, v| if v.abs() > acc.abs() { v } else { acc }),
        };
        self.processors.iter().fold(raw, |v, p| p.apply(v))
    }
}

//! The universal currency: every input is a float over time, identified by a
//! `SignalId`. Binary/edge queries are derived reads, never separate types.

use bevy::input::gamepad::{GamepadAxis, GamepadButton};

#[derive(Clone, Copy, Default, Debug)]
pub struct Signal {
    pub value: f32,
    pub prev: f32,
}

impl Signal {
    pub(crate) fn push(&mut self, v: f32) {
        self.prev = self.value;
        self.value = v;
    }
    pub fn binary(&self, threshold: f32) -> bool {
        self.value >= threshold
    }
    pub fn pressed_this_frame(&self, threshold: f32) -> bool {
        self.value >= threshold && self.prev < threshold
    }
    pub fn released_this_frame(&self, threshold: f32) -> bool {
        self.value < threshold && self.prev >= threshold
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stick {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RollDir {
    Cw,
    Ccw,
}

/// Names every readable signal. Because this is a plain value type, a binding
/// can always be backtraced to the physical control (or stick gesture) that
/// feeds it -- rebind menus and "press any input" capture read these directly.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalId {
    /// Analog value of any button (triggers report 0..1, digital buttons 0/1).
    Button(GamepadButton),
    /// Deadzoned, renormalized axis value (-1..1).
    Axis(GamepadAxis),
    /// Deadzoned stick magnitude (0..1).
    StickMagnitude(Stick),
    /// Pulses to 1.0 for one frame when a tap lands in this octant.
    /// Octant 0 is centered on +X (east), counting counter-clockwise.
    OctantTap { stick: Stick, octant: u8 },
    /// Angular speed of a stick roll, normalized: 1.0 == `roll_max_rpm`.
    Roll { stick: Stick, dir: RollDir },
}

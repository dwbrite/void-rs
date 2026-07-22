//! The game-facing layer. `ActionMap` (a.k.a. the PissBucket) holds the
//! action -> bindings table plus per-action state: current value, held flag
//! with hysteresis, and press-edge timestamps for input buffering.
//! Gameplay code only ever talks to this; it never sees SignalIds or pads.

use bevy::prelude::*;
use std::collections::HashMap;
use std::hash::Hash;

use super::binding::Binding;
use super::detect::InputSignals;

pub trait ActionLike: Copy + Eq + Hash + Send + Sync + 'static {}
impl<T: Copy + Eq + Hash + Send + Sync + 'static> ActionLike for T {}

#[derive(Clone, Copy, Default, Debug)]
struct ActionState {
    /// Processed action value after binding processors have been applied.
    value: f32,
    /// Raw action value from the underlying signal before processors.
    raw_value: f32,
    held: bool,
    pressed_this_frame: bool,
    /// Timestamp of the most recent press edge; consumed by buffered reads.
    last_press: Option<f32>,
}

#[derive(Resource)]
pub struct ActionMap<A: ActionLike> {
    /// Press/release use hysteresis so analog sources don't flicker.
    pub press_threshold: f32,
    pub release_threshold: f32,
    bindings: HashMap<A, Vec<Binding>>,
    state: HashMap<A, ActionState>,
    now: f32,
}

impl<A: ActionLike> Default for ActionMap<A> {
    fn default() -> Self {
        Self {
            press_threshold: 0.5,
            release_threshold: 0.4,
            bindings: HashMap::new(),
            state: HashMap::new(),
            now: 0.0,
        }
    }
}

impl<A: ActionLike> ActionMap<A> {
    pub fn bind(&mut self, action: A, binding: Binding) -> &mut Self {
        self.bindings.entry(action).or_default().push(binding);
        self
    }
    pub fn clear_bindings(&mut self, action: A) {
        self.bindings.remove(&action);
    }
    /// The forward table doubles as the backtrace: each Binding carries its
    /// SignalId, so this is all a rebind menu needs to display sources.
    pub fn bindings(&self, action: A) -> &[Binding] {
        self.bindings.get(&action).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Combined analog value across all bindings after processors are applied.
    pub fn value(&self, action: A) -> f32 {
        self.state.get(&action).map(|s| s.value).unwrap_or(0.0)
    }
    /// Combined analog value across all bindings before processors are applied.
    pub fn raw_value(&self, action: A) -> f32 {
        self.state.get(&action).map(|s| s.raw_value).unwrap_or(0.0)
    }
    pub fn is_down(&self, action: A) -> bool {
        self.state.get(&action).map(|s| s.held || s.pressed_this_frame).unwrap_or(false)
    }
    pub fn just_pressed(&self, action: A) -> bool {
        self.state.get(&action).map(|s| s.pressed_this_frame).unwrap_or(false)
    }

    /// True if a press edge happened within the last `window` seconds.
    /// Consumes the press so one buffered input can't trigger twice.
    pub fn buffered_press(&mut self, action: A, window: f32) -> bool {
        let now = self.now;
        if let Some(st) = self.state.get_mut(&action) {
            if let Some(t) = st.last_press {
                if now - t <= window {
                    st.last_press = None;
                    return true;
                }
            }
        }
        false
    }
}

pub(crate) fn evaluate_actions<A: ActionLike>(
    time: Res<Time>,
    sig: Res<InputSignals>,
    mut map: ResMut<ActionMap<A>>,
) {
    let now = time.elapsed_secs();
    map.now = now;
    let press_t = map.press_threshold;
    let release_t = map.release_threshold;

    let ActionMap { bindings, state, .. } = &mut *map;
    for (action, binds) in bindings.iter_mut() {
        let raw_value = binds
            .iter_mut()
            .map(|b| b.eval_raw(&sig))
            .fold(0.0, |acc: f32, v| if v.abs() > acc.abs() { v } else { acc });
        let value = binds
            .iter_mut()
            .map(|b| b.eval(&sig))
            .fold(0.0, |acc: f32, v| if v.abs() > acc.abs() { v } else { acc });

        let st = state.entry(*action).or_default();
        st.raw_value = raw_value;
        st.value = value;
        st.pressed_this_frame = false;
        let mag = value.abs();
        if !st.held && mag >= press_t {
            st.held = true;
            st.pressed_this_frame = true;
            st.last_press = Some(now);
        } else if st.held && mag < release_t {
            st.held = false;
        }
    }
}

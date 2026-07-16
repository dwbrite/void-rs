//! Gamepad -> ProcessedInput signals -> Bindings -> InputActions.
//!
//! Targets Bevy 0.18 (gamepads are entities with a `Gamepad` component).
//!
//! Module map:
//!   signal  - Signal (float + prev) and SignalId (provenance for backtracing)
//!   config  - DetectorConfig (device-level feel tuning)
//!   detect  - hardware-facing: tap/roll detectors, tick_signals system
//!   binding - DeviceFilter, Processor chains, Binding (signal -> action glue)
//!   action  - ActionMap (the PissBucket) + evaluate_actions system

pub mod action;
pub mod binding;
pub mod config;
pub mod detect;
pub mod signal;

pub use action::{ActionLike, ActionMap};
pub use binding::{Binding, DeviceFilter, Processor};
pub use config::DetectorConfig;
pub use detect::{InputSignals, PadSignals};
pub use signal::{RollDir, Signal, SignalId, Stick};

use bevy::prelude::*;
use std::marker::PhantomData;

pub struct GameInputPlugin<A: ActionLike>(PhantomData<A>);

impl<A: ActionLike> Default for GameInputPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: ActionLike> Plugin for GameInputPlugin<A> {
    fn build(&self, app: &mut App) {
        app.init_resource::<DetectorConfig>()
            .init_resource::<InputSignals>()
            .init_resource::<ActionMap<A>>()
            .add_systems(
                PreUpdate,
                (detect::tick_signals, action::evaluate_actions::<A>)
                    .chain()
                    // On Bevy <= 0.16 this set is called `InputSystem`.
                    .after(bevy::input::InputSystems),
            );
    }
}

// ---------------------------------------------------------------------------
// Example usage
// ---------------------------------------------------------------------------

#[allow(dead_code)]
mod example {
    use super::*;
    use bevy::input::gamepad::{GamepadAxis, GamepadButton};
    use crate::systems::input::signal::Octant::North;

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum Action {
        Jump,
        Punch,
        MoveX,
    }

    fn plugin(app: &mut App) {
        app.add_plugins(GameInputPlugin::<Action>::default())
            .add_systems(Startup, setup_bindings)
            .add_systems(Update, player_control);
    }

    fn setup_bindings(mut map: ResMut<ActionMap<Action>>) {
        // Jump: A button, OR a tap in the "up" octant (octant 2 = north),
        // OR (the insane one) the left-right axis past 0.75 either way.
        map.bind(Action::Jump, Binding::button(GamepadButton::South));
        map.bind(Action::Jump, Binding::tap(Stick::Left, North));
        map.bind(
            Action::Jump,
            Binding::axis(GamepadAxis::LeftStickX)
                .with(Processor::Abs)
                .with(Processor::Threshold(0.75)),
        );

        // Punch: clockwise roll on the right stick, needs >= 60% of max rpm.
        map.bind(
            Action::Punch,
            Binding::roll(Stick::Right, RollDir::Cw).with(Processor::Threshold(0.6)),
        );

        // MoveX: plain signed axis.
        map.bind(Action::MoveX, Binding::axis(GamepadAxis::LeftStickX));
    }

    fn player_control(mut map: ResMut<ActionMap<Action>>) {
        let move_x = map.value(Action::MoveX);
        let _ = move_x; // feed into your character controller

        // 120 ms jump buffer; consume-on-read.
        if map.buffered_press(Action::Jump, 0.12) {
            // do the jump
        }

        if map.just_pressed(Action::Punch) {
            // do the punch
        }
    }
}

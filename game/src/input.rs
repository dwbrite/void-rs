use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;

use crate::systems::input::{
    ActionMap, Binding, GameInputPlugin, Processor, RollDir, Stick,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum InputAction {
    Jump,
    Special,
    Attack,
    MoveX,
    MoveY,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameInputPlugin::<InputAction>::default())
            .add_systems(Startup, setup_bindings);
    }
}

fn setup_bindings(mut map: ResMut<ActionMap<InputAction>>) {
    // Jump: A button, tap up on the left stick, or a strong left-stick X push.
    map.bind(InputAction::Jump, Binding::button(GamepadButton::North));
    map.bind(InputAction::Jump, Binding::button(GamepadButton::East));
    // map.bind(InputAction::Jump, Binding::tap(Stick::Left, 2)); TODO: improve tap detection

    // Special: left shoulder / right trigger style shortcuts.
    map.bind(InputAction::Special, Binding::button(GamepadButton::West));

    // Attack: standard confirm button plus a right-stick roll gesture.
    map.bind(InputAction::Attack, Binding::button(GamepadButton::South));
    map.bind(InputAction::Attack, Binding::roll(Stick::Right, RollDir::Ccw).with(Processor::Threshold(0.1)));

    // Motion: direct analog sticks.
    map.bind(InputAction::MoveX, Binding::axis(GamepadAxis::LeftStickX));
    map.bind(InputAction::MoveY, Binding::axis(GamepadAxis::LeftStickY));
}


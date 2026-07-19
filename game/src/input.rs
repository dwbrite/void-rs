use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;
use crate::input::AttackControl::{East, MoveXY, North, South, West};
use crate::systems::input::{
    ActionMap, Binding, GameInputPlugin, Processor, RollDir, Stick,
};
use crate::systems::input::RollDir::{Ccw, Cw};
use crate::systems::input::signal::Octant;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AttackControl {
    North,
    South,
    Neutral,
    East,
    West,
    MoveXY, // use the stick DI (or whatever other input you're using for MoveX/MoveY) to determine the direction of the attack
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum InputAction {
    Jump,
    Special(AttackControl),
    Attack(AttackControl),
    DropDown,
    Spin,
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

    // Special: left shoulder / right trigger style shortcuts.
    map.bind(InputAction::Special(MoveXY), Binding::button(GamepadButton::West));

    // Attack: standard confirm button plus a right-stick roll gesture.
    map.bind(InputAction::Attack(MoveXY), Binding::button(GamepadButton::South));

    map.bind(InputAction::Attack(North), Binding::tap(Stick::Right, Octant::North).with(Processor::Threshold(0.1)));
    map.bind(InputAction::Attack(East), Binding::tap(Stick::Right, Octant::East).with(Processor::Threshold(0.1)));
    map.bind(InputAction::Attack(West), Binding::tap(Stick::Right, Octant::West).with(Processor::Threshold(0.1)));
    // south-attack has two parts
    // map.bind(InputAction::Attack(South), Binding::tap(Stick::Right, Octant::South).with(Processor::Threshold(0.1)));
    map.bind(InputAction::Attack(South), Binding::axis(GamepadAxis::RightStickY).with(Processor::Negate).with(Processor::Threshold(0.9)));

    let b = Binding::button(GamepadButton::South);

    // Roll c-stick to spin:
    map.bind(InputAction::Spin, Binding::roll(Stick::Right, Ccw)
        .with(Processor::Threshold(0.120))
        .with(Processor::DownSlew { from: 0.0, slew: 0.03})
        .with(Processor::UpSlew { from: 0.0, slew: 0.07}));
    map.bind(InputAction::Spin, Binding::roll(Stick::Right, Cw)
        .with(Processor::Threshold(0.120))
        .with(Processor::DownSlew { from: 0.0, slew: 0.03})
        .with(Processor::UpSlew { from: 0.0, slew: 0.07}));

    // Motion: direct analog sticks.
    map.bind(InputAction::MoveX, Binding::axis(GamepadAxis::LeftStickX));
    map.bind(InputAction::MoveY, Binding::axis(GamepadAxis::LeftStickY));

    // Drop down fast
    map.bind(InputAction::DropDown, Binding::tap(Stick::Left, Octant::South).with(Processor::Threshold(0.1)));
}


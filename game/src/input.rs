use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;
use bevy::prelude::GamepadAxis::RightStickY;
use crate::input::AttackControl::{East, MoveXY, North, South, West};
use crate::systems::input::{
    ActionMap, Binding, GameInputPlugin, Processor, RollDir, Stick,
};
use crate::systems::input::Processor::Negate;
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
    Dash,
    AirJump,
    DownRelease,
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
    // press/trigger: tap — now roll-suppressed like the other three directions
    map.bind(InputAction::Attack(South),
             Binding::axis(RightStickY).with(Processor::Negate).with(Processor::Threshold(0.9)));
    // hold: the old axis binding, moved to an action nothing treats as a press
    map.bind(InputAction::DownRelease,
             Binding::deflect_release(Stick::Right, Octant::South).with(Processor::Threshold(0.1)));

    // Roll c-stick to spin:
    map.bind(InputAction::Spin, Binding::roll(Stick::Right, Ccw)
        .with(Processor::Threshold(0.100))
        .with(Processor::DownSlew { from: 0.0, slew: 0.03})
        .with(Processor::UpSlew { from: 0.0, slew: 0.07}));
    map.bind(InputAction::Spin, Binding::roll(Stick::Right, Cw)
        .with(Processor::Threshold(0.100))
        .with(Processor::DownSlew { from: 0.0, slew: 0.03})
        .with(Processor::UpSlew { from: 0.0, slew: 0.07}));

    // Motion: direct analog sticks.
    map.bind(InputAction::MoveX, Binding::axis(GamepadAxis::LeftStickX));
    map.bind(InputAction::MoveY, Binding::axis(GamepadAxis::LeftStickY));

    // Dash:
    map.bind(InputAction::Dash, Binding::tap(Stick::Left, Octant::East).with(Processor::Threshold(0.1)));
    map.bind(InputAction::Dash, Binding::tap(Stick::Left, Octant::West).with(Processor::Threshold(0.1)));

    // Drop down fast
    map.bind(InputAction::DropDown, Binding::tap(Stick::Left, Octant::South).with(Processor::Threshold(0.1)));

    map.bind(InputAction::AirJump, Binding::tap(Stick::Left, Octant::North).with(Processor::Threshold(0.1)));
}


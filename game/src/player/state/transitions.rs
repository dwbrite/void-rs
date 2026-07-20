use std::collections::HashSet;
use bevy::math::Vec2;
use bevy::prelude::{Changed, Entity, MessageReader, Query, ResMut};
use bevy::tasks::futures_lite::StreamExt;
use bevy_aseprite_ultra::prelude::{Animation, AnimationEvents, AseAnimation};
use bevy_rapier2d::dynamics::Velocity;
use crate::input::{AttackControl, InputAction};
use crate::player::{CharacterStatus, AIR_JUMP_BASE_IMPULSE, AIR_JUMP_DURATION};
use crate::player::state::{AirborneState, Facing, PreviousState, SpringMass};
use crate::player::state::AirborneState::{Airborne, Grounded};
use crate::player::state::PlayerState::SuperCrouch;
use super::types::{AirJumpsRemaining, PlayerAction, PlayerState, StateTicks};
use crate::systems::input::ActionMap;


// TODO: track how much we've been jumping around - let lil chud catch his breath over a few seconds, with heavy breathing to light breathing.
// TODO: once he's caught his breath, if he's bored he can look around, look down at the ground, kick dirt, etc. various idle animations bc fuck yeah.
// TODO: down-b charge into side-spec for bonus momentum

const BUFFER_WINDOW: f32 = 0.1;
const SUPER_CROUCH_CHARGE_TICKS: u32 = 24;
const SUPER_JUMP_LOCK_TICKS: u32 = 12;
const JUMP_LOCKED_TICKS: u32 = 4;
const INTERACT_MIN_TICKS: u32 = 0;


/// Named fields instead of a 10-wide tuple. Iteration yields
/// `PlayerQueryItem`, so everything below is `p.state`, `p.facing`, etc.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct PlayerQuery {
    entity: Entity,
    state: &'static mut PlayerState,
    action: &'static mut PlayerAction,
    air_jumps: &'static mut AirJumpsRemaining,
    state_ticks: &'static StateTicks, // was `mut` before, but never written here
    airborne: &'static AirborneState,
    old_state: &'static mut PreviousState,
    velocity: &'static Velocity,
    facing: &'static mut Facing,
    status: &'static mut CharacterStatus,
    spring_mass: &'static SpringMass,
}



use PlayerState::*;
use crate::player::state::transitions::AtkDirection::{Fwd, Neutral};
use bevy::ecs::change_detection::DetectChangesMut;
use bevy::ecs::query::QueryData;
use crate::input::AttackControl::{MoveXY, South};
use crate::player::state::AtkDirection::{Back, Down, Up};
use crate::player::state::PlayerAction::SpinAttack;

#[derive(Copy, Clone, Debug)]
pub enum AtkDirection {
    Up,
    Down,
    Back,
    Fwd,
    Neutral,
}

/// Cardinal (c-stick style) direction. North/South are absolute;
/// East/West are interpreted relative to facing, so East while facing
/// left is a *back* attack.
fn cardinal_direction(control: AttackControl, facing: &Facing) -> AtkDirection {
    match control {
        AttackControl::North => Up,
        AttackControl::South => Down,
        AttackControl::East if facing.sign() > 0.0 => Fwd,
        AttackControl::East => Back,
        AttackControl::West if facing.sign() > 0.0 => Back,
        AttackControl::West => Fwd,
        AttackControl::Neutral => AtkDirection::Neutral,
        AttackControl::MoveXY => unreachable!("MoveXY resolves via attack_direction"),
    }
}

fn pressed_or_buffered(map: &mut ActionMap<InputAction>, busy: bool, action: InputAction) -> bool {
    map.just_pressed(action) || (!busy && map.buffered_press(action, BUFFER_WINDOW))
}

/// Checks every binding of one action family (`InputAction::Attack` or
/// `InputAction::Special` — pass the enum constructor) and resolves the
/// direction it implies. Cardinal bindings win over MoveXY: the c-stick
/// is the more explicit input, so it overrides stick DI.
fn resolve_directional(
    map: &mut ActionMap<InputAction>,
    busy: bool,
    facing: &Facing,
    move_x: f32,
    move_y: f32,
    family: fn(AttackControl) -> InputAction,
) -> Option<AtkDirection> {
    use AttackControl::*;
    for control in [North, South, East, West, Neutral] {
        if pressed_or_buffered(map, busy, family(control)) {
            return Some(cardinal_direction(control, facing));
        }
    }
    if pressed_or_buffered(map, busy, family(MoveXY)) {
        return Some(attack_direction(move_x, move_y, facing));
    }
    None
}


struct FrameInput {
    move_x: f32,
    move_y: f32,
    attack_held: bool,
    action: PlayerAction,
}

// wherever InputAction lives, or a small input helpers module:
pub fn spin_input_speed(map: &ActionMap<InputAction>) -> f32 {
    if map.value(InputAction::Spin) >= 1.0 {
        map.raw_value(InputAction::Spin) * 1.2
    } else {
        map.value(InputAction::Spin) * 0.060 * 1.2
    }
}

impl FrameInput {
    fn read(map: &mut ActionMap<InputAction>, facing: &Facing, busy: bool) -> Self {
        let move_x = map.value(InputAction::MoveX);
        let move_y = map.value(InputAction::MoveY);

        // Priority: Jump > Special > Attack, same as before.
        // (Jump is deliberately unbuffered; consider buffering it too —
        // it's a common QoL win for inputs a few frames before landing.)
        let action = if map.just_pressed(InputAction::Jump) {
            PlayerAction::Jump
        } else if let Some(dir) = resolve_directional(map, busy, facing, move_x, move_y, InputAction::Special) {
            PlayerAction::Special(dir)
        } else if let Some(dir) = resolve_directional(map, busy, facing, move_x, move_y, InputAction::Attack) {
            PlayerAction::Attack(dir)
        } else if map.is_down(InputAction::Spin) {
            PlayerAction::SpinAttack(spin_input_speed(map))
        } else if map.is_down(InputAction::DropDown) {
            PlayerAction::DropDown
        } else {
            PlayerAction::None
        };

        Self {
            move_x,
            move_y,
            attack_held: map.is_down(InputAction::Attack(AttackControl::MoveXY)),
            action,
        }
    }
}

/// The state you fall back to when an attack/animation/jump finishes.
fn neutral_state(airborne: AirborneState) -> PlayerState {
    match airborne {
        Grounded => Idle,
        Airborne => ControlledAirborne,
    }
}


pub fn attack_direction(move_x: f32, move_y: f32, facing: &Facing) -> AtkDirection {
    match (move_x, move_y) {
        (_, y) if y >  0.6 => AtkDirection::Up,
        (_, y) if y < -0.85 => AtkDirection::Down,
        (x, _) if x * facing.sign() >  0.4 => AtkDirection::Fwd,
        (x, _) if x * facing.sign() < -0.4 => AtkDirection::Back,
        (_, _) => { AtkDirection::Neutral }
    }
}

pub fn update_playerstate(
    mut animation_events: MessageReader<AnimationEvents>,
    mut query: Query<PlayerQuery>,
    mut input_map: ResMut<ActionMap<InputAction>>,
) {
    let finished: HashSet<Entity> = animation_events
        .read()
        .filter_map(|e| match e {
            AnimationEvents::Finished(entity) => Some(*entity),
            _ => None,
        })
        .collect();

    for mut p in &mut query {
        let input = FrameInput::read(&mut input_map, &p.facing, p.status.busy);
        *p.action = input.action;

        if !input_map.is_down(InputAction::Jump) {
            (*p.status).holding_jump = false;
        }

        let start_state = PreviousState(*p.state);
        let anim_finished = finished.contains(&p.entity);
        let airborne = *p.airborne;

        match (*p.state, *p.action) {
            // -------- state-specific overrides (checked first) --------
            (PreJump, _) => {
                if anim_finished || !p.status.holding_jump  {
                    *p.state = Jumping;
                }
            }

            (SuperCrouch, PlayerAction::Jump) if p.state_ticks.0 >= SUPER_CROUCH_CHARGE_TICKS => {
                *p.state = SuperJump;
            }

            // SuperJump lockout: swallow all inputs until the jump has
            // played out. Intentionally empty.
            (SuperJump, _) if p.status.busy || p.state_ticks.0 <= SUPER_JUMP_LOCK_TICKS => {}

            // SuperJump lockout: swallow all inputs until the jump has
            // played out. Intentionally empty.
            (Jumping, _) if p.status.busy || p.state_ticks.0 <= JUMP_LOCKED_TICKS => {}

            // ChargedPunch rides until we're falling fast or grounded.
            (ChargedPunch, PlayerAction::None) => {
                let falling_fast = p.velocity.linear.y < -30.0;
                if !p.status.busy && (falling_fast || airborne == Grounded) {
                    *p.state = neutral_state(airborne);
                }
            }

            // GroundPound cancels into ChargedPunch on a horizontal special.
            (GroundPound, PlayerAction::Special(Back | Fwd)) => {
                change_facing(input.move_x, &mut p.facing);
                *p.state = ChargedPunch;
            }

            (PlayerState::SpinMove(speed), PlayerAction::None) => {
                *p.state = neutral_state(airborne);
            }

            (PlayerState::PreDownKick, PlayerAction::None) => {
                if !input_map.is_down(InputAction::Attack(South)) {
                    *p.state = DownKick;
                }
            }

            // -------- animation-completion exits --------
            (UpAir | FwdAir | BackAir | Jumping, PlayerAction::None)
            if anim_finished =>
                {
                    *p.state = neutral_state(airborne);
                }
            (NeutralAir, _) if anim_finished => *p.state = Interact,
            (Interact, _) => {
                if p.state_ticks.0 >= INTERACT_MIN_TICKS && !input.attack_held {
                    *p.state = Interactnt;
                }
            }
            (Interactnt, _) if anim_finished => *p.state = neutral_state(airborne),

            (ControlledAirborne, PlayerAction::DropDown) => {
                if p.velocity.linear.y > -AIR_JUMP_BASE_IMPULSE {
                    *p.state = SmashDrop;
                }
            }

            // -------- jumps --------
            (_, PlayerAction::Jump) if !p.status.no_jump && !p.status.busy => match airborne {
                Airborne if p.air_jumps.0 > 0 => {
                    p.air_jumps.0 -= 1;
                    // TODO: should change facing be strictly by momentum here?
                    change_facing(input.move_x, &mut p.facing);
                    *p.state = AirJump;
                }
                Grounded => {
                    change_facing(input.move_x, &mut p.facing);

                    if p.spring_mass.vy > 0.0 {
                        p.status.holding_jump = true;
                        *p.state = Jumping;
                    } else {
                        *p.state = PreJump;
                    }
                }
                _ => {} // airborne, out of air jumps
            },
            (state, PlayerAction::Jump) => {
                println!("jump blocked in state: {state:?}, air_jumps: {}", p.air_jumps.0);
            }

            // Initial jump lock over: hand control back.
            (Jumping | AirJump | SuperJump, PlayerAction::None)
            if airborne == Airborne && !p.status.busy =>
                {
                    *p.state = ControlledAirborne;
                }

            (GroundPound, PlayerAction::None) => if !p.status.busy && airborne == Grounded  {
                if input.move_y < -0.96 {
                    // DELIBERATE: bypassing change detection keeps
                    // reset_state_ticks from firing, so a landed ground
                    // pound carries its ticks into SuperCrouch --
                    // down-B -> hold down -> super jump charge is already
                    // done. Do not "fix" this into a normal write.
                    *p.state.bypass_change_detection() = SuperCrouch;
                } else {
                    *p.state = Idle;
                }
            }

            // -------- grounded movement --------
            (_, PlayerAction::None) if !p.status.busy && airborne == Grounded => {
                change_facing(input.move_x, &mut p.facing);
                let next = if input.move_x.abs() >= 0.3 {
                    Running
                } else if input.move_y > 0.3 {
                    Lookup
                } else if input.move_y < -0.96
                    && (p.state_ticks.0 >= SUPER_CROUCH_CHARGE_TICKS || *p.state == SuperCrouch)
                {
                    SuperCrouch
                } else if input.move_y < -0.3 {
                    Crouch
                } else {
                    Idle
                };
                // Only writes (and only fires Changed<PlayerState>) when the state actually changes
                p.state.set_if_neq(next);
            }

            (_, PlayerAction::SpinAttack(speed)) => {
                if !matches!(*p.state, SpinMove(_)) {
                    *p.state = SpinMove(speed);
                }
            }

            // -------- attacks --------
            // NOTE: air and ground attacks currently share the aerial
            // states on purpose-for-now; split this arm when ground
            // attacks get their own states.
            (_, PlayerAction::Attack(dir)) if !p.status.busy => {
                *p.state = match dir {
                    Up => UpAir,
                    Down => PreDownKick,
                    Back => {
                        if matches!(airborne, Grounded) {
                            change_facing(input.move_x, &mut p.facing);
                            FwdAir
                        } else {
                            BackAir
                        }
                    },
                    Fwd => FwdAir,
                    AtkDirection::Neutral => NeutralAir,
                };
            }

            (_, PlayerAction::Special(dir)) if !p.status.busy => {
                *p.state = match dir {
                    Up => Roll,
                    Down if airborne != Grounded => GroundPound,
                    Back | Fwd => {
                        change_facing(input.move_x, &mut p.facing);
                        ChargedPunch
                    }
                    // grounded down-special and neutral both roll
                    Down | AtkDirection::Neutral => Roll,
                };
            }

            // -------- fallthrough --------
            _ if !p.status.busy => *p.state = neutral_state(airborne),
            _ => {}
        }

        *p.old_state = start_state;
    }
}


fn change_facing(move_x: f32, facing: &mut Facing) {
    if move_x < -0.2 {
        *facing = Facing::Left;
    } else if move_x > 0.2 {
        *facing = Facing::Right;
    }
}


pub fn reset_state_ticks(
    mut query: Query<(&mut StateTicks, &PreviousState, &PlayerState), Changed<PlayerState>>,
) {
    for (mut ticks, prestate, state) in &mut query {
        // With set_if_neq in the movement arm, Changed<PlayerState> should
        // only fire on real transitions -- this check is now belt-and-braces.
        if prestate.0 != *state {
            println!("reset state ticks: {ticks:?}, {:?} -> {state:?}", prestate.0);
            ticks.0 = 0;
        }
    }
}

pub fn increment_state_ticks(mut query: Query<&mut StateTicks>) {
    for mut ticks in &mut query {
        ticks.0 += 1;
    }
}

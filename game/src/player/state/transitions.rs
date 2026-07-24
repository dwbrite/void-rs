use std::collections::HashSet;
use bevy::math::Vec2;
use bevy::prelude::{Changed, Component, Entity, MessageReader, Query, ResMut};
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
const JUMP_BUFFER_WINDOW: f32 = 0.15;
const SUPER_CROUCH_CHARGE_TICKS: u32 = 24;
const SUPER_JUMP_LOCK_TICKS: u32 = 12;
const JUMP_LOCKED_TICKS: u32 = 4;
const INTERACT_MIN_TICKS: u32 = 0;


#[derive(Component, Debug)]
pub enum AnimationStatus {
    Finished,
    Playing,
    Looped, // boundary of start/end
}


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
    animation_status: &'static mut AnimationStatus,
    animation: &'static AseAnimation,
}

pub fn update_animation_status(
    mut query: Query<PlayerQuery>,
    mut animation_events: MessageReader<AnimationEvents>)
{
    let finished: HashSet<Entity> = animation_events
        .read()
        .filter_map(|e| match e {
            AnimationEvents::Finished(entity) => Some(*entity),
            _ => None,
        })
        .collect();

    for mut p in &mut query {
        if finished.contains(&p.entity) {
            *p.animation_status = AnimationStatus::Finished;
            return;
        }
        if !(*p.state == *&p.old_state.0) {
            *p.animation_status = AnimationStatus::Playing;
        }
    }
}



use PlayerState::*;
use crate::player::state::transitions::AtkDirection::{Fwd, Neutral};
use bevy::ecs::change_detection::DetectChangesMut;
use bevy::ecs::query::QueryData;
use crate::input::AttackControl::{MoveXY, South};
use crate::player::state::AnimationStatus::{Finished, Playing};
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
    (!busy && map.buffered_press(action, BUFFER_WINDOW)) || map.just_pressed(action)
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

        let action = if map.just_pressed(InputAction::Jump) || (!busy && map.buffered_press(InputAction::Jump, JUMP_BUFFER_WINDOW)) {
            PlayerAction::Jump
        } else if map.just_pressed(InputAction::AirJump) || (!busy && map.buffered_press(InputAction::AirJump, JUMP_BUFFER_WINDOW)) {
            PlayerAction::AirJump
        } else if let Some(dir) = resolve_directional(map, busy, facing, move_x, move_y, InputAction::Special) {
            PlayerAction::Special(dir)
        } else if let Some(dir) = resolve_directional(map, busy, facing, move_x, move_y, InputAction::Attack) {
            PlayerAction::Attack(dir)
        } else if map.is_down(InputAction::Spin) {
            PlayerAction::SpinAttack(spin_input_speed(map))
        } else if map.is_down(InputAction::DropDown) {
            PlayerAction::DropDown
        } else if map.is_down(InputAction::Dash){
            PlayerAction::Dash
        } else if map.just_pressed(InputAction::DownRelease){
            PlayerAction::DownRelease
        } else {
            PlayerAction::None
        };
        // Priority: Jump > Special > Attack, same as before.

        Self {
            move_x,
            move_y,
            attack_held: map.is_down(InputAction::Attack(AttackControl::MoveXY)),
            action,
        }
    }
}

/// The state you fall back to when an attack/animation/jump finishes.
fn neutral_state(airborne: AirborneState, input: &FrameInput, facing: &mut Facing) -> PlayerState {
    match airborne {
        Grounded => {
            change_facing(input.move_x, facing);
            let next = if input.move_x.abs() >= 0.3 {
                Running
            } else if input.move_y > 0.3 {
                Lookup
            } else if input.move_y < -0.3 {
                Crouch
            } else {
                Idle
            };
            next
        },
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
    mut query: Query<PlayerQuery>,
    mut input_map: ResMut<ActionMap<InputAction>>,
) {
    for mut p in &mut query {
        let input_locked = p.status.busy
            || matches!(*p.state, DashFlop | Interact | Interactnt)
            || (matches!(*p.state, Jumping)   && p.state_ticks.0 <= JUMP_LOCKED_TICKS)
            || (matches!(*p.state, SuperJump) && p.state_ticks.0 <= SUPER_JUMP_LOCK_TICKS);
        let input = FrameInput::read(&mut input_map, &p.facing, input_locked);

        // // in update_playerstate, right after FrameInput::read:
        // if !matches!(input.action, PlayerAction::None) {
        //     println!("[action {:>9.3}] {:?} (state={:?} busy={} locked={})",
        //              now, input.action, *p.state, p.status.busy, input_locked);
        // }

        *p.action = input.action;

        if !input_map.is_down(InputAction::Jump) {
            (*p.status).holding_jump = false;
        }

        let start_state = PreviousState(*p.state);
        let airborne = *p.airborne;
        let anim_finished = matches!(*p.animation_status, AnimationStatus::Finished);

        match (*p.state, *p.action) {
            // -------- state-specific overrides (checked first) --------
            (PreJump, _) => {
                if anim_finished || !p.status.holding_jump  {
                    *p.state = Jumping;
                }
            }

            (BackAir, _) if airborne == Grounded && (4..=48).contains(&p.state_ticks.0) => {
                p.status.slide_charged = false;
                *p.facing = match *p.facing {
                    Facing::Left => Facing::Right,
                    Facing::Right => Facing::Left,
                };
                *p.state = Slide;
            }

            (UpAir, _) if airborne == Grounded && p.state_ticks.0 >= 4 => {
                *p.state = DashFlop2;
            }

            (DownKick, _) => {
                if anim_finished {
                    *p.state = neutral_state(airborne, &input, &mut p.facing);
                    println!("swag");
                }
            }

            (DashFlop | DashFlop2, action) => {
                let ticks = p.state_ticks.0;
                if ticks >= 32 {
                    *p.state = match action {
                        PlayerAction::Attack(_) => {
                            *p.facing = match *p.facing {
                                Facing::Left => Facing::Right,
                                Facing::Right => Facing::Left,
                            };
                            UpAir
                        }
                        PlayerAction::None => DashFlop,
                        _ => neutral_state(airborne, &input, &mut p.facing),
                    };
                }

                // otherwise we're doing _nothing_
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
                    *p.state = neutral_state(airborne, &input, &mut p.facing);
                }
            }

            // GroundPound cancels into ChargedPunch on a horizontal special.
            (GroundPound, PlayerAction::Special(Back | Fwd)) => {
                change_facing(input.move_x, &mut p.facing);
                *p.state = ChargedPunch;
            }

            (PlayerState::SpinMove(speed), PlayerAction::None) => {
                *p.state = neutral_state(airborne, &input, &mut p.facing);
            }

            (PreDownKick, _) if airborne == Grounded && p.velocity.linear.x.abs() >= 48.0 => {
                p.status.slide_charged = true;
                *p.state = Slide;
            }

            (PlayerState::PreDownKick, _) => {
                if !input_map.is_down(InputAction::DownRelease) {
                    *p.state = DownKick;
                }
            }

            (UpAir, PlayerAction::Attack(Back)) if p.state_ticks.0 > 12 => {
                *p.state = BackAir;
            }

            // -------- animation-completion exits --------
            (UpAir | FwdAir | BackAir | Jumping, PlayerAction::None)
            if anim_finished =>
                {
                    *p.state = neutral_state(airborne, &input, &mut p.facing);
                }
            (NeutralAir, _) => *p.state = Interact,
            (Interact, _) => {
                if !input.attack_held {
                    *p.state = Interactnt;
                }
            }
            (Interactnt, _) if anim_finished => *p.state = neutral_state(airborne, &input, &mut p.facing),

            (ControlledAirborne, PlayerAction::DropDown) => {
                if p.velocity.linear.y > -AIR_JUMP_BASE_IMPULSE {
                    *p.state = FlickDrop;
                }
            }

            (Slide, action) => {
                if p.status.slide_charged && !input_map.is_down(InputAction::DownRelease) {
                    *p.state = DownKick;
                } else {
                    match action {
                        PlayerAction::Attack(_) => *p.state = UpAir,
                        PlayerAction::Jump => *p.state = Jumping,
                        _ => {
                            if p.velocity.linear.x.abs() <= 1.0 && p.state_ticks.0 >= 16 {
                                *p.state = neutral_state(airborne, &input, &mut p.facing);
                            }
                        }
                    }
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

            // -------- jumps --------
            (_, PlayerAction::AirJump) if !p.status.no_jump && !p.status.busy => match airborne {
                Airborne if p.air_jumps.0 > 0 => {
                    p.air_jumps.0 -= 1;
                    // TODO: should change facing be strictly by momentum here?
                    change_facing(input.move_x, &mut p.facing);
                    *p.state = AirJump;
                }
                _ => {} // airborne, out of air jumps
            },

            (_, PlayerAction::Dash) if !p.status.busy && matches!(airborne, Grounded) => {
                change_facing(input.move_x, &mut p.facing);
                *p.state = FlickDash;
            }
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

            (_, PlayerAction::SpinAttack(speed)) if !p.status.busy => {
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
                    Down => {
                        match *p.state {
                            SuperCrouch if p.state_ticks.0 >= SUPER_CROUCH_CHARGE_TICKS => SuperJump,
                            _ => PreDownKick,
                        }
                    },
                    Back => {
                        if matches!(airborne, Grounded) && p.velocity.linear.x.abs() > 64.0 {
                            DashFlop
                        } else if matches!(airborne, Grounded) {
                            *p.facing = match *p.facing {
                                Facing::Left => Facing::Right,
                                Facing::Right => Facing::Left,
                            };
                            FwdAir
                        } else {
                            BackAir
                        }
                    },
                    Fwd => {
                        if matches!(airborne, Grounded) && p.velocity.linear.x.abs() > 64.0 {
                            DashFlop
                        } else {
                            FwdAir
                        }
                    },
                    Neutral => NeutralAir,
                };
            }

            (_, PlayerAction::Special(dir)) if !p.status.busy => {
                *p.state = match dir {
                    Up => neutral_state(airborne, &input, &mut p.facing),
                    Down if airborne != Grounded => GroundPound,
                    Back | Fwd => {
                        change_facing(input.move_x, &mut p.facing);
                        ChargedPunch
                    }
                    // grounded down-special and neutral both roll
                    Neutral => Roll,
                    Down => match *p.state {
                        SuperCrouch => SuperCrouch,
                        _ => GroundPound,
                    }
                };
            }

            // -------- fallthrough --------
            _ if !p.status.busy => *p.state = neutral_state(airborne, &input, &mut p.facing),
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

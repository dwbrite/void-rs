use std::collections::HashSet;
use bevy::math::Vec2;
use bevy::prelude::{Changed, Entity, MessageReader, Query, ResMut};
use bevy::tasks::futures_lite::StreamExt;
use bevy_aseprite_ultra::prelude::{AnimationEvents, AseAnimation};
use bevy_rapier2d::dynamics::Velocity;
use crate::input::InputAction;
use crate::player::{CharacterStatus, AIR_JUMP_DURATION};
use crate::player::state::{AirborneState, Facing, PreviousState};
use crate::player::state::AirborneState::{Airborne, Grounded};
use crate::player::state::PlayerState::SuperCrouch;
use super::types::{AirJumpsRemaining, PlayerAction, PlayerState, StateTicks};
use crate::systems::input::ActionMap;


// TODO: track how much we've been jumping around - let lil chud catch his breath over a few seconds, with heavy breathing to light breathing.
// TODO: once he's caught his breath, if he's bored he can look around, look down at the ground, kick dirt, etc. various idle animations bc fuck yeah.


use PlayerState::*;
use crate::player::state::transitions::AtkDirection::{Fwd, Neutral};
use bevy::ecs::change_detection::DetectChangesMut;


enum AtkDirection {
    Up,
    Down,
    Back,
    Fwd,
    Neutral,
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
    mut query: Query<(Entity, &mut PlayerState, &mut PlayerAction, &mut AirJumpsRemaining, &mut StateTicks, &AirborneState, &mut PreviousState, &Velocity, &mut Facing, &CharacterStatus)>,
    mut input_map: ResMut<ActionMap<InputAction>>,
) {
    let finished: HashSet<Entity> = animation_events
        .read()
        .filter_map(|e| match e {
            AnimationEvents::Finished(entity) => Some(*entity),
            _ => None,
        })
        .collect();

    // TODO: down-b charge into side-spec for bonus momentum
    for (entity, mut playerstate, mut playeraction, mut air_jumps, mut state_ticks, airborne, mut old_state, velocity, mut facing, status) in &mut query {
        let move_x = input_map.value(InputAction::MoveX);
        let move_y = input_map.value(InputAction::MoveY);
        let jump_pressed = input_map.just_pressed(InputAction::Jump);
        let spec_pressed = input_map.just_pressed(InputAction::Special);
        let spec_buffered = !status.busy && input_map.buffered_press(InputAction::Special, 0.1);
        let spec_pressed = spec_pressed || spec_buffered;
        let atk_pressed = input_map.just_pressed(InputAction::Attack);
        let atk_buffered = !status.busy && input_map.buffered_press(InputAction::Attack, 0.1);
        let atk_pressed = atk_pressed || atk_buffered;
        let atk_held = input_map.is_down(InputAction::Attack);

        *playeraction = if jump_pressed {
            PlayerAction::Jump
        } else if spec_pressed {
            PlayerAction::Special(Vec2::new(move_x, move_y))
        } else if atk_pressed {
            PlayerAction::Attack(Vec2::new(move_x, move_y))
        } else {
            PlayerAction::None
        };

        let start_state = PreviousState(*playerstate);
        match (&*playerstate, &*playeraction) {
            (SuperCrouch, PlayerAction::Jump) if state_ticks.0 >= 24 => {
                *playerstate = SuperJump;
            }

            (SuperJump, _) if status.busy || state_ticks.0 <= 20 => {
                // this is here for a reason I think
            }

            // special case for our special boy :^)
            (ChargedPunch, _) => {
                if !status.busy && (velocity.linear.y < -30.0 || *airborne == Grounded){
                    if *airborne == Grounded {
                        *playerstate = Idle;
                    } else {
                        *playerstate = ControlledAirborne;
                    }
                }
            },

            // another special case~
            (GroundPound, PlayerAction::Special(_dir)) => {
                match attack_direction(move_x, move_y, &facing) {
                    AtkDirection::Back | AtkDirection::Fwd => {
                        change_facing(move_x, &mut facing);
                        *playerstate = ChargedPunch;
                    }
                    _ => {}
                }
            },

            // attacks/animations that are complete upon animation completion
            (UpAir | DownAir | FwdAir | BackAir| Jumping, PlayerAction::None) if finished.contains(&entity) => {
                match airborne {
                    Airborne => {
                        *playerstate = ControlledAirborne;
                    }
                    Grounded => {
                        *playerstate = Idle;
                    }
                }
            },
            (NeutralAir, _) if finished.contains(&entity) => {
                *playerstate = Interact;
            }
            (Interact, _) => {
                if state_ticks.0 >= 8 && !atk_held {
                    *playerstate = Interactnt;
                }
            }
            (Interactnt, _) if finished.contains(&entity) => {
                match airborne {
                    Airborne => {
                        *playerstate = ControlledAirborne;
                    }
                    Grounded => {
                        *playerstate = Idle;
                    }
                }
            }
            // jump
            (_, PlayerAction::Jump) if !status.no_jump && !status.busy => {
                if *airborne == Airborne && air_jumps.0 > 0 {
                    *playerstate = AirJump;
                    air_jumps.0 -= 1;
                    // TODO: figure out if we should change facing strictly by momentum here?
                    change_facing(move_x, &mut facing);
                } else if *airborne == Grounded {
                    *playerstate = Jumping;
                    change_facing(move_x, &mut facing);
                }
            },
            // jump debug
            (state, PlayerAction::Jump) => {
                println!("jump blocked in state: {:?}, air_jumps: {}", state, air_jumps.0);
            },

            // once initial jump lock ends, hand over to fully controllable airborne state
            (Jumping | AirJump | SuperJump, PlayerAction::None) if *airborne == Airborne && !status.busy => {
                *playerstate = ControlledAirborne;
            },

            (GroundPound, PlayerAction::None) if !status.busy => {
                if *airborne == Grounded {
                    if move_y < -0.96 {
                        // make this access not trigger bevy ecs "Changed<PlayerState>"
                        *playerstate.bypass_change_detection() = SuperCrouch;
                    } else {
                        *playerstate = Idle;
                    }
                }
            }

            (_, PlayerAction::None) if !status.busy && *airborne == Grounded => {
                change_facing(move_x, &mut facing);
                if move_x.abs() >= 0.3 {
                    *playerstate = Running
                } else if move_y > 0.3 {
                    *playerstate = Lookup
                } else if move_y < -0.96 && (state_ticks.0 >= 24 || *playerstate.as_ref() == SuperCrouch) {
                    if old_state.0 != SuperCrouch {
                        *playerstate = SuperCrouch;
                    }
                } else if move_y < -0.3 {
                    *playerstate = Crouch
                } else {
                    *playerstate = Idle
                }
            }

            // airborne attack
            (_, PlayerAction::Attack(_)) if *airborne == Airborne => {
                *playerstate = match attack_direction(move_x, move_y, &facing) {
                    AtkDirection::Up => { UpAir }
                    AtkDirection::Down => { DownAir }
                    AtkDirection::Back => { BackAir }
                    AtkDirection::Fwd => { FwdAir }
                    AtkDirection::Neutral => { NeutralAir }
                }
            }

            // ground attack
            (_, PlayerAction::Attack(_)) if *airborne == Grounded => {
                *playerstate = match attack_direction(move_x, move_y, &facing) {
                    AtkDirection::Up => { UpAir }
                    AtkDirection::Down => { DownAir }
                    AtkDirection::Back => { BackAir }
                    AtkDirection::Fwd => { FwdAir }
                    AtkDirection::Neutral => { NeutralAir }
                }
            }

            // special attack
            (_, PlayerAction::Special(_)) => {
                *playerstate = match attack_direction(move_x, move_y, &facing) {
                    AtkDirection::Up => { SpinMove }
                    AtkDirection::Down if *airborne != Grounded => { GroundPound }
                    AtkDirection::Back => { change_facing(move_x, &mut facing); ChargedPunch }
                    AtkDirection::Fwd => { change_facing(move_x, &mut facing); ChargedPunch }
                    AtkDirection::Neutral => { Roll }
                    AtkDirection::Down => { Roll } // yeah fuck you
                }
            }
            _ if *airborne == Grounded && !status.busy => *playerstate = Idle,
            _ if *airborne == Airborne && !status.busy => *playerstate = ControlledAirborne,
            _ => {}
        }

        *old_state = start_state;
    }
}

fn change_facing(move_x: f32, facing: &mut Facing) {
    if move_x.abs() >= 0.2 {
        *facing = match move_x {
            x if x < -0.2 => Facing::Left,
            x if x > 0.2 => Facing::Right,
            _ => *facing
        };
    }
}

pub fn reset_state_ticks(mut query: Query<(&mut StateTicks, &PreviousState, &PlayerState), Changed<PlayerState>>) {
    for (mut ticks, prestate, state) in &mut query {
        if prestate.0 != *state {
            println!("reset state tick: {:?}, state: {:?} -> {:?}", ticks, prestate.0, state);
            ticks.0 = 0;
        }
    }
}

pub fn increment_state_ticks(mut query: Query<&mut StateTicks>) {
    for mut ticks in &mut query {
        ticks.0 += 1;
    }
}

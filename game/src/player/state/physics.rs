use bevy::prelude::Query;
use bevy_rapier2d::dynamics::{ExternalImpulse, Velocity};
use crate::input::RawInput;
use crate::player::{air_shit, special_attacks, CharacterStatus, AIR_FRICTION, AIR_JUMP_BASE_IMPULSE, AIR_SPEED};
use super::types::{AirJumpsRemaining, Facing, PlayerState, StateTicks};

const GROUND_FRICTION: f32 = 0.6;

// TODO: variable jump height + short hops (releasing jump before the pre-jump animation returns control for VJH)


use PlayerState::*;

pub fn update_playerstate_physics(mut query: Query<(&PlayerState, &StateTicks, &RawInput, &AirJumpsRemaining, &Facing, &mut Velocity, &mut CharacterStatus)>) {
    for (mut state, state_ticks, raw_input, air_jumps, facing, mut velocity, mut status) in &mut query {
        match &*state {
            state if state.has_ground_physics() => {
                status.busy = false;
                status.no_jump = false;

                if raw_input.stick.x.abs() >= 0.25 {
                    if velocity.linear.x < 40.0 && velocity.linear.x > -40.0 {
                        velocity.linear.x *= GROUND_FRICTION;
                        velocity.linear.x += raw_input.stick.x * 28.;
                    } else {
                        // velocity.linear.x *= AIR_FRICTION;
                        // velocity.linear.x += raw_input.stick.x * 40.;
                    }
                }
            }
            Jumping => {
                if state_ticks.0 == 0 {
                    status.busy = true;
                    status.no_jump = true;

                    velocity.linear.y = AIR_JUMP_BASE_IMPULSE;

                    // if stick dir != velocity dir -- but oh shit frame of reference might be an issue down the line if we add windspeed... ohwell.
                    if velocity.linear.x * raw_input.stick.x > 0. {
                        velocity.linear.x = 80. * raw_input.stick.x ;
                    } else {
                        velocity.linear.x += 30. * raw_input.stick.x ;
                    }
                }

                if state_ticks.0 >= 10 {
                    status.busy = false;
                }

                if state_ticks.0 >= 100 || velocity.linear.y < 30.0 {
                    status.no_jump = false;
                }
                aerial_movement(&raw_input, &mut velocity);
            }
            AirJump => {
                status.busy = true;
                status.no_jump = true;
                air_shit::air_jump_phys(state, &state_ticks, &raw_input, air_jumps, &mut velocity, &mut status);

                if state_ticks.0 >= 6 {
                    status.busy = false;
                }
                if state_ticks.0 >= 20 {
                    status.no_jump = false;
                }
            },
            UpAir | DownAir | FwdAir | BackAir | NeutralAir => {
                status.busy = true;
                status.no_jump = true;
                aerial_movement(&raw_input, &mut velocity);
            },
            ControlledFall => {
                status.busy = false;
                status.no_jump = false;
                aerial_movement(&raw_input, &mut velocity);
            }
            SpinMove => {
                status.busy = true;
                status.no_jump = true;
                velocity.linear.x *= 0.98;
                velocity.linear.y *= GROUND_FRICTION;
            }
            ChargedPunch => {
                status.busy = true;
                status.no_jump = true;
                special_attacks::side_special_phys(state, state_ticks, raw_input, *facing, &mut velocity);

                if state_ticks.0 > 80 {
                    status.busy = false;
                    status.no_jump = false;
                }
            }
            GroundPound => {
                status.busy = true;
                status.no_jump = true;

                velocity.linear.x *= 0.98;
                velocity.linear.y *= GROUND_FRICTION;
            }
            _ => {
                status.busy = false;
                status.no_jump = false;
            }
        }
    }
}

pub(crate) fn aerial_movement(raw_input: &RawInput, velocity: &mut Velocity) {
    let apex_boost = (1.0 - velocity.linear.y.abs() / 20.0).clamp(0.0, 1.0);

    velocity.linear.x *= AIR_FRICTION;
    velocity.linear.x += raw_input.stick.x * AIR_SPEED * (1.0 + 0.4 * apex_boost);
}
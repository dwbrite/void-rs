use bevy::prelude::Query;
use bevy::ecs::query::QueryData;
use bevy_rapier2d::dynamics::Velocity;
use bevy_rapier2d::prelude::GravityScale;
use crate::input::InputAction;
use crate::player::{air_shit, special_attacks, CharacterStatus, AIR_FRICTION, AIR_JUMP_BASE_IMPULSE, AIR_SPEED};
use crate::systems::input::ActionMap;
use super::types::{AirJumpsRemaining, Facing, PlayerState, StateTicks, AirborneState, SpringMass};

const GROUND_FRICTION: f32 = 0.6;
const HELD_ASCENT_GRAVITY: f32 = 0.8;  // 0.07 / 0.28
const FLOAT_FALL_GRAVITY:  f32 = 0.95;  // 0.045 / 0.28
const GROUND_TARGET_SPEED: f32 = 48.0;
const LANDING_BLEND_TICKS: f32 = 500.0;   // frames to fully "plant"
const LANDING_GRIP: f32 = 0.0001;         // per-tick pull toward target right at touchdown

const SPIN_GROUND_ACCEL: f32 = 2.8;      // per-tick velocity gain toward stick dir
const SPIN_GROUND_MAX_SPEED: f32 = 80.0; // spin can out-speed running (48) — that's the point
const SPIN_AIR_RAMP_SECS_TICKS: f32 = 128.0; // ~1s at 60Hz to full heaviness
const SPIN_AIR_X_DECAY: f32 = 0.994;      // horizontal bleed per tick while air-spinning
const SPIN_CATCH_RATE: f32 = 0.16;   // per-tick approach toward target sink; the "catch" softness
const SPIN_FLOAT_SINK: f32 = 0.1;    // sink speed during the float phase
const SPIN_END_SINK:  f32 = 220.0;    // sink speed at end of ramp (≈ your normal terminal vel)
const SPIN_CATCH_RAMP_TICKS: f32 = 20.0; // how long the catch takes to reach full grip

use PlayerState::*;
use crate::player::state::AirborneState::Grounded;
use crate::player::state::spin_input_speed;

fn old_school_gravity(status: &CharacterStatus, velocity: &Velocity, gravity: &mut GravityScale) {
    gravity.0 = if !status.holding_jump {
        1.0
    } else if velocity.linear.y > 0.0 {
        HELD_ASCENT_GRAVITY   // rising + held: extend the jump
    } else {
        FLOAT_FALL_GRAVITY    // fell past apex still holding: feather fall
    };
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct PlayerPhysicsQuery {
    state: &'static PlayerState,
    state_ticks: &'static StateTicks,
    airborne: &'static AirborneState,
    velocity: &'static mut Velocity,
    facing: &'static Facing,
    status: &'static mut CharacterStatus,
    air_jumps: &'static AirJumpsRemaining,
    spring_mass: &'static SpringMass,
    gravity: &'static mut GravityScale,
}

pub fn update_playerstate_physics(
    input_map: bevy::prelude::Res<ActionMap<InputAction>>,
    mut query: Query<PlayerPhysicsQuery>,
) {

    let move_x = input_map.value(InputAction::MoveX);
    let move_y = input_map.value(InputAction::MoveY);
    for mut p in &mut query {
        match p.airborne {
            AirborneState::Grounded => {
                p.status.ticks_since_landed = p.status.ticks_since_landed.saturating_add(1);
                p.status.spin_float_used = false;   // touching ground re-arms the float
            }
            AirborneState::Airborne => p.status.ticks_since_landed = 0,
        }

        let di_multiplier = match p.state {
            UpAir => (1.0, 1.0),
            DownKick => (0.8, 0.8),
            FwdAir => (0.9, 1.0),
            BackAir => (1.3, 1.0),
            NeutralAir => (1.0, 1.0),
            PreJump => (1.3, 1.0),
            _ => (1.0, 1.0),
        };

        match p.state {
            state if state.has_ground_physics() => {
                p.status.busy = false;
                p.status.no_jump = false;

                // 0.0 = just landed, 1.0 = fully planted
                let t = (p.status.ticks_since_landed as f32 / LANDING_BLEND_TICKS).clamp(0.0, 1.0);

                if move_x.abs() >= 0.25 {
                    let target = move_x * GROUND_TARGET_SPEED;
                    let same_dir = target * p.velocity.linear.x > 0.0;
                    let carrying_speed = p.velocity.linear.x.abs() > target.abs();

                    let grip = if same_dir && carrying_speed {
                        // landed with momentum in the direction you're pushing:
                        // bleed down gently instead of snapping to 48
                        LANDING_GRIP + (1.0 - LANDING_GRIP) * t
                    } else {
                        // reversing, or slower than ground speed: full grip.
                        // turns stay crisp, slow landings don't feel mushy.
                        1.0
                    };

                    p.velocity.linear.x += (target - p.velocity.linear.x) * grip;
                } else {
                    p.velocity.linear.x *= 0.90;
                }
            }
            SuperJump => {
                if p.state_ticks.0 == 10 {
                    p.status.busy = true;
                    p.status.no_jump = true;

                    p.velocity.linear.y = AIR_JUMP_BASE_IMPULSE*2.0;

                    // Prioritize player intent on jump start to avoid carrying stale ground momentum.
                    p.velocity.linear.x = 120. * move_x;
                }

                if p.state_ticks.0 >= 15 {
                    p.status.busy = false;
                }

                if p.state_ticks.0 >= 100 || p.velocity.linear.y < 30.0 {
                    p.status.no_jump = false;
                }
                aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
            }
            PreJump => {
                p.status.holding_jump = true;
                aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
            }
            Jumping => {
                if p.state_ticks.0 == 0 {
                    let intent = 80. * move_x;
                    p.velocity.linear.x = if intent * p.velocity.linear.x > 0.0
                        && p.velocity.linear.x.abs() > intent.abs()
                    {
                        p.velocity.linear.x   // already faster in the intended direction: keep it
                    } else {
                        intent              // standstill / reversal: old behavior
                    };

                    if p.status.holding_jump {
                        let spring_mult = 1.0 + (p.spring_mass.vy / 11.0);
                        p.velocity.linear.y = AIR_JUMP_BASE_IMPULSE * 0.8 * spring_mult; // full impulse, frame 0
                    } else {
                        println!("short-hop");
                        p.velocity.linear.y = AIR_JUMP_BASE_IMPULSE * 0.65; // full impulse, frame 0
                    }
                }
                if p.state_ticks.0 >= 4 {
                    p.status.busy = false;
                }
                old_school_gravity(&p.status, &p.velocity, &mut p.gravity);
                if p.state_ticks.0 >= 100 || p.velocity.linear.y < 30.0 {
                    p.status.no_jump = false;
                }
                p.velocity.linear.y = p.velocity.linear.y.max(-AIR_JUMP_BASE_IMPULSE * 1.4); // terminal vel
                aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
            }
            AirJump => {
                p.status.busy = true;
                p.status.no_jump = true;
                air_shit::air_jump_phys(p.state, p.state_ticks, move_x, move_y, p.air_jumps, &mut p.velocity, &mut p.status);

                if p.state_ticks.0 >= 6 {
                    p.status.busy = false;
                }
                if p.state_ticks.0 >= 20 {
                    p.status.no_jump = false;
                }
            },
            PreDownKick => {
                p.status.busy = true;
                p.status.no_jump = true;

                aerial_x_movement(move_x, &mut p.velocity, (0.4, 1.0));
            }
            DownKick => {
                if p.state_ticks.0 == 0 {
                    p.status.busy = true;
                    p.status.no_jump = true;

                    // TODO: did_hit, and make it check for the entire durations
                    let did_hit = false;
                    if did_hit || matches!(p.airborne, Grounded) {
                        p.velocity.linear.y = AIR_JUMP_BASE_IMPULSE * 0.9;
                    }
                } else if p.state_ticks.0 >= 6 {
                    p.status.busy = false;
                    p.status.no_jump = false;
                }
            }
            UpAir | FwdAir | BackAir | NeutralAir => {
                p.status.busy = true;
                p.status.no_jump = true;

                if *p.state == UpAir && *p.airborne == AirborneState::Grounded {
                    // fake it till you make it lmao
                    air_shit::air_jump_phys(p.state, p.state_ticks, move_x, move_y, &AirJumpsRemaining(1), &mut p.velocity, &mut p.status);
                } else {
                    match p.airborne {
                        AirborneState::Airborne => aerial_x_movement(move_x, &mut p.velocity, di_multiplier),
                        AirborneState::Grounded => aerial_x_movement(move_x, &mut p.velocity, (0.8, 1.0)),
                    }
                }
            },
            ControlledAirborne => {
                p.status.busy = false;
                p.status.no_jump = false;
                old_school_gravity(&p.status, &p.velocity, &mut p.gravity);
                aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
            }
            SmashDrop => {
                p.status.no_jump = false;
                if p.state_ticks.0 == 0 {
                    p.status.busy = true;
                } else if p.state_ticks.0 >= 8 {
                    p.status.busy = false;
                }

                if p.velocity.linear.y > -AIR_JUMP_BASE_IMPULSE * 1.0 {
                    p.velocity.linear.y = -AIR_JUMP_BASE_IMPULSE * 1.0;
                }
                old_school_gravity(&p.status, &p.velocity, &mut p.gravity);
            }
            SpinMove(_) => {
                p.status.busy = true;
                p.status.no_jump = true;

                // Entry edge: eligibility decided once, here.
                if !p.status.was_spinning {
                    p.status.spin_can_float = !p.status.spin_float_used;
                    p.status.spin_fall_ticks = 0;
                }

                let live_spin = input_map.value(InputAction::Spin);

                match p.airborne {
                    AirborneState::Grounded => {
                        let max_speed = SPIN_GROUND_MAX_SPEED + 24.0 * (1.0 + (spin_input_speed(&input_map) * 1.5).clamp(0.0, 1.0));

                        p.gravity.0 = 1.0;
                        let accel = SPIN_GROUND_ACCEL * live_spin.clamp(0.2, 1.5);
                        p.velocity.linear.x += move_x * accel;
                        p.velocity.linear.x = p.velocity.linear.x
                            .clamp(-SPIN_GROUND_MAX_SPEED, max_speed);
                        if move_x.abs() < 0.25 {
                            p.velocity.linear.x *= 0.985;
                        }
                    }
                    AirborneState::Airborne => {
                        if p.status.spin_can_float {
                            p.status.spin_float_used = true;

                            if p.velocity.linear.y > 0.0 {
                                // Still rising: the float doesn't touch ascent. Normal gravity
                                // bends the jump over naturally; the catch waits at the apex.
                                p.gravity.0 = 1.0;
                            } else {
                                // Descending: float physics. All timers are fall-relative —
                                // spinning mid-ascent doesn't pre-burn the catch or the hover.
                                p.gravity.0 = 0.0;
                                p.status.spin_fall_ticks = p.status.spin_fall_ticks.saturating_add(1);

                                let t = (p.status.spin_fall_ticks as f32 / SPIN_AIR_RAMP_SECS_TICKS).clamp(0.0, 1.0);
                                let c = (p.status.spin_fall_ticks as f32 / SPIN_CATCH_RAMP_TICKS).clamp(0.0, 1.0);
                                let rate = SPIN_CATCH_RATE * (c * c);

                                let target_sink = -(SPIN_FLOAT_SINK + (SPIN_END_SINK - SPIN_FLOAT_SINK) * t * t);
                                p.velocity.linear.y += (target_sink - p.velocity.linear.y) * rate;
                            }
                        } else {
                            p.gravity.0 = 1.5;
                        }
                        p.velocity.linear.x *= SPIN_AIR_X_DECAY;
                    }
                }
            }
            ChargedPunch => {
                p.status.busy = true;
                p.status.no_jump = true;
                special_attacks::side_special_phys(p.state, p.state_ticks, *p.facing, move_x, move_y, &mut p.velocity);

                if p.state_ticks.0 > 70 {
                    p.status.busy = false;
                }
                if p.state_ticks.0 > 80 {
                    p.status.no_jump = false;
                }
            }
            GroundPound => {
                if p.state_ticks.0 == 0 {
                    p.status.busy = true;
                    p.status.no_jump = true;
                }
                if p.state_ticks.0 > 40 {
                    p.status.busy = false;
                }

                // yay magic number :3
                p.velocity.linear.x *= 0.92;

                p.velocity.linear.y *= 0.999;
                p.velocity.linear.y -= 12.0;
            }
            Interact => {
                p.status.busy = true;
            }
            Interactnt => {
                p.status.busy = false;
            }
            _ => {
                p.status.busy = false;
                p.status.no_jump = false;
            }
        }
        p.status.was_spinning = matches!(p.state, SpinMove(_));
    }
}

pub(crate) fn aerial_x_movement(move_x: f32, velocity: &mut Velocity, di_multiplier: (f32, f32)) {
    let apex_boost = (1.0 - velocity.linear.y.abs() / 20.0).clamp(0.0, 1.0);

    velocity.linear.x *= AIR_FRICTION;
    velocity.linear.x += move_x * di_multiplier.0 * AIR_SPEED * (1.0 + 0.4 * apex_boost);
}

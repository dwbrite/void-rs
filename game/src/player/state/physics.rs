use avian2d::prelude::{GravityScale, LinearVelocity};
use bevy::prelude::Query;
use bevy::ecs::query::QueryData;
use crate::input::InputAction;
use crate::player::{air_shit, special_attacks, CharacterStatus, AIR_JUMP_BASE_IMPULSE};
use crate::systems::input::ActionMap;
use super::types::{AirJumpsRemaining, Facing, PlayerState, StateTicks, AirborneState, SpringMass};

const GROUND_FRICTION: f32 = 0.6;
const HELD_ASCENT_GRAVITY: f32 = 0.8;  // 0.07 / 0.28
const FLOAT_FALL_GRAVITY:  f32 = 0.95;  // 0.045 / 0.28
const GROUND_SPEED: f32 = 48.0;
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
use crate::player::state::{spin_input_speed, PreviousState};

fn old_school_gravity(status: &CharacterStatus, velocity: &LinearVelocity, gravity: &mut GravityScale) {
    gravity.0 = if !status.holding_jump {
        1.0
    } else if velocity.y > 0.0 {
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
    velocity: &'static mut LinearVelocity,
    facing: &'static Facing,
    status: &'static mut CharacterStatus,
    air_jumps: &'static AirJumpsRemaining,
    spring_mass: &'static SpringMass,
    gravity: &'static mut GravityScale,
    old_state: &'static mut PreviousState,
}

const GROUND_DASH_SPEED: f32 = 72.0;

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
            PreJump => (1.3, 1.0),
            _ => (1.0, 1.0),
        };

        match p.state {
            state if state.has_ground_physics() => {
                p.status.busy = false;
                p.status.no_jump = false;

                // 0.0 = just landed, 1.0 = fully planted
                let t = (p.status.ticks_since_landed as f32 / LANDING_BLEND_TICKS).clamp(0.0, 1.0);

                // move 64 to run speed target minimum
                let target_speed = if p.velocity.x.abs() > 48.0 {
                    move_x * GROUND_DASH_SPEED
                } else {
                    move_x * GROUND_SPEED
                };

                if move_x.abs() >= 0.25 {
                    let target = target_speed;
                    let same_dir = target * p.velocity.x > 0.0;
                    let carrying_speed = p.velocity.x.abs() > target.abs();

                    let grip = if same_dir && carrying_speed {
                        // landed with momentum in the direction you're pushing:
                        // bleed down gently instead of snapping to 48
                        LANDING_GRIP + (1.0 - LANDING_GRIP) * t
                    } else {
                        // reversing, or slower than ground speed: full grip.
                        // turns stay crisp, slow landings don't feel mushy.
                        1.0
                    };

                    p.velocity.x += (target - p.velocity.x) * grip;
                } else {
                    p.velocity.x *= 0.90;
                }
            }
            FlickDash => {
                if move_x >= 0.0 {
                    p.velocity.x = GROUND_DASH_SPEED;
                } else {
                    p.velocity.x = -GROUND_DASH_SPEED;
                }
            }
            Slide => {
                p.status.busy = false;
                p.status.no_jump = false;
                ground_friction(&mut p.velocity, SLIDE_MU, SLIDE_DRAG);
            }
            DashFlop | DashFlop2 => {
                p.status.busy = false;
                p.status.no_jump = false;
                ground_friction(&mut p.velocity, TRIP_MU, TRIP_DRAG);
            }
            SuperJump => {
                if p.state_ticks.0 == 10 {
                    p.status.busy = true;
                    p.status.no_jump = true;

                    p.velocity.y = AIR_JUMP_BASE_IMPULSE*2.0;

                    // Prioritize player intent on jump start to avoid carrying stale ground momentum.
                    p.velocity.x = 120. * move_x;
                }

                if p.state_ticks.0 >= 15 {
                    p.status.busy = false;
                }

                if p.state_ticks.0 >= 100 || p.velocity.y < 30.0 {
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
                    p.velocity.x = if intent * p.velocity.x > 0.0
                        && p.velocity.x.abs() > intent.abs()
                    {
                        p.velocity.x   // already faster in the intended direction: keep it
                    } else {
                        intent              // standstill / reversal: old behavior
                    };

                    if p.status.holding_jump {
                        let spring_mult = 1.0 + (p.spring_mass.vy / 11.0);
                        p.velocity.y = AIR_JUMP_BASE_IMPULSE * 0.8 * spring_mult; // full impulse, frame 0
                    } else {
                        println!("short-hop");
                        p.velocity.y = AIR_JUMP_BASE_IMPULSE * 0.65; // full impulse, frame 0
                    }
                }
                if p.state_ticks.0 >= 4 {
                    p.status.busy = false;
                }
                old_school_gravity(&p.status, &p.velocity, &mut p.gravity);
                if p.state_ticks.0 >= 100 || p.velocity.y < 30.0 {
                    p.status.no_jump = false;
                }
                p.velocity.y = p.velocity.y.max(-AIR_JUMP_BASE_IMPULSE * 1.4); // terminal vel
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
                if p.airborne == &Grounded {
                    aerial_x_movement(move_x, &mut p.velocity, (0.05, 1.0));
                } else {
                    aerial_x_movement(move_x, &mut p.velocity, (1.0, 1.0));
                }
            }
            DownKick => {
                if p.state_ticks.0 == 0 {
                    p.status.busy = false;
                    p.status.no_jump = false;
                } else if p.state_ticks.0 <= 24 {
                    // TODO: did_hit, and make it check for the entire durations
                    let did_hit = false;
                    if did_hit || matches!(p.airborne, Grounded) {
                        p.velocity.y = AIR_JUMP_BASE_IMPULSE * 0.75;
                    }

                    p.status.busy = false;
                    p.status.no_jump = false;
                }
                aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
            }
            UpAir | FwdAir | BackAir => {
                p.status.busy = true;
                p.status.no_jump = true;

                if *p.state == UpAir && *p.airborne == AirborneState::Grounded {
                    // fake it till you make it lmao
                    if &p.old_state.0 == &DashFlop {
                        air_shit::air_jump_phys(p.state, p.state_ticks, move_x, move_y, &AirJumpsRemaining(0), &mut p.velocity, &mut p.status);
                    } else {
                        air_shit::air_jump_phys(p.state, p.state_ticks, move_x, move_y, &AirJumpsRemaining(2), &mut p.velocity, &mut p.status);
                    }
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
            FlickDrop => {
                p.status.no_jump = false;
                if p.state_ticks.0 == 0 {
                    p.status.busy = true;
                } else if p.state_ticks.0 >= 8 {
                    p.status.busy = false;
                }

                if p.velocity.y > -AIR_JUMP_BASE_IMPULSE * 1.0 {
                    p.velocity.y = -AIR_JUMP_BASE_IMPULSE * 1.0;
                }
                old_school_gravity(&p.status, &p.velocity, &mut p.gravity);
            }
            SpinMove(_) => {
                p.status.busy = true;
                p.status.no_jump = true;

                // Entry edge: eligibility decided once, here.
                if !p.status.was_spinning {
                    p.status.spin_can_float = true; // formerly !p.status.spin_float_used;
                    p.status.spin_fall_ticks = 0;
                }

                let live_spin = input_map.value(InputAction::Spin);
                const SPIN_GROUND_OVERCAP_FRICTION: f32 = 0.5; // per tick; carried speed eases to cap, ~64 u/s

                match p.airborne {

                    AirborneState::Grounded => {
                        let max_speed = SPIN_GROUND_MAX_SPEED
                            + 24.0 * (1.0 + (spin_input_speed(&input_map) * 1.5).clamp(0.0, 1.0));

                        p.gravity.0 = 1.0;
                        let accel = SPIN_GROUND_ACCEL * live_spin.clamp(0.2, 1.5);
                        let v = p.velocity.x;

                        // Accel only pushes you up to the cap — never reduces over-cap speed.
                        // (Same .max(v) trick as the air code: the clamp can't pull you down.)
                        let nv = if move_x > 0.0 {
                            (v + move_x * accel).min(max_speed.max(v))
                        } else if move_x < 0.0 {
                            (v + move_x * accel).max((-max_speed).min(v))
                        } else {
                            v
                        };
                        p.velocity.x = nv;

                        // Carried over-cap momentum bleeds gently toward the cap instead of snapping.
                        if p.velocity.x.abs() > max_speed {
                            let target = max_speed * p.velocity.x.signum();
                            let d = p.velocity.x - target;
                            p.velocity.x = if d.abs() <= SPIN_GROUND_OVERCAP_FRICTION {
                                target
                            } else {
                                p.velocity.x - SPIN_GROUND_OVERCAP_FRICTION * d.signum()
                            };
                        }

                        if move_x.abs() < 0.25 {
                            p.velocity.x *= 0.985;
                        }
                    }
                    AirborneState::Airborne => {
                        if p.status.spin_can_float {
                            p.status.spin_float_used = true;

                            if p.velocity.y > 0.0 {
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
                                p.velocity.y += (target_sink - p.velocity.y) * rate;
                            }
                        } else {
                            p.gravity.0 = 1.5;
                        }
                        aerial_x_movement(move_x, &mut p.velocity, di_multiplier);
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
                p.velocity.x *= 0.92;

                p.velocity.y *= 0.999;
                p.velocity.y -= 12.0;
            }
            Interact => {
                p.status.busy = true;
            }
            Interactnt => {
                p.status.busy = true;
            }
            _ => {
                p.status.busy = false;
                p.status.no_jump = false;
                ground_friction(&mut p.velocity, 10.0, 0.005);
            }
        }
        p.status.was_spinning = matches!(p.state, SpinMove(_));
    }
}

const AIR_MAX_SPEED: f32 = 86.0;         // was 76; also gives real headroom over the 72 slide gate
const AIR_ACCEL: f32 = 7.0;
const AIR_FRICTION_LIN: f32 = 1.3;       // neutral bleed, unchanged
const AIR_HELD_OVERCAP_FRICTION: f32 = 0.35; // over-cap bleed while holding into it (~45 u/s)
const CAP_SOFT_BAND: f32 = 14.0;         // accel fades over the last 14 units below cap

const SLIDE_MU: f32 = 0.35;
const SLIDE_DRAG: f32 = 8.0e-5;
const TRIP_MU: f32 = 0.6;
const TRIP_DRAG: f32 = 1.3e-4;

/// `mu`   — flat per-tick decel. Guarantees termination, sets the low-speed feel.
/// `drag` — quadratic term. Top-end tax only; ~vanishes below 24 u/s.
fn ground_friction(velocity: &mut LinearVelocity, mu: f32, drag: f32) {
    let v = velocity.x;
    if v == 0.0 { return; }
    let speed = v.abs() - (mu + drag * v * v);
    velocity.x = if speed <= 0.0 { 0.0 } else { speed * v.signum() };
}

pub(crate) fn aerial_x_movement(move_x: f32, velocity: &mut LinearVelocity, di: (f32, f32)) {
    let v = velocity.x;
    // di scales authority, not the ceiling. >1 (BackAir 1.3) still raises it — that's a perk.
    let cap = AIR_MAX_SPEED * di.0.max(1.0);
    let accel = move_x * AIR_ACCEL * di.0;
    let neutral = move_x.abs() < 0.15;
    let pushing_along = move_x * v > 0.0;

    let mut nv = v;

    // Accel: full strength on reversal / from rest; tapers smoothly to zero
    // over CAP_SOFT_BAND approaching the cap. No hard clip anymore.
    if !neutral {
        let gain = if pushing_along {
            ((cap - v.abs()) / CAP_SOFT_BAND).clamp(0.0, 1.0)
        } else {
            1.0
        };
        nv += accel * gain;
    }

    // Friction, all linear:
    //   over cap + holding into it  → gentle bleed toward cap
    //   over cap + neutral/reverse  → full bleed toward cap
    //   below cap + neutral         → full bleed toward 0 (unchanged feel)
    let over = nv.abs() > cap;
    if over {
        let fric = if pushing_along && !neutral { AIR_HELD_OVERCAP_FRICTION } else { AIR_FRICTION_LIN };
        let target = cap * nv.signum();
        let d = nv - target;
        nv = if d.abs() <= fric { target } else { nv - fric * d.signum() };
    } else if neutral {
        nv = if nv.abs() <= AIR_FRICTION_LIN { 0.0 } else { nv - AIR_FRICTION_LIN * nv.signum() };
    }

    velocity.x = nv;
}
use std::collections::HashSet;
use bevy::math::ops::abs;
use bevy::prelude::{Changed, Entity, Query, Res, Sprite};
use bevy::ui::State;
use bevy_aseprite_ultra::prelude::{Animation, AnimationDirection, AnimationEvents, AnimationRepeat, AseAnimation};
use bevy_aseprite_ultra::prelude::AnimationRepeat::{Count, Loop};
use bevy_rapier2d::prelude::Velocity;
use crate::input::InputAction;
use crate::player::AIR_SPEED;
use crate::player::state::{Facing, PlayerState, SpringMass, StateTicks};
use crate::systems::input::ActionMap;

pub fn flip_sprite(mut query: Query<(&Facing, &mut Sprite), Changed<Facing>>) {
    for (facing, mut sprite) in &mut query {
        sprite.flip_x = *facing == Facing::Left;
    }
}

pub fn playerstate_animation(
    input_map: Res<ActionMap<InputAction>>,
    mut query: Query<(&PlayerState, &mut AseAnimation, &Velocity, &StateTicks, &SpringMass)>,
) {
    let move_y = input_map.value(InputAction::MoveY);
    for (playerstate, mut animation, velocity, ticks, spring) in &mut query {
        match playerstate {
            PlayerState::Idle | PlayerState::Running => {
                match spring.y {
                    y if y >= -10.0 && playerstate == &PlayerState::Running => { animation.animation = Animation::tag("run").with_speed(abs(velocity.linear.x / 56.0)); }
                    y if y > 5.0 => { animation.animation = Animation::tag("lookup1"); }
                    y if y < -50.0 => { animation.animation = Animation::tag("pre-supercrouch"); }
                    y if y < -45.0 => { animation.animation = Animation::tag("hardcrouch"); }
                    y if y < -40.0 => { animation.animation = Animation::tag("crouch"); }
                    y if y < -35.0 => { animation.animation = Animation::tag("half-crouch"); }
                    y if y < -30.0 => { animation.animation = Animation::tag("qtr-crouch"); }
                    y if y < -10.0 => { animation.animation = Animation::tag("lookdown"); }
                    _ => { animation.animation = Animation::tag("idle"); }
                }
            },
            PlayerState::Lookup => {
                match move_y {
                    y if y > 0.96 => { animation.animation = Animation::tag("lookup4"); }
                    y if y > 0.90 => { animation.animation = Animation::tag("lookup3"); }
                    y if y > 0.60 => { animation.animation = Animation::tag("lookup2"); }
                    y if y > 0.30 => { animation.animation = Animation::tag("lookup1"); }
                    _ => { /* not in look up animation */ }
                }
            },
            PlayerState::GroundPound => if ticks.0 == 0 {
                animation.animation = Animation::tag("groundpound").with_repeat(Count(0));
            }
            PlayerState::Crouch => {
                match move_y {
                    y if y < -0.90 => { animation.animation = Animation::tag("pre-supercrouch"); animation.animation.pause(); }
                    y if y < -0.70 => { animation.animation = Animation::tag("hardcrouch"); }
                    y if y < -0.65 => { animation.animation = Animation::tag("crouch"); }
                    y if y < -0.55 => { animation.animation = Animation::tag("half-crouch"); }
                    y if y < -0.45 => { animation.animation = Animation::tag("qtr-crouch"); }
                    y if y < -0.25 => { animation.animation = Animation::tag("lookdown"); }
                    _ => {}
                }
            },
            PlayerState::SuperCrouch => {
                match ticks.0 {
                    0 => {
                        animation.animation = Animation::tag("pre-supercrouch").with_repeat(Count(0));
                    }
                    t if t >= 45 => {
                        animation.animation = Animation::tag("supercrouch").with_repeat(AnimationRepeat::Loop);
                    }
                    _ => {}
                }
            },
            PlayerState::BackAir if ticks.0 == 0 => {
                animation.animation = Animation::tag("air back kick").with_repeat(Count(0)).with_then("abk-hit", Count(1)).with_then("abk-rec", Count(0));
            },
            PlayerState::FwdAir => { // TODO: find out if fwd air is... supposed to be... resettable?
                animation.animation = Animation::tag("air kick").with_repeat(Count(0));
            }
            PlayerState::DownAir if ticks.0 == 0 => {
                animation.animation = Animation::tag("downair").with_repeat(Count(2));
            }
            PlayerState::SpinMove if ticks.0 == 0 => {
                animation.animation = Animation::tag("downair").with_repeat(Loop);
            }
            PlayerState::NeutralAir => {
                animation.animation = Animation::tag("basic punch").with_repeat(Count(0));
            }
            PlayerState::Interact => {
                animation.animation = Animation::tag("interact").with_repeat(Loop);
            }
            PlayerState::Interactnt => if ticks.0 == 0 {
                animation.animation = Animation::tag("basic punch rec").with_repeat(Count(0));
            }
            PlayerState::NeutralAir => {
                animation.animation = Animation::tag("basic punch").with_repeat(Count(0));
            }
            PlayerState::UpAir if ticks.0 == 0 => {
                animation.animation = Animation::tag("upkick").with_repeat(Count(0));
            }
            PlayerState::ControlledAirborne => {
                match velocity.linear.y {
                    y if y > 110.0 => animation.animation = Animation::tag("jumpfast"),
                    y if y > 100.0 => animation.animation = Animation::tag("jump2"),

                    y if y < -80. => animation.animation = Animation::tag("fast fall"),
                    y if y < -20. => animation.animation = Animation::tag("fall2"),
                    y if y < -10.0 => animation.animation = Animation::tag("fall1"),
                    y if y <  0.0 => animation.animation = Animation::tag("fall-tween"),
                    y if y <  20. => animation.animation = Animation::tag("jump_apex"),
                    _ => {}
                }
            }
            PlayerState::SuperJump if ticks.0 == 0 => {
                animation.animation = Animation::tag("superjump").with_repeat(AnimationRepeat::Count(0));
            }
            PlayerState::Jumping if ticks.0 == 0 => {
                animation.animation = Animation::tag("jumpfast").with_repeat(AnimationRepeat::Count(0));
            }
            PlayerState::AirJump if ticks.0 == 0 => {
                println!("started air jump anim");
                animation.animation = Animation::tag("jumpfast").with_repeat(Count(0)).with_then("jump2", Count(0));
            },
            PlayerState::ChargedPunch if ticks.0 == 0 => {
                animation.animation = Animation::tag("air punch").with_repeat(AnimationRepeat::Count(0)).with_then("air-punch-fin", AnimationRepeat::Count(10)).with_then("air-punch-recovery", AnimationRepeat::Count(0));
            },
            _ => continue,
        }
    }
}
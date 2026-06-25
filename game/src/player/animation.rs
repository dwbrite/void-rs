use bevy::math::ops::abs;
use bevy::prelude::{Changed, Query, Sprite};
use bevy::ui::State;
use bevy_aseprite_ultra::prelude::{Animation, AnimationRepeat, AseAnimation};
use bevy_aseprite_ultra::prelude::AnimationRepeat::Count;
use bevy_rapier2d::prelude::Velocity;
use crate::input::RawInput;
use crate::player::AIR_SPEED;
use crate::player::state::{Facing, PlayerState, StateTicks};

pub fn flip_sprite(mut query: Query<(&Facing, &mut Sprite), Changed<Facing>>) {
    for (facing, mut sprite) in &mut query {
        sprite.flip_x = *facing == Facing::Left;
    }
}

pub fn playerstate_animation(mut query: Query<(&PlayerState, &mut AseAnimation, &Velocity, &StateTicks, &RawInput)>) {
    for (playerstate, mut animation, velocity, ticks, raw_input) in &mut query {
        match playerstate {
            PlayerState::Idle => {
                animation.animation = Animation::tag("idle");
            },
            PlayerState::Lookup => {
                match raw_input.stick.y {
                    y if y > 0.96 => { animation.animation = Animation::tag("lookup4"); }
                    y if y > 0.90 => { animation.animation = Animation::tag("lookup3"); }
                    y if y > 0.60 => { animation.animation = Animation::tag("lookup2"); }
                    y if y > 0.30 => { animation.animation = Animation::tag("lookup1"); }
                    _ => { /* not in look up animation */ }
                }
            },
            PlayerState::GroundPound => {
                // animation.animation = Animation::tag("downair");
            }
            PlayerState::Crouch => {
                match raw_input.stick.y {
                    y if y < -0.90 => { animation.animation = Animation::tag("pre-supercrouch"); animation.animation.pause(); }
                    y if y < -0.80 => { animation.animation = Animation::tag("hardcrouch"); }
                    y if y < -0.60 => { animation.animation = Animation::tag("crouch"); }
                    y if y < -0.30 => { animation.animation = Animation::tag("half-crouch"); }
                    _ => {}
                }
            },
            PlayerState::SuperCrouch if ticks.0 == 0 => {
                animation.animation = Animation::tag("pre-supercrouch").with_repeat(Count(0)).with_then("supercrouch", AnimationRepeat::Loop);
            },
            PlayerState::Running => {
                animation.animation = Animation::tag("run").with_speed(abs(velocity.linear.x / 96.0));
            },
            PlayerState::BackAir if ticks.0 == 0 => {
                animation.animation = Animation::tag("air back kick").with_repeat(Count(0)).with_then("abk-hit", Count(1)).with_then("abk-rec", Count(0));
            },
            PlayerState::FwdAir => {
                animation.animation = Animation::tag("air kick").with_repeat(Count(0));
            }
            PlayerState::DownAir if ticks.0 == 0 => {
                animation.animation = Animation::tag("downair").with_repeat(Count(2));
            }
            PlayerState::ControlledFall => {
                match velocity.linear.y {
                    y if y > 110.0 => animation.animation = Animation::tag("jumpfast"),
                    y if y > 100.0 => animation.animation = Animation::tag("jump2"),

                    y if y < -80. => animation.animation = Animation::tag("fast fall"),
                    y if y < 0.0 => animation.animation = Animation::tag("fall"),
                    y if y < 20.0 => animation.animation = Animation::tag("jump_apex"),

                    _ => {}
                }
            },
            PlayerState::Jumping => {
                // TODO: maybe move these to controlled fall, oop
                match velocity.linear.y {
                    _ if ticks.0 == 0 =>  animation.animation = Animation::tag("jump").with_repeat(AnimationRepeat::Count(0)),
                    _ => {}
                }
            }
            PlayerState::AirJump if ticks.0 == 0 => {
                println!("started air jump anim");
                animation.animation = Animation::tag("jump").with_repeat(AnimationRepeat::Count(0));
            },
            PlayerState::ChargedPunch if ticks.0 == 0 => {
                animation.animation = Animation::tag("air punch").with_repeat(AnimationRepeat::Count(0)).with_then("air-punch-fin", AnimationRepeat::Count(10)).with_then("air-punch-recovery", AnimationRepeat::Count(0));
            },
            _ => continue,
        }
    }
}
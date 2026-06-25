use bevy::prelude::Mut;
use bevy_rapier2d::dynamics::Velocity;
use crate::input::RawInput;
use crate::player::AIR_SPEED;
use crate::player::state::{Facing, PlayerState, StateTicks};

pub fn side_special_phys(state: &PlayerState, state_ticks: &StateTicks, raw_input: &RawInput, facing: Facing, velocity: &mut Velocity) {
    // say animation state is 1 second, that's 128Hz = is 128 per second, 128 ticks is 100%;
    const CHARGE_FRAMES: u32 = 30;
    const ATK_FRAMES: u32 = 70;

    // how far are we into the animation
    let di = raw_input.stick;

    match state_ticks.0 {
        ticks if ticks <= CHARGE_FRAMES => {
            velocity.linear.x = 0.0;
            velocity.linear.y = 0.0;
        }
        ticks if ticks <= CHARGE_FRAMES + ATK_FRAMES => {
            let pct_atk = 1.0 - ((state_ticks.0 - CHARGE_FRAMES) as f32 / ATK_FRAMES as f32);

            let y_influence = (di.y * 0.9) * AIR_SPEED * pct_atk;

            velocity.linear.x = facing.sign() * AIR_SPEED * 18.0 * pct_atk;

            velocity.linear.y *= pct_atk;
            velocity.linear.y += y_influence;
        }
        _ => {
        }
    }
}
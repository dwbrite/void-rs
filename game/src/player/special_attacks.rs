use avian2d::prelude::LinearVelocity;
use crate::player::AIR_SPEED;
use crate::player::state::{Facing, PlayerState, StateTicks};

pub fn side_special_phys(_state: &PlayerState, state_ticks: &StateTicks, facing: Facing, move_x: f32, move_y: f32, velocity: &mut LinearVelocity) {
    // say animation state is 1 second, that's 128Hz = is 128 per second, 128 ticks is 100%;
    const CHARGE_FRAMES: u32 = 30;
    const ATK_FRAMES: u32 = 70;

    // how far are we into the animation
    let _ = move_x;

    match state_ticks.0 {
        ticks if ticks <= CHARGE_FRAMES => {
            velocity.x = 0.0;
            velocity.y = 0.0;
        }
        ticks if ticks <= CHARGE_FRAMES + ATK_FRAMES => {
            let pct_atk = 1.0 - ((state_ticks.0 - CHARGE_FRAMES) as f32 / ATK_FRAMES as f32);

            let y_influence = (move_y * 0.9) * AIR_SPEED * pct_atk;

            velocity.x = facing.sign() * AIR_SPEED * 18.0 * pct_atk;

            velocity.y *= pct_atk;
            velocity.y += y_influence;
        }
        _ => {
        }
    }
}
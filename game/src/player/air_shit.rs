use bevy::prelude::Mut;
use bevy_rapier2d::dynamics::Velocity;
use crate::input::RawInput;
use crate::player::{CharacterStatus, AIR_FRICTION, AIR_JUMPS, AIR_JUMP_BASE_IMPULSE, AIR_JUMP_DI_DOWN_STRENGTH, AIR_JUMP_DI_HORIZ_STRENGTH, AIR_JUMP_DI_UP_STRENGTH, AIR_JUMP_DURATION, AIR_JUMP_IMPULSE_DECAY, AIR_SPEED};
use crate::player::state::{AirJumpsRemaining, PlayerState, StateTicks};

pub fn air_jump_phys(mut state: &PlayerState, state_ticks: &StateTicks, raw_input: &RawInput, air_jumps: &AirJumpsRemaining, mut velocity: &mut Velocity, x: &mut CharacterStatus) {
    // Frame 0: apply impulse with DI influence
    if state_ticks.0 == 0 {
        let jumps_used = (AIR_JUMPS - air_jumps.0) as f32; // need AirJumpsRemaining in query

        let di = raw_input.stick;
        let vertical_di = if di.y > 0.0 {
            di.y * AIR_JUMP_DI_UP_STRENGTH    // nerfed upward
        } else {
            di.y * AIR_JUMP_DI_DOWN_STRENGTH   // full downward
        };

        velocity.linear.y = AIR_JUMP_BASE_IMPULSE + (20. *  vertical_di) - (AIR_JUMP_BASE_IMPULSE * AIR_JUMP_IMPULSE_DECAY * jumps_used);
        velocity.linear.x = velocity.linear.x * 0.5 + di.x * AIR_JUMP_DI_HORIZ_STRENGTH;
    }

    crate::player::state::physics::aerial_movement(&raw_input, &mut velocity, (1.0, 1.0));
}
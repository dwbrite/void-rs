use bevy::app::{App, FixedUpdate, Plugin, Startup, Update};
use bevy::prelude::*;
use bevy_rapier2d::dynamics::Velocity;
use crate::input;
use crate::input::{InputBuffer, RawInput};
use crate::player::state::{sproing, AirJumpsRemaining, AirborneState, PlayerAction, PlayerState, PreviousState, StateTicks};

pub mod special_attacks;
pub mod state;
pub(crate) mod animation;
pub(crate) mod air_shit;

const COYOTE_FRAMES: u32 = 6;
pub const AIR_JUMP_DURATION: u32 = 2;
pub const AIR_FRICTION: f32 = 0.92;
// horizontal velocity multiplier per tick
pub const AIR_SPEED: f32 = 8.1;
// number of air jumps
pub const AIR_JUMPS: u32 = 6;
// horizontal air acceleration
pub const AIR_JUMP_BASE_IMPULSE: f32 = 130.;
pub const AIR_JUMP_IMPULSE_DECAY: f32 = 0.07;
// each jump is this much weaker
pub const AIR_JUMP_DI_UP_STRENGTH: f32 = 0.1;
// nerfed upward DI
pub const AIR_JUMP_DI_DOWN_STRENGTH: f32 = 0.8;
// full downward DI kept
pub const AIR_JUMP_DI_HORIZ_STRENGTH: f32 = 0.3;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, input::setup_player);
        app.add_systems(Update, input::buffer_jump_input);
        app.insert_resource(InputBuffer {
            jump_buffered: false,
            spec_buffered: false,
            atk_buffered: false,
        });
        app.add_systems(FixedUpdate, (
            input::read_raw_input,
            state::detect_ground,
            state::update_playerstate,
            state::reset_state_ticks,
            animation::flip_sprite,
            animation::playerstate_animation,
            state::reset_air_jumps,
            state::update_playerstate_physics,
            update_kinetic_energy,
            sproing,
            state::increment_state_ticks,
        ).chain());
    }
}

#[derive(Component)]
pub struct CharacterStatus {
    pub busy: bool, // when he so busy doing other shit
    pub no_jump: bool,
}

#[derive(Component)]
pub struct KineticEnergy {
    pub value: f32,
    pub peak: f32,
    pub frames_since_loss: u8,
}

pub fn update_kinetic_energy(
    mut query: Query<&mut KineticEnergy>,
) {
    const DECAY_FRAMES: f32 = 10.0;
    for mut ke in query {
        if ke.value >= ke.peak {
            ke.peak = ke.value;
            ke.frames_since_loss = 0;
        } else {
            ke.frames_since_loss += 1;

            let t = (ke.frames_since_loss as f32 / DECAY_FRAMES).clamp(0.0, 1.0);
            let retention = 1.0 - t.powi(5);

            ke.value = (ke.peak - ke.value) * retention;
        }
    }
}

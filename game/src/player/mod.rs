use bevy::app::{App, FixedUpdate, Plugin, Startup};
use bevy::asset::AssetServer;
use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::{Animation, AseAnimation, Aseprite};
use bevy_rapier2d::dynamics::Velocity;
use bevy_rapier2d::prelude::{ActiveEvents, Collider, LockedAxes, RigidBody};
use crate::{AnimationIndices, AnimationTimer, PIXEL_PERFECT_LAYERS};
use crate::player::state::{sproing, AirJumpsRemaining, AirborneState, Facing, PlayerAction, PlayerState, PreviousState, SpringMass, StateTicks};

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
        app.add_systems(Startup, setup_player)
            .add_systems(FixedUpdate, (
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
pub struct PlayerGamepad;

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let animation_indices = AnimationIndices { first: 0, last: 8 };

    commands.spawn((
        PlayerGamepad,
        PlayerState::Idle,
        AirborneState::Grounded,
        PlayerAction::None,
        StateTicks(0),
        Transform::from_xyz(0.0, 60.0, 0.0),
        animation_indices,
        PIXEL_PERFECT_LAYERS,
        AirJumpsRemaining(5),
        Collider::cuboid(4.0, 5.0),
        RigidBody::Dynamic,
        Sprite::default(),
    ))
    .insert((
        Velocity::zero(),
        ActiveEvents::COLLISION_EVENTS,
        LockedAxes::ROTATION_LOCKED,
        AseAnimation {
            aseprite: asset_server.load::<Aseprite>("gamer.aseprite"),
            animation: Animation::tag("slide"),
        },
        Facing::Right,
        PreviousState(PlayerState::Idle),
        KineticEnergy {
            value: 0.0,
            peak: 0.0,
            frames_since_loss: 0,
        },
        CharacterStatus {
            busy: false,
            no_jump: false,
        },
        SpringMass {
            y: 0.0,
            vy: 0.0,
            k: 0.042,
            damping: 0.40,
            mass: 3.5,
            last_parent_vy: 0.0,
            parent_coupling: 0.06,
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
}

#[derive(Component)]
pub struct CharacterStatus {
    pub busy: bool,
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

use bevy::camera::visibility::RenderLayers;
use bevy::input::{ButtonInput, InputPlugin};
use bevy::input::gamepad::GamepadEvent;
use bevy::math::Vec2;
use bevy::prelude::*;
use crate::{AnimationIndices, AnimationTimer, PIXEL_PERFECT_LAYERS};

const COYOTE_FRAMES: u32 = 6;
const AIR_JUMP_DURATION: u32 = 2;      // frames before transitioning to ControlledFall
const AIR_JUMP_IMPULSE: f32 = 0.8;      // base upward impulse on frame 0
const AIR_JUMP_DI_STRENGTH: f32 = 0.8;  // how much stick influences the impulse direction
const AIR_FRICTION: f32 = 0.92;         // horizontal velocity multiplier per tick
const AIR_SPEED: f32 = 0.08;            // horizontal air acceleration
const AIR_JUMP_BASE_IMPULSE: f32 = 0.8;
const AIR_JUMP_IMPULSE_DECAY: f32 = 0.15; // each jump is this much weaker
const AIR_JUMP_DI_UP_STRENGTH: f32 = 0.1;   // nerfed upward DI
const AIR_JUMP_DI_DOWN_STRENGTH: f32 = 0.8; // full downward DI kept
const AIR_JUMP_DI_HORIZ_STRENGTH: f32 = 0.3;

const GRAVITY: f32 = -0.015;


#[derive(Component, Debug)]
pub struct Velocity(Vec2);

#[derive(Component)]
pub struct PlayerGamepad;

#[derive(Component)]
pub struct RawInput {
    pub stick: Vec2,
    pub jump_held: bool,
    pub jump_pressed: bool,
}

#[derive(Resource)]
pub struct InputBuffer {
    pub jump_buffered: bool,
}

// runs in Update — catches the press the frame it happens
pub fn buffer_jump_input(
    gamepads: Query<(&Name, &Gamepad)>,
    mut buffer: ResMut<InputBuffer>,
) {
    let jump = GamepadButton::North;
    for (name, gamepad) in &gamepads {
        if !name.contains("Ultimate") { continue; }  // same filter as everywhere else
        if gamepad.just_pressed(jump) {
            buffer.jump_buffered = true;
        }
    }
}

// runs in FixedUpdate — consumes the buffer
pub fn read_raw_input(
    mut query: Query<&mut RawInput>,
    gamepads: Query<(&Name, &Gamepad)>,
    mut buffer: ResMut<InputBuffer>,
) {
    let jump = GamepadButton::North;

    let jump_pressed = buffer.jump_buffered;
    buffer.jump_buffered = false;

    for mut raw_inputs in &mut query {
        raw_inputs.jump_pressed = jump_pressed;

        for (name, gamepad) in &gamepads {
            if !name.contains("Ultimate") { continue; }
            raw_inputs.jump_held = gamepad.pressed(jump);
            raw_inputs.stick = gamepad.left_stick();
        }
    }
}

#[derive(Component)]
pub enum AirborneState {
    Grounded,
    Airborne,
    CoyoteTime(u32),
}

#[derive(Component)]
pub struct AirJumpsRemaining(pub u32);

#[derive(Bundle)]
pub struct PlayerBundle {
    player_gamepad: PlayerGamepad,
    player_state: PlayerState,
    airborne_state: AirborneState,
    player_action: PlayerAction,
    state_ticks: StateTicks,
    position: Transform,
    velocity: Velocity,
    sprite: Sprite,
    pub animation_indices: AnimationIndices,
    pub animation_timer: AnimationTimer,
    render_layers: RenderLayers,
    pub raw_input: RawInput,
    air_jumps: AirJumpsRemaining,
}

pub struct PlayerPlugin;

#[derive(Resource)]
pub struct Gravity(pub f32);

fn reset_air_jumps(mut query: Query<(&AirborneState, &mut AirJumpsRemaining), Changed<AirborneState>>) {
    for (state, mut jumps) in &mut query {
        if matches!(state, AirborneState::Grounded) {
            jumps.0 = 5;
        }
    }
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(GRAVITY));
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, buffer_jump_input);
        app.insert_resource(InputBuffer {
            jump_buffered: false,
        });
        app.add_systems(FixedUpdate, (
            read_raw_input,          // 1. sample controller state
            update_playerstate,      // 2. state transitions (uses raw_input)
            reset_air_jumps,
            reset_state_ticks,       // 3. reset ticks on any state that just changed
            update_playerstate_physics, // 4. physics decisions (reads state_ticks.0, wants 0 on transition frame)
            apply_gravity,           // 5. gravity accumulation
            apply_velocity,          // 6. integrate velocity into position
            increment_state_ticks,   // 7. increment last so next frame sees the right count
        ).chain());
    }
}

pub fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 3, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let animation_indices = AnimationIndices { first: 0, last: 8 };


    println!("spawning player bundle?");
    commands.spawn(PlayerBundle {
        player_gamepad: PlayerGamepad,
        player_state: PlayerState::Idle,
        airborne_state: AirborneState::Grounded,
        player_action: PlayerAction::None,
        state_ticks: StateTicks(0),
        position: Transform::from_xyz(0.0, 0.0, 0.0),
        velocity: Velocity(Vec2::ZERO),
        sprite: Sprite::from_atlas_image(asset_server.load("gamer.png"), TextureAtlas {
            layout: texture_atlas_layout,
            index: animation_indices.first,
        }),
        animation_indices,
        animation_timer: AnimationTimer(Timer::from_seconds(0.080, TimerMode::Repeating)),
        raw_input: RawInput {
            stick: Vec2 { x: 0.0, y: 0.0 },
            jump_held: false,
            jump_pressed: false,
        },
        render_layers: PIXEL_PERFECT_LAYERS,
        air_jumps: AirJumpsRemaining(5),
    });
}

pub fn apply_gravity(mut query: Query<&mut Velocity>, gravity: Res<Gravity>) {
    for mut velocity in query {
        velocity.0.y += gravity.0;
    }

}

pub fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x;
        transform.translation.y += velocity.0.y;

        // todo: removeme
        // temporary floor at y=0
        if transform.translation.y < -60.0 {
            transform.translation.y = -60.0;
        }
    }
}


pub fn update_playerstate(
    mut query: Query<(&mut PlayerState, &mut PlayerAction, &RawInput, &mut AirJumpsRemaining)>,
) {
    for (mut playerstate, mut playeraction, raw_input, mut air_jumps) in &mut query {
        *playeraction = if raw_input.jump_pressed {
            PlayerAction::Jump
        } else {
            PlayerAction::None
        };

        match (&*playerstate, &*playeraction) {
            (PlayerState::Idle, PlayerAction::Jump) => {
                *playerstate = PlayerState::Jumping;
            },
            (PlayerState::Jumping | PlayerState::ControlledFall, PlayerAction::Jump)
            if air_jumps.0 > 0 => {
                air_jumps.0 -= 1;
                *playerstate = PlayerState::AirJump;
            }
            _ => {}
        }
    }
}

#[derive(Component)]
pub struct StateTicks(pub u32);

// reset on transition
fn reset_state_ticks(mut query: Query<&mut StateTicks, Changed<PlayerState>>) {
    for mut ticks in &mut query {
        ticks.0 = 0;
    }
}

// increment every tick
fn increment_state_ticks(mut query: Query<&mut StateTicks>) {
    for mut ticks in &mut query {
        ticks.0 += 1;
    }
}

pub fn update_playerstate_physics(mut query: Query<(&mut Velocity, &mut PlayerState, &StateTicks, &RawInput)>, gravity: Res<Gravity>) {
    for (mut velocity, mut state, state_ticks, raw_input) in &mut query {
        match &*state {
            PlayerState::Jumping | PlayerState::ControlledFall => {
                // air friction + directional control
                velocity.0.x *= AIR_FRICTION;
                velocity.0.x += raw_input.stick.x * AIR_SPEED;
            }

            PlayerState::AirJump => {
                // Frame 0: apply impulse with DI influence
                if state_ticks.0 == 0 {
                    let di = raw_input.stick * AIR_JUMP_DI_STRENGTH;
                    velocity.0.y = AIR_JUMP_IMPULSE + di.y.max(0.0); // DI can boost but not reduce upward
                    velocity.0.x = velocity.0.x * 0.5 + di.x;        // DI blends with existing momentum
                }

                // Air friction during jump too
                velocity.0.x *= AIR_FRICTION;
                velocity.0.x += raw_input.stick.x * AIR_SPEED;

                // Transition to ControlledFall after duration expires
                if state_ticks.0 >= AIR_JUMP_DURATION {
                    *state = PlayerState::ControlledFall;
                }
            }

            _ => {}
        }
    }
}

#[derive(Component)]
#[derive(Debug)]
pub enum PlayerState {
    Idle,
    Crouch,
    Dash,
    Running,
    Slide,
    Jumping,

    Landing,
    Hitstun,
    Tumble,

    WallGrab,
    WallJump,

    GroundTech,
    WallTech,

    AirJump,
    AirDodge,
    SpotDodge,

    ControlledFall,
    UncontrolledFall,

    UpAir,
    DownAir,
    FwdAir,
    BackAir,
    NeutralAir,

    UpAtk,
    DownAtk,
    FwdAtk,
    BackAtk,
    NeutralAtk,
}

#[derive(Component)]
#[derive(Debug)]
pub enum PlayerAction {
    Jump,
    Attack(Vec2),
    Special(Vec2),
    Grab,
    Dodge,
    None,
}



use bevy::camera::visibility::RenderLayers;
use bevy::input::{ButtonInput, InputPlugin};
use bevy::input::gamepad::GamepadEvent;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ActiveEvents, Collider, CollisionEvent, Damping, ExternalImpulse, LockedAxes, RigidBody, Velocity};
use crate::{AnimationIndices, AnimationTimer, PIXEL_PERFECT_LAYERS};

const COYOTE_FRAMES: u32 = 6;
const AIR_JUMP_DURATION: u32 = 2;      // frames before transitioning to ControlledFall
const AIR_JUMP_IMPULSE: f32 = 200.0;      // base upward impulse on frame 0
const AIR_JUMP_DI_STRENGTH: f32 = 0.8;  // how much stick influences the impulse direction
const AIR_FRICTION: f32 = 0.92;         // horizontal velocity multiplier per tick
const AIR_SPEED: f32 = 8.;            // horizontal air acceleration
const AIR_JUMP_BASE_IMPULSE: f32 = 100.;
const AIR_JUMP_IMPULSE_DECAY: f32 = 0.05; // each jump is this much weaker
const AIR_JUMP_DI_UP_STRENGTH: f32 = 0.1;   // nerfed upward DI
const AIR_JUMP_DI_DOWN_STRENGTH: f32 = 0.8; // full downward DI kept
const AIR_JUMP_DI_HORIZ_STRENGTH: f32 = 0.3;

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

pub struct PlayerPlugin;

fn reset_air_jumps(mut query: Query<(&AirborneState, &mut AirJumpsRemaining), Changed<AirborneState>>) {
    for (state, mut jumps) in &mut query {
        if matches!(state, AirborneState::Grounded) {
            jumps.0 = 5;
        }
    }
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, buffer_jump_input);
        app.insert_resource(InputBuffer {
            jump_buffered: false,
        });
        app.add_systems(FixedUpdate, (
            read_raw_input,
            detect_ground,
            update_playerstate,
            changed_state_debug,
            reset_air_jumps,
            reset_state_ticks,       // 3. reset ticks on any state that just changed
            update_playerstate_physics, // 4. physics decisions (reads state_ticks.0, wants 0 on transition frame)
            increment_state_ticks,   // 7. increment last so next frame sees the right count
        ).chain());
    }
}

pub fn detect_ground(
    mut collision_events: MessageReader<CollisionEvent>,
    mut player_query: Query<&mut AirborneState, With<PlayerGamepad>>,
) {
    for event in collision_events.read() {
        match event {
            CollisionEvent::Started(e1, e2, _) => {
                println!("ur grounded");
                if let Ok(mut airborne) = player_query.get_mut(*e1) {
                    *airborne = AirborneState::Grounded;
                } else if let Ok(mut airborne) = player_query.get_mut(*e2) {
                    *airborne = AirborneState::Grounded;
                }
            }
            CollisionEvent::Stopped(e1, e2, _) => {
                if let Ok(mut airborne) = player_query.get_mut(*e1) {
                    *airborne = AirborneState::Airborne;
                } else if let Ok(mut airborne) = player_query.get_mut(*e2) {
                    *airborne = AirborneState::Airborne;
                }
            }
        }
    }
}

pub fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 3, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let animation_indices = AnimationIndices { first: 0, last: 8 };

    World::new();

    println!("spawning player bundle?");
    commands.spawn((
        PlayerGamepad,
        PlayerState::Idle,
        AirborneState::Grounded,
        PlayerAction::None,
        StateTicks(0),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Sprite::from_atlas_image(asset_server.load("gamer.png"), TextureAtlas {
            layout: texture_atlas_layout,
            index: animation_indices.first,
        }),
        animation_indices,
        AnimationTimer(Timer::from_seconds(0.080, TimerMode::Repeating)),
        RawInput {
            stick: Vec2 { x: 0.0, y: 0.0 },
            jump_held: false,
            jump_pressed: false,
        },
        PIXEL_PERFECT_LAYERS,
        AirJumpsRemaining(5),
        Collider::cuboid(4., 4.),
        RigidBody::Dynamic,
        ExternalImpulse::default(),
    )).insert((
        Velocity::zero(),
        ActiveEvents::COLLISION_EVENTS,
        LockedAxes::ROTATION_LOCKED,
    ));
}

pub fn update_playerstate(
    mut query: Query<(&mut PlayerState, &mut PlayerAction, &RawInput, &mut AirJumpsRemaining)>,
) {
    for (mut playerstate, mut playeraction, raw_input, mut air_jumps) in &mut query {
        *playeraction = if raw_input.jump_pressed {
            println!("pressed jump");
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
                println!("air jumping");
                air_jumps.0 -= 1;
                *playerstate = PlayerState::AirJump;
            },
            (state, PlayerAction::Jump) => {
                println!("jump blocked in state: {:?}, air_jumps: {}", state, air_jumps.0);
            }
            _ => {}
        }
    }
}

pub fn changed_state_debug(query: Query<&PlayerState, Changed<PlayerState>>) {
    for s in query {
        println!("changed state: {:?}", s);
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

pub fn update_playerstate_physics(mut query: Query<(&mut PlayerState, &StateTicks, &RawInput, &AirJumpsRemaining, &mut Velocity, &mut ExternalImpulse)>) {
    for (mut state, state_ticks, raw_input, air_jumps, mut velocity, mut impulse) in &mut query {
        match &*state {
            PlayerState::Jumping | PlayerState::ControlledFall => {
                // air friction + directional control
                velocity.linear.x *= AIR_FRICTION;
                velocity.linear.x += raw_input.stick.x * AIR_SPEED;
            }

            PlayerState::AirJump => {
                // Frame 0: apply impulse with DI influence
                if state_ticks.0 == 0 {
                    // jumps_remaining goes 4→0 as you use them, so jumps_used = 5 - remaining
                    let jumps_used = (5 - air_jumps.0) as f32; // need AirJumpsRemaining in query
                    let impulse = (AIR_JUMP_BASE_IMPULSE - jumps_used * AIR_JUMP_IMPULSE_DECAY).max(9.0);

                    let di = raw_input.stick;
                    let vertical_di = if di.y > 0.0 {
                        di.y * AIR_JUMP_DI_UP_STRENGTH    // nerfed upward
                    } else {
                        di.y * AIR_JUMP_DI_DOWN_STRENGTH   // full downward
                    };

                    println!("doing an air jump!!!");

                    // velocity.linear.y = impulse + vertical_di.max(-impulse * 0.8); // down DI can't fully cancel jump
                    velocity.linear.y = AIR_JUMP_BASE_IMPULSE;
                    velocity.linear.x = velocity.linear.x * 0.5 + di.x * AIR_JUMP_DI_HORIZ_STRENGTH;
                }

                // Air friction during jump too
                velocity.linear.x *= AIR_FRICTION;
                velocity.linear.x += raw_input.stick.x * AIR_SPEED;

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



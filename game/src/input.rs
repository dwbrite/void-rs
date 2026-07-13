use std::range::Range;
use bevy::app::DynEq;
use bevy::asset::ErasedAssetLoader;
use bevy::camera::visibility::RenderLayers;
use bevy::input::{ButtonInput, InputPlugin};
use bevy::input::gamepad::GamepadEvent;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::reflect::List;
use bevy::tasks::futures_lite::StreamExt;
use bevy_aseprite_ultra::prelude::{Animation, AnimationRepeat, AseAnimation};
use bevy_rapier2d::prelude::{ActiveEvents, Collider, CollisionEvent, Damping, ExternalImpulse, LockedAxes, RigidBody, Velocity};
use crate::{AnimationIndices, AnimationTimer, PIXEL_PERFECT_LAYERS};
use crate::player::{air_shit, animation, special_attacks, state, CharacterStatus, KineticEnergy, AIR_FRICTION, AIR_JUMP_BASE_IMPULSE, AIR_JUMP_DI_DOWN_STRENGTH, AIR_JUMP_DI_HORIZ_STRENGTH, AIR_JUMP_DI_UP_STRENGTH, AIR_JUMP_DURATION, AIR_JUMP_IMPULSE_DECAY, AIR_SPEED};
use crate::player::state::{AirJumpsRemaining, AirborneState, Facing, PlayerAction, PlayerState, PreviousState, SpringMass, StateTicks};

#[derive(Component)]
pub struct PlayerGamepad;

use heapless::{HistoryBuf};
use crate::input::Octant::{East, North, South, West};

#[derive(Debug)]
pub enum StickZone {
    CenterX,
    CenterY,
    Edge(Octant),
    Octant(Octant),
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Octant {
    East, Northeast, North, Northwest, West, Southwest, South, Southeast
}

impl Octant {
    pub fn from_oct(v: u8) -> Octant {
        match v {
            0 => Octant::East,
            1 => Octant::Northeast,
            2 => Octant::North,
            3 => Octant::Northwest,
            4 => Octant::West,
            5 => Octant::Southwest,
            6 => Octant::South,
            7 => Octant::Southeast,
            _ => panic!("Invalid octant value: {}", v),
        }
    }

    pub fn to_u8(&self, octant: Octant) -> u8 {
        match octant {
            Octant::East => 0,
            Octant::Northeast => 1,
            Octant::North => 2,
            Octant::Northwest => 3,
            Octant::West => 4,
            Octant::Southwest => 5,
            Octant::South => 6,
            Octant::Southeast => 7,
        }
    }
}

impl StickZone {
    fn in_zone(&self, pos: &Vec2) -> bool {
        let r2 = pos.x * pos.x + pos.y * pos.y;
        let in_outer_ring = r2 > 0.90 * 0.90;

        // Split the unit circle into 8 equal sectors (E, NE, N, NW, W, SW, S, SE).
        let xy_octant = {
            let angle = pos.y.atan2(pos.x).rem_euclid(std::f32::consts::TAU);
            let octant = (((angle + std::f32::consts::PI / 8.0) / (std::f32::consts::PI / 4.0)).floor() as u8) % 8;
            Octant::from_oct(octant)
        };

        match self {
            StickZone::CenterX => pos.x.abs() <= 0.40,
            StickZone::CenterY => pos.y.abs() <= 0.40,
            StickZone::Edge(octant) => octant == &xy_octant && in_outer_ring,
            StickZone::Octant(octant) => octant == &xy_octant,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinDir { Cw, Ccw }

pub enum StickAction {
    Flick(Octant),
    Recenter,
    Rotate(SpinDir),
}

pub struct ZoneTrigger {
    from_zone: StickZone,
    to_zone: StickZone,
    frames: u64,
    action: StickAction,
}

#[derive(Component)]
pub struct StickWatcher9000 {
    zone_triggers: Vec<ZoneTrigger>,
    stick_hist: HistoryBuf<Vec2, 32>,
}

pub fn stick_watch(mut query: Query<(&mut StickWatcher9000, &RawInput)>) {
    for (mut stickwatch, raw_input) in query.iter_mut() {
        let pos = raw_input.stick;
        stickwatch.stick_hist.write(pos);
        stickwatch.check_zone_triggers();
    }
}

impl StickWatcher9000 {
    pub fn check_zone_triggers(&mut self) {
        for trigger in self.zone_triggers.iter() {
            if trigger.to_zone.in_zone(self.stick_hist.recent().unwrap_or(&Vec2::ZERO)) {
                let last_n_recent: Vec<_> = self.stick_hist.oldest_ordered().rev().skip(1).take(trigger.frames as usize).collect();

                for (idx, &pos) in last_n_recent.iter().enumerate() {
                    if trigger.to_zone.in_zone(pos) {
                        // stop processing if we've already triggered a flick
                        break;
                    }

                    if trigger.from_zone.in_zone(pos) {
                        // TODO: report flick and stop processing
                        println!("went from zone {:?} to {:?} in {} frames", trigger.from_zone, trigger.to_zone, idx+1);
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct RawInput {
    pub stick: Vec2,
    pub jump_held: bool,
    pub spec_held: bool,
    pub atk_held: bool,
    pub jump_pressed: bool,
    pub spec_pressed: bool,
    pub atk_pressed: bool,
}

#[derive(Resource)]
pub struct InputBuffer {
    pub jump_buffered: bool,
    pub spec_buffered: bool,
    pub atk_buffered: bool,
}

pub fn buffer_jump_input(
    gamepads: Query<(&Name, &Gamepad)>,
    mut buffer: ResMut<InputBuffer>,
) {
    let jump = GamepadButton::North;
    let spec = GamepadButton::West;
    let atk = GamepadButton::South;
    for (name, gamepad) in &gamepads {
        if !name.contains("Ultimate") { continue; }  // same filter as everywhere else
        if gamepad.just_pressed(jump) {
            buffer.jump_buffered = true;
        }

        if gamepad.just_pressed(spec) {
            buffer.spec_buffered = true;
        }

        if gamepad.just_pressed(atk) {
            buffer.atk_buffered = true;
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
    let spec = GamepadButton::West;
    let atk = GamepadButton::South;

    let jump_pressed = buffer.jump_buffered;
    buffer.jump_buffered = false;

    let spec_pressed = buffer.spec_buffered;
    buffer.spec_buffered = false;

    let atk_pressed = buffer.atk_buffered;
    buffer.atk_buffered = false;

    for mut raw_inputs in &mut query {
        raw_inputs.jump_pressed = jump_pressed;
        raw_inputs.spec_pressed = spec_pressed;
        raw_inputs.atk_pressed = atk_pressed;

        for (name, gamepad) in &gamepads {
            if !name.contains("Ultimate") { continue; }
            raw_inputs.jump_held = gamepad.pressed(jump);
            raw_inputs.spec_held = gamepad.pressed(spec);
            raw_inputs.atk_held = gamepad.pressed(atk);
            raw_inputs.stick = gamepad.left_stick();
        }
    }
}

pub fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 3, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let animation_indices = AnimationIndices { first: 0, last: 8 };

    World::new();

    commands.spawn((
        PlayerGamepad,
        PlayerState::Idle,
        AirborneState::Grounded,
        PlayerAction::None,
        StateTicks(0),
        Transform::from_xyz(0.0, 0.0, 0.0),
        animation_indices,
        RawInput {
            stick: Vec2 { x: 0.0, y: 0.0 },
            jump_held: false,
            spec_held: false,
            atk_held: false,
            jump_pressed: false,
            spec_pressed: false,
            atk_pressed: false,
        },
        PIXEL_PERFECT_LAYERS,
        AirJumpsRemaining(5),
        Collider::cuboid(4., 5.),
        RigidBody::Dynamic,
        ExternalImpulse::default(),
        Sprite::default(),
    )).insert((
        Velocity::zero(),
        ActiveEvents::COLLISION_EVENTS,
        LockedAxes::ROTATION_LOCKED,
        AseAnimation {
            aseprite: asset_server.load("gamer.aseprite"),
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
        StickWatcher9000 {
            zone_triggers: {
                let mut triggers = vec![];

                let rotations: Vec<ZoneTrigger> = (0..8).flat_map(|i| {
                    let a = Octant::from_oct(i);
                    let b = Octant::from_oct((i + 1) % 8);
                    [
                        ZoneTrigger { from_zone: StickZone::Edge(a), to_zone: StickZone::Edge(b),
                            frames: 2, action: StickAction::Rotate(SpinDir::Ccw) },
                        ZoneTrigger { from_zone: StickZone::Edge(b), to_zone: StickZone::Edge(a),
                            frames: 2, action: StickAction::Rotate(SpinDir::Cw) },
                    ]
                }).collect();

                let flicks: Vec<ZoneTrigger> = vec![
                    ZoneTrigger {
                        from_zone: StickZone::CenterX,
                        to_zone: StickZone::Edge(East),
                        frames: 3,
                        action: StickAction::Flick(East),
                    },
                    ZoneTrigger {
                        from_zone: StickZone::CenterX,
                        to_zone: StickZone::Edge(West),
                        frames: 3,
                        action: StickAction::Flick(West),
                    },
                    ZoneTrigger {
                        from_zone: StickZone::CenterY,
                        to_zone: StickZone::Edge(North),
                        frames: 3,
                        action: StickAction::Flick(North),
                    },
                    ZoneTrigger {
                        from_zone: StickZone::CenterY,
                        to_zone: StickZone::Edge(South),
                        frames: 3,
                        action: StickAction::Flick(South),
                    }
                ];



                triggers.extend(rotations);
                triggers.extend(flicks);

                triggers
            },
            stick_hist: HistoryBuf::new(),
        },
        SpringMass {
            y: 0.0,
            vy: 0.0,
            k: 0.042,          // slower return (less stiff)
            damping: 0.40,     // near single-overshoot territory
            mass: 3.5,         // heavier = slower motion
            last_parent_vy: 0.0,
            parent_coupling: 0.06
        }
    ));
}


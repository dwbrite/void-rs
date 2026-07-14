use std::collections::HashMap;
use enum_map::{enum_map, Enum, EnumMap};
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
use crate::input::ButtonState::{Held, JustPressed, JustReleased, Released};
use crate::input::Octant::{East, North, South, West};

#[derive(Debug)]
pub enum StickZone {
    CenterX,
    CenterY,
    Edge(Octant),
    Octant(Octant),
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Enum)]
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

    pub fn to_u8(&self) -> u8 {
        match self {
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

#[derive(Debug, Clone, Copy, PartialEq, Hash, Enum)]
pub enum InputAction {
    Jump,
    Special,
    Attack,
    Grab,
    Dodge,
    Pause,
    SelectUndef,
    LStickUndef,
    RStickUndef,
    LBUndef,
    RBUndef,
    DUp,
    DDown,
    DLeft,
    DRight,
    LSTap(Octant),
    RSTap(Octant),
}

pub fn piss_buckets() -> PissBucket {
    let ctrl_map: EnumMap<InputAction, GamepadButton> = enum_map! {
        InputAction::Jump => GamepadButton::North,
        InputAction::Special => GamepadButton::West,
        InputAction::Attack => GamepadButton::South,
        InputAction::Grab => GamepadButton::East,
        InputAction::Dodge => GamepadButton::RightTrigger,
        InputAction::Pause => GamepadButton::Start,
        InputAction::SelectUndef => GamepadButton::Select,
        InputAction::LStickUndef => GamepadButton::LeftThumb,
        InputAction::RStickUndef => GamepadButton::RightThumb,
        InputAction::LBUndef => GamepadButton::LeftTrigger,
        InputAction::RBUndef => GamepadButton::RightTrigger,
        InputAction::DUp => GamepadButton::DPadUp,
        InputAction::DDown => GamepadButton::DPadDown,
        InputAction::DLeft => GamepadButton::DPadLeft,
        InputAction::DRight => GamepadButton::DPadRight,
        // TODO: make all this Optional and cry about the "complexity" later.
        InputAction::LSTap(o) => GamepadButton::Other(o.to_u8()),
        InputAction::RSTap(o) => GamepadButton::Other(o.to_u8()),
    };

    PissBucket {
        input_map: ctrl_map,
        piss_butt: enum_map! {,
            InputAction::Jump => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::Special => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::Attack => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::Grab => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::Dodge => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::Pause => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::SelectUndef => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::LStickUndef => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::RStickUndef => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::LBUndef => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::RBUndef => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::DUp => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::DDown => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::DLeft => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::DRight => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::LSTap(_) => ButtBuffer { state: Released, ticks_since_pressed: 255 },
            InputAction::RSTap(_) => ButtBuffer { state: Released, ticks_since_pressed: 255 },
        }
    }
}

pub enum ButtonState {
    JustPressed,
    Held, // todo: pressure or any other analog signal. typically seen from ps2/3 face btns
    JustReleased,
    Released,
}

impl ButtonState {
    pub fn is_pressed(&self) -> bool {
        match self {
            JustPressed => { true }
            Held => { true }
            JustReleased => { false }
            Released => { false }
        }
    }
}

pub struct ButtBuffer {
    state: ButtonState,
    ticks_since_pressed: u8,
}

impl ButtBuffer {
    pub fn is_pressed(&self) -> bool {
        // inline always pls lmao
        self.state.is_pressed()
    }

    pub fn is_buffered(&self) -> bool {
        self.ticks_since_pressed < 16
    }

    pub fn eat_buffer(&mut self) -> bool {
        let b = self.ticks_since_pressed < 16;
        self.ticks_since_pressed = 255;
        b
    }
}

#[derive(Component)]
pub struct PissBucket {
    input_map: EnumMap<InputAction, GamepadButton>,
    piss_butt: EnumMap<InputAction, ButtBuffer>,
}

impl PissBucket {
    pub fn update_muchachos(&mut self, gamepad: &Gamepad) {
        for (action, buffer) in self.piss_butt.iter_mut() {
            let btn: GamepadButton = self.input_map[action];
            let pressed = gamepad.pressed(btn);

            buffer.state = if pressed {
                match buffer.state {
                    JustPressed | Held => { Held }
                    _ => {
                        buffer.ticks_since_pressed = 0;
                        JustPressed
                    },
                }
            } else {
                // we only track up to 2 seconds of inputs, so, idfk
                if buffer.ticks_since_pressed < 255 {
                    buffer.ticks_since_pressed += 1;
                }

                match buffer.state {
                    JustPressed | Held => { JustReleased }
                    _ => { Released }
                }
            };
        }
    }

    pub fn is_buffered(&self, action: InputAction) -> bool {
        self.piss_butt[action].is_buffered()
    }

    pub fn eat_buffer(&mut self, action: InputAction) -> bool {
        self.piss_butt[action].eat_buffer()
    }
}
// runs in FixedUpdate — consumes the buffer
pub fn update_inputter(
    mut piss_bucket: Query<&mut PissBucket>,
    gamepads: Query<(&Name, &Gamepad)>,
) {
    for mut piss in &mut piss_bucket {
        for (name, gamepad) in &gamepads {
            // TODO: save controller join order
            if !name.contains("Ultimate") { continue; }
            piss.update_muchachos(gamepad);
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


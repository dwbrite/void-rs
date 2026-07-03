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

#[derive(PartialEq)]
#[derive(Debug)]
pub enum StickRegion {
    Inner,
    Middle,
    Outer,
    OuterN,
    OuterNE,
    OuterE,
    OuterSE,
    OuterS,
    OuterSW,
    OuterW,
    OuterNW,
}

impl StickRegion {
    pub fn in_region(&self, (x, y): (f32, f32)) -> bool {
        let r2 = x*x + y*y;

        const INNER_RADIUS: f32 = 0.25;
        const MIDDLE_RADIUS: f32 = 0.7;

        let in_inner = r2 < INNER_RADIUS * INNER_RADIUS;
        let in_middle = r2 < MIDDLE_RADIUS * MIDDLE_RADIUS;


        match self {
            StickRegion::Inner => in_inner,
            StickRegion::Middle => in_middle,

            StickRegion::Outer => !in_inner && !in_middle,

            StickRegion::OuterE => x >  0.92 && x.abs() > y.abs(),
            StickRegion::OuterW => x < -0.92 && x.abs() > y.abs(),
            StickRegion::OuterN => y >  0.92 && y.abs() > x.abs(),
            StickRegion::OuterS => y < -0.92 && y.abs() > x.abs(),

            StickRegion::OuterNE => x > 0.70 && y > 0.70,
            StickRegion::OuterNW => x < -0.70 && y > 0.70,
            StickRegion::OuterSE => x > 0.70 && y < -0.70,
            StickRegion::OuterSW => x < -0.70 && y < -0.70,
        }
    }

    pub fn from_point((x, y): (f32, f32)) -> Self {
        let r2 = x * x + y * y;

        const INNER_RADIUS: f32 = 0.25;
        const MIDDLE_RADIUS: f32 = 0.7;

        let inner2 = INNER_RADIUS * INNER_RADIUS;
        let middle2 = MIDDLE_RADIUS * MIDDLE_RADIUS;

        if r2 < inner2 {
            return Self::Inner;
        }

        if r2 < middle2 {
            return Self::Middle;
        }

        if x >  0.92 && x.abs() > y.abs() { return Self::OuterE; }
        if x < -0.92 && x.abs() > y.abs() { return Self::OuterW; }

        if y >  0.92 && y.abs() > x.abs() { return Self::OuterN; }
        if y < -0.92 && y.abs() > x.abs() { return Self::OuterS; }

        if x > 0.70 && y > 0.70 { return Self::OuterNE; }
        if x < -0.70 && y > 0.70 { return Self::OuterNW; }
        if x > 0.70 && y < -0.70 { return Self::OuterSE; }
        if x < -0.70 && y < -0.70 { return Self::OuterSW; }

        Self::Outer
    }

    fn ring_index(&self) -> Option<i8> {
        use StickRegion::*;

        match self {
            OuterE  => Some(0),
            OuterNE => Some(1),
            OuterN  => Some(2),
            OuterNW => Some(3),
            OuterW  => Some(4),
            OuterSW => Some(5),
            OuterS  => Some(6),
            OuterSE => Some(7),
            _ => None, // Inner / Middle / Outer
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum RollDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum DoHeRoll {
    OhHeRollin(RollDirection),
    HeBeenRollin(RollDirection),
    HeDoneRollin,
}

#[derive(PartialEq)]
pub enum StickEvent {
    SmashEvent(StickRegion),
    RollEvent(DoHeRoll),
    HoldEvent(StickRegion, usize), // ticks since hold
    Wiggle,
}


// just imagine circles within the unit circle
#[derive(Component)]
pub struct StickWatcher9000 {
    history: HistoryBuf<(StickRegion, usize), 15>,
    ticks: usize,
    roll_state: Option<RollDirection>,
    smash: Option<SmashDirection>,
}


fn longest_run<I>(iter: I) -> usize
where
    I: Iterator<Item = bool>,
{
    let (mut best, mut cur) = (0, 0);

    for ok in iter {
        if ok {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 0;
        }
    }

    best
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum SmashDirection {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}


impl StickWatcher9000 {
    pub fn detect_roll(&mut self) -> Option<RollDirection> {
        const MAX_AGE: usize = 64; // 0.5s, meaning we must have at least 120rpm to start a roll, which is babycakes easy
        const ROLL_LEN: usize = 5;

        let recent: Vec<_> = self.history
            .iter()
            .filter(|(_, tick)| self.ticks - *tick <= MAX_AGE)
            .filter_map(|(region, tick)| region.ring_index().map(|r| (r, *tick)))
            .collect();

        if recent.len() < 2 {
            return None;
        }

        let cw = longest_run(
            recent.windows(2).map(|w| {
                let ((a, _), (b, _)) = (w[0], w[1]);
                (b - a).rem_euclid(8) == 7
            })
        );

        let ccw = longest_run(
            recent.windows(2).map(|w| {
                let ((a, _), (b, _)) = (w[0], w[1]);
                (b - a).rem_euclid(8) == 1
            })
        );

        if cw >= ROLL_LEN && cw >= ccw {
            Some(RollDirection::Clockwise)
        } else if ccw >= ROLL_LEN {
            Some(RollDirection::CounterClockwise)
        } else {
            None
        }
    }

    pub fn detect_smash(&self) -> Option<SmashDirection> {
        const MAX_FLICK_FRAMES: usize = 60; // tune this (4–8 usually feels good)

        // let history: Vec<_> = self.history.iter().collect();

        // search the most recent to find when we were in an inner state
        // find the edge and compare that to the most recent OuterCardinal, and do a smash if

        // find latest inner tick
        let (inner_tick, last_tick) = self.history.windows(2).rfind(|w| w[0].0 == StickRegion::Inner && w[1].0 != StickRegion::Inner)
            .map(|w| (Some(&w[0].1), w[1].1)).unwrap_or((None, 0));

        // if inner tick exists and
        if inner_tick.is_some() {
            if let Some((_, recent_tick)) = self.history.recent() {
                let tick_ct = recent_tick - last_tick;
                if recent_tick - last_tick > MAX_FLICK_FRAMES {
                    // println!("took {tick_ct} ticks");
                    return None;
                }
            }

            match self.history.recent() {
                Some((StickRegion::OuterN, _)) => return Some(SmashDirection::N),
                Some((StickRegion::OuterNE, _)) => return Some(SmashDirection::NE),
                Some((StickRegion::OuterE, _)) => return Some(SmashDirection::E),
                Some((StickRegion::OuterSE, _)) => return Some(SmashDirection::SE),
                Some((StickRegion::OuterS, _)) => return Some(SmashDirection::S),
                Some((StickRegion::OuterSW, _)) => return Some(SmashDirection::SW),
                Some((StickRegion::OuterW, _)) => return Some(SmashDirection::W),
                Some((StickRegion::OuterNW, _)) => return Some(SmashDirection::NW),
                _ => { return None; }
            }
        }

        None
    }
}

pub fn detect_stick_events(mut query: Query<(&mut StickWatcher9000, &RawInput)>) {
    for (mut stickwatch, input) in query.iter_mut() {
        let ticks = stickwatch.ticks;
        let region = StickRegion::from_point((input.stick.x, input.stick.y));
        if stickwatch.history.recent().is_some_and(|(last, _)| region != *last) || stickwatch.history.is_empty() {
            stickwatch.history.write((region, ticks));
        }

        let maybe_roll = stickwatch.detect_roll();
        if maybe_roll != stickwatch.roll_state {
            println!("edging to {:?}", maybe_roll);
        }
        stickwatch.roll_state = maybe_roll;


        let maybe_smash = stickwatch.detect_smash();
        if maybe_smash != stickwatch.smash {
            println!("edge smashing to {:?}", maybe_smash);
        }
        stickwatch.smash = maybe_smash;


        stickwatch.ticks += 1;
    }
}

#[derive(Component)]
pub struct RawInput {
    pub stick: Vec2,
    pub jump_held: bool,
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
            // raw_inputs.spec_held = gamepad.pressed(spec);
            // raw_inputs.atk_held = gamepad.pressed(atk);
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
            history: HistoryBuf::new(),
            ticks: 0,
            roll_state: None,
            smash: None,
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


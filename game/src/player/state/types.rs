use bevy::prelude::{Component, Query};
use bevy::math::Vec2;
use bevy_rapier2d::prelude::Velocity;

#[derive(Component, Debug)]
pub struct StateTicks(pub u32);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    pub fn sign(&self) -> f32 {
        match self {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        }
    }
}

#[derive(Component, Debug, PartialEq)]
pub enum AirborneState {
    Grounded,
    Airborne,
}

#[derive(Component)]
pub struct AirJumpsRemaining(pub u32);

#[derive(Component, Debug, PartialEq, Copy, Clone)]
pub enum PlayerState {
    Idle,
    Lookup,
    Crouch,
    Running,

    Dash,
    Slide,

    Hitstun,
    Tumble,

    WallGrab,
    WallJump,

    GroundTech,
    WallTech,

    Jumping,
    AirJump,
    AirDodge,
    SpotDodge,

    ControlledAirborne,
    UncontrolledFall,

    SmashDash,
    SmashDrop,

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

    SpinMove,
    GroundPound,
    ChargedPunch,
    Roll,

    SuperCrouch,
    SuperJump,
}

impl PlayerState {
    pub fn has_ground_physics(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Crouch | PlayerState::Running)
    }
}

#[derive(Component, Debug)]
pub struct PreviousState(pub PlayerState);

#[derive(Component, Debug)]
pub enum PlayerAction {
    Jump,
    Attack(Vec2),
    Special(Vec2),
    Grab,
    Dodge,
    None,
}

#[derive(Component, Debug)]
pub struct SpringMass {
    pub y: f32, // spring-mass displacement
    pub vy: f32,
    pub k: f32,       // spring stiffness
    pub damping: f32,
    pub mass: f32,
    pub last_parent_vy: f32,
    pub parent_coupling: f32, // how much parent acceleration effects teh sproing
}

pub fn sproing(mut query: Query<(&mut SpringMass, &Velocity)>) {
    for (mut spring, velocity) in &mut query {
        let parent_vy = velocity.linear.y;

        // parent velocity changes excite teh sproing
        let parent_dv = parent_vy - spring.last_parent_vy;
        spring.vy -= parent_dv * spring.parent_coupling;

        // integrate, in the mathematical sense
        let accel = (-spring.k * spring.y - spring.damping * spring.vy) / spring.mass.max(0.0001);
        spring.vy += accel;
        spring.y += spring.vy;
        spring.last_parent_vy = parent_vy;

        // close enough to zero
        if spring.y.abs() < 0.001 && spring.vy.abs() < 0.001 {
            spring.y = 0.0;
            spring.vy = 0.0;
        }
    }
}

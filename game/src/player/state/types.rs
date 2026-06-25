use bevy::prelude::Component;
use bevy::math::Vec2;

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

    ControlledFall,
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

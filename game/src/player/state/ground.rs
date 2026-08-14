use avian2d::prelude::{Sensor, ShapeHits};
use bevy::prelude::{Changed, Query, With};
use super::types::{AirborneState, AirJumpsRemaining};

/// cos of the max walkable slope. 0.64 ≈ 50°; → 1.0 for flat-only.
const MIN_GROUND_NORMAL_Y: f32 = 0.64;

pub fn detect_ground(
    mut query: Query<(&ShapeHits, &mut AirborneState)>,
    sensors: Query<(), With<Sensor>>,
) {
    for (hits, mut airborne) in &mut query {
        let grounded = hits.iter().any(|hit| {
            !sensors.contains(hit.entity) && hit.normal1.y >= MIN_GROUND_NORMAL_Y
        });

        let next = if grounded { AirborneState::Grounded } else { AirborneState::Airborne };
        // Guarded: `reset_air_jumps` filters on Changed<AirborneState>, so an
        // unconditional write refills your 6 air jumps every single tick.
        if *airborne != next {
            *airborne = next;
        }
    }
}

pub fn reset_air_jumps(mut query: Query<(&AirborneState, &mut AirJumpsRemaining), Changed<AirborneState>>) {
    for (state, mut jumps) in &mut query {
        if matches!(state, AirborneState::Grounded) {
            jumps.0 = 6;
        }
    }
}
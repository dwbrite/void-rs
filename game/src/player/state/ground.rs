use bevy::prelude::{Changed, MessageReader, Query};
use super::types::{AirborneState, AirJumpsRemaining};

// pub fn detect_ground(
//     mut collision_events: MessageReader<CollisionEvent>,
//     mut player_query: Query<&mut AirborneState>,
// ) {
//     for event in collision_events.read() {
//         match event {
//             CollisionEvent::Started(e1, e2, _) => {
//                 println!("ur grounded");
//                 if let Ok(mut airborne) = player_query.get_mut(*e1) {
//                     *airborne = AirborneState::Grounded;
//                 } else if let Ok(mut airborne) = player_query.get_mut(*e2) {
//                     *airborne = AirborneState::Grounded;
//                 }
//             }
//             CollisionEvent::Stopped(e1, e2, _) => {
//                 if let Ok(mut airborne) = player_query.get_mut(*e1) {
//                     *airborne = AirborneState::Airborne;
//                 } else if let Ok(mut airborne) = player_query.get_mut(*e2) {
//                     *airborne = AirborneState::Airborne;
//                 }
//             }
//         }
//     }
// }

pub fn reset_air_jumps(mut query: Query<(&AirborneState, &mut AirJumpsRemaining), Changed<AirborneState>>) {
    for (state, mut jumps) in &mut query {
        if matches!(state, AirborneState::Grounded) {
            jumps.0 = 6;
        }
    }
}
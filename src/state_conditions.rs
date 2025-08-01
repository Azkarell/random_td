use bevy::{
    ecs::{
        query::With,
        schedule::{IntoScheduleConfigs, ScheduleConfigs},
        system::{IntoSystem, Query, Res, ResMut, ScheduleSystem, System},
    },
    log::info,
    state::state::NextState,
};

use crate::{
    GameState,
    enemy::{Enemy, SpawnCounter},
    player::Player,
    stats::Health,
};

pub fn wave_done(
    spawn: Res<SpawnCounter>,
    enemies: Query<(), With<Enemy>>,
    player: Query<&Health, With<Player>>,
) -> bool {
    (!spawn.can_spawn() && enemies.is_empty()) || player.iter().all(|h| h.0 <= 0.0)
}

pub fn change_state(state: GameState) -> ScheduleConfigs<ScheduleSystem> {
    (move |mut next_state: ResMut<NextState<GameState>>| {
        next_state.set(state);
        info!("changing state: {state:?}");
    })
    .into_configs()
}

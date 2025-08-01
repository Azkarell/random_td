use bevy::ecs::{
    component::Component,
    event::Event,
    observer::Trigger,
    query::With,
    system::{Commands, Query, Single},
};

use crate::{assets::PLAYER_INITIAL_GOLD, stats::Health};

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Gold(pub u32);

#[derive(Event)]
pub struct GoldGained {
    pub amount: u32,
}

pub fn game_running(player: Query<&Health, With<Player>>) -> bool {
    player.iter().any(|h| h.0 > 0.0)
}

pub fn setup_player(mut commands: Commands) {
    commands.spawn((Player, Gold(PLAYER_INITIAL_GOLD), Health(10.0)));
}

pub fn on_gold_gained(trigger: Trigger<GoldGained>, mut query: Single<&mut Gold, With<Player>>) {
    query.0 += trigger.event().amount;
}

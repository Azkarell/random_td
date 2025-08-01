use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};

use crate::{
    assets::{ALIVE_ENEMIES_ICON, FONT, FONT_SIZE, GOLD_IMAGE_ICON, HEART_IMAGE},
    enemy::Enemy,
    player::{Gold, Player},
    stats::Health,
};

pub mod debug {
    use bevy::prelude::*;

    use crate::input::{InputSet, MouseWorldPos};

    pub struct DebugUiOverlay;

    impl Plugin for DebugUiOverlay {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, debug_ui_overlay.after(InputSet));
            app.add_systems(Update, update_debug_ui_overlay.after(InputSet));
        }
    }
    #[derive(Component)]
    pub struct MousePositionText;

    fn debug_ui_overlay(mut commands: Commands, mouse_position: Res<MouseWorldPos>) {
        let text = if let Some(p) = **mouse_position {
            format!("mouse: {},{}", p.x, p.y)
        } else {
            "mouse: None".to_string()
        };
        commands.spawn((
            Text::new(text),
            MousePositionText,
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..Default::default()
            },
        ));
    }

    fn update_debug_ui_overlay(
        query: Query<&mut Text, With<MousePositionText>>,
        mouse_position: Res<MouseWorldPos>,
    ) {
        if mouse_position.is_changed() {
            let text = if let Some(p) = **mouse_position {
                format!("mouse: {},{}", p.x, p.y)
            } else {
                "mouse: None".to_string()
            };
            for mut t in query {
                t.0 = text.clone();
            }
        }
    }
}
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiSet;
pub struct UiOverlay;
impl Plugin for UiOverlay {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiFontSize(FONT_SIZE));
        app.add_systems(Startup, prepare_ui_overlay);
        app.add_systems(
            Update,
            (update_gold_label, update_health_label, update_enemies_count).in_set(UiSet),
        );
    }
}

#[derive(Resource, Deref)]
pub struct UiFont(pub Handle<Font>);
#[derive(Resource, Deref)]
pub struct UiFontSize(pub f32);
#[derive(Resource, Deref)]
pub struct UiNode(pub Entity);
#[derive(Component)]
pub struct HealthBar;
#[derive(Component)]
pub struct HealthTextLabel;
#[derive(Component)]
pub struct GoldTextLabel;
#[derive(Component)]
pub struct AliveEnemiesLabel;

// TODO: only works for one player for now
pub fn update_gold_label(
    gold_query: Query<&mut Text, With<GoldTextLabel>>,
    player: Query<&Gold, With<Player>>,
) {
    if let Ok(player_gold) = player.single() {
        for mut t in gold_query {
            t.0 = format!("{}", player_gold.0)
        }
    }
}
// TODO: only works for one player for now
pub fn update_health_label(
    gold_query: Query<(&mut Text), With<HealthTextLabel>>,
    player: Query<&Health, With<Player>>,
) {
    if let Ok(player_health) = player.single() {
        for mut t in gold_query {
            t.0 = format!("{}", player_health.0)
        }
    }
}

pub fn update_enemies_count(
    text_query: Query<&mut Text, With<AliveEnemiesLabel>>,
    enemies_query: Query<(Entity), With<Enemy>>,
) {
    let count = enemies_query.iter().count();
    for mut t in text_query {
        t.0 = format!("{count}");
    }
}
pub fn prepare_ui_overlay(mut commands: Commands, assets: Res<AssetServer>) {
    let font = assets.load(FONT);
    let gold_image = assets.load(GOLD_IMAGE_ICON);
    let heart_image: Handle<Image> = assets.load(HEART_IMAGE);
    let alive_enemies_icon: Handle<Image> = assets.load(ALIVE_ENEMIES_ICON);
    commands.insert_resource(UiFont(font.clone()));

    let e = commands
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::ColumnReverse,
                row_gap: Val::Px(5.0),
                column_gap: Val::Px(5.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    Node {
                        display: Display::Flex,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::FlexEnd,
                        justify_items: JustifyItems::Center,
                        padding: UiRect::all(Val::Px(15.0)),
                        flex_direction: FlexDirection::RowReverse,
                        ..Default::default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(spawn_icon_with_text(
                    gold_image,
                    font.clone(),
                    GoldTextLabel,
                    "0".to_owned(),
                    Val::Auto,
                    Val::Px(20.0),
                    Val::Px(10.0),
                    Val::Auto,
                    Val::Auto,
                    Val::Px(10.0),
                    Val::Px(65.0),
                    Val::Auto,
                    40.0,
                    40.0,
                    FONT_SIZE,
                    40.0,
                    Color::hsl(240.0, 1.0, 1.0),
                ))
                .with_children(spawn_icon_with_text(
                    heart_image,
                    font.clone(),
                    HealthTextLabel,
                    "0".to_owned(),
                    Val::Auto,
                    Val::Px(20.0),
                    Val::Px(85.0),
                    Val::Auto,
                    Val::Auto,
                    Val::Px(10.0),
                    Val::Px(135.0),
                    Val::Auto,
                    40.0,
                    40.0,
                    FONT_SIZE,
                    40.0,
                    Color::hsl(240.0, 1.0, 1.0),
                ))
                .with_children(spawn_icon_with_text(
                    alive_enemies_icon,
                    font.clone(),
                    AliveEnemiesLabel,
                    "0".to_owned(),
                    Val::Auto,
                    Val::Px(20.0),
                    Val::Auto,
                    Val::Px(50.0),
                    Val::Auto,
                    Val::Px(5.0),
                    Val::Auto,
                    Val::Px(10.0),
                    40.0,
                    40.0,
                    FONT_SIZE,
                    40.0,
                    Color::hsl(240.0, 0.8, 0.8),
                ));
        })
        .id();

    commands.insert_resource(UiNode(e));
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_icon_with_text(
    icon: Handle<Image>,
    font: Handle<Font>,
    text_label: impl Component,
    initial_text: String,
    text_position_bottom: Val,
    text_position_top: Val,
    text_position_right: Val,
    text_position_left: Val,
    icon_position_bottom: Val,
    icon_position_top: Val,
    icon_position_right: Val,
    icon_position_left: Val,
    text_width: f32,
    text_height: f32,
    font_size: f32,
    icon_size: f32,
    color: Color,
) -> impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>) {
    move |builder| {
        builder.spawn((
            Node {
                display: Display::Block,
                height: Val::Px(text_height),
                top: text_position_top,
                right: text_position_right,
                bottom: text_position_bottom,
                left: text_position_left,
                position_type: PositionType::Absolute,
                width: Val::Px(text_width),
                ..Default::default()
            },
            Text::new(initial_text),
            TextFont::default()
                .with_font(font.clone())
                .with_font_size(font_size),
            text_label,
        ));

        builder.spawn((
            Node {
                display: Display::Block,
                width: Val::Px(icon_size),
                justify_self: JustifySelf::Center,
                top: icon_position_top,
                right: icon_position_right,
                bottom: icon_position_bottom,
                left: icon_position_left,
                position_type: PositionType::Absolute,
                height: Val::Px(icon_size),
                ..Default::default()
            },
            ImageNode::new(icon).with_color(color),
        ));
    }
}

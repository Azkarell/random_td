use bevy::{
    app::{Plugin, Update},
    ecs::{
        resource::Resource,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Commands, Query},
    },
    math::Vec2,
    prelude::{Deref, DerefMut},
    render::camera::Camera,
    transform::components::GlobalTransform,
    window::Window,
};

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Update, update_world_pos.in_set(InputSet));
        app.insert_resource(MouseWorldPos(None));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct MouseWorldPos(pub Option<Vec2>);
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub fn update_world_pos(
    mut commands: Commands,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (c, t) = camera.single().expect("Camera should be singelton");
    let w = window.single().expect("window should be singelton");
    let pos = w
        .cursor_position()
        .and_then(|cursor| c.viewport_to_world_2d(t, cursor).ok());
    commands.insert_resource(MouseWorldPos(pos));
}

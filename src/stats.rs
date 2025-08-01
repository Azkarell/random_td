use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::Deref,
    time::Timer,
};

#[derive(Component, Deref, Clone, Copy)]
pub struct Damage(pub f32);
#[derive(Component, Deref, Clone, Copy)]
pub struct Range(pub f32);
#[derive(Component)]
pub struct FireRate(pub f32, pub Option<Timer>);
#[derive(Component)]
pub struct Health(pub f32);
#[derive(Component)]
pub struct Speed(pub f32);
#[derive(Component)]
pub struct Armor(pub f32);

#[derive(Resource, Deref, Debug)]
pub struct Wave(pub u32);

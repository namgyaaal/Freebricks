use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use glam::{Quat, Vec3};

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Position(pub Vec3);

impl Default for Position {
    fn default() -> Self {
        Position(Vec3::ZERO)
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Rotation(pub Quat);

impl Default for Rotation {
    fn default() -> Self {
        Rotation(Quat::IDENTITY)
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Size(pub Vec3);

impl Default for Size {
    fn default() -> Self {
        Size(Vec3::new(4.0, 1.0, 2.0))
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Color(pub [u8; 4]);

impl Default for Color {
    fn default() -> Self {
        Color([128, 128, 128, 255])
    }
}

use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use rapier3d::prelude::*;
use std::collections::HashSet;

use crate::ecs::common::{Position, Rotation, Size};

#[derive(Component, Debug)]
pub struct Physical {
    pub anchored: bool,
}

impl Default for Physical {
    fn default() -> Self {
        Physical { anchored: true }
    }
}

impl Physical {
    pub fn dynamic() -> Self {
        Physical { anchored: false }
    }

    pub fn anchored() -> Self {
        Physical::default()
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BodyHandle(pub RigidBodyHandle);

#[derive(Component, Debug, Clone, Copy)]
pub struct ShapeHandle(pub ColliderHandle);

#[derive(Component, Debug)]
pub struct AnchoredTo(pub HashSet<Entity>);

#[derive(Component, Debug)]
pub struct AnchorSource;

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct QPhysics {
    pub entity: Entity,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub physical: &'static Physical,
}

#[derive(Event)]
pub struct PhysicsCleanup {
    pub entity: Entity,
    pub shape: Option<ShapeHandle>,
    pub body: Option<BodyHandle>,
    pub parent: Option<Entity>,
}

use bevy_ecs::query::QueryData;
use bevy_ecs::{prelude::*, query::QueryFilter};
use bevy_platform::collections::HashSet;
use rapier3d::prelude::*;

use crate::ecs::common::{Position, Rotation, Size};

#[derive(Component, Debug, Default)]
pub struct Physical;
#[derive(Component, Debug, Default)]
#[require(Physical)]
pub struct Anchor;

#[derive(Component, Debug)]
pub struct Anchored(pub HashSet<Entity>);

#[derive(Component, Debug, Clone, Copy)]
pub struct BodyHandle(pub RigidBodyHandle);

#[derive(Component, Debug, Clone, Copy)]
pub struct ShapeHandle(pub ColliderHandle);

/*  ---------------------

    Queries

*/

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct QPhysics {
    pub entity: Entity,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub physical: &'static Physical,
}

/*  ---------------------

    Filters

*/

#[derive(QueryFilter)]
pub struct FAnchored {
    generic_tuple: (With<Anchored>, Without<Anchor>),
}

#[derive(QueryFilter)]
pub struct FUnanchored {
    generic_tuple: (Without<Anchored>, Without<Anchor>),
}

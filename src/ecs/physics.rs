use bevy_ecs::query::QueryData;
use bevy_ecs::{prelude::*, query::QueryFilter};
use bevy_platform::collections::HashSet;
use rapier3d::prelude::*;

use crate::ecs::common::{Position, Rotation, Size};

#[derive(Component, Debug, Default)]
/// Tag indicates that it should be integrated into physics engine.
pub struct Physical;

#[derive(Component, Debug, Default)]
#[require(Physical)]
/// Tag indicates that it is an anchor source.
/// (Note: Anchor is equivalent to anchored in Roblox, bricks that aren't guranteed to be "in place" but are
///     due to being snapped to 'anchored' bricks are considered anchored in our terminology, as anchors are the 'ground
///     truth' for them).
pub struct Anchor;

#[derive(Component, Debug)]
/// Anchored with a hashset of the anchor sources (There could be multiple!)
/// bevy_ecs does not yet support many-to-many relations, so this is a workaround until it does.
pub struct Anchored(pub HashSet<Entity>);

#[derive(Component, Debug, Clone, Copy)]
/// Component wrapper around rigid body handle.;
pub struct BodyHandle(pub RigidBodyHandle);

#[derive(Component, Debug, Clone, Copy)]
/// Component wrapper around collider handle.
pub struct ShapeHandle(pub ColliderHandle);

/*  ---------------------

    Queries

*/

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Query for spatial data agnostic to what it is.
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
/// Is it anchored filter
pub struct FAnchored {
    generic_tuple: (With<Anchored>, Without<Anchor>),
}

#[derive(QueryFilter)]
/// Is it unanchored filter
pub struct FUnanchored {
    generic_tuple: (Without<Anchored>, Without<Anchor>),
}

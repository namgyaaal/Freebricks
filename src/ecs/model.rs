use bevy_ecs::prelude::*;
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_platform::collections::HashSet;
use core::fmt;
use petgraph::prelude::UnGraphMap;
use std::fmt::Debug;

use crate::ecs::physics::BodyHandle;

#[derive(Component)]
/// Component for model. Keeps track of parts as an undirected graph and keeps track of anchors connected to parts with a hashset.
pub struct Model {
    pub graph: UnGraphMap<Entity, ()>,
    pub anchors: HashSet<Entity>,
    // Set to true when parts are deleted, prevents graph traversals when the graph isn't modified.
    pub dirty: bool,
}

// UnMatrix doesn't impl Debug :(
impl Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Model").field("graph", &"UnMatrix").finish()
    }
}

/*  ---------------------

    Queries

*/

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Standard Model Query
pub struct QModel {
    pub entity: Entity,
    pub model: &'static Model,
    pub children: &'static Children,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
/// Standard Mutable Model Query
pub struct QModelUpdate {
    pub entity: Entity,
    pub model: &'static mut Model,
    pub children: &'static mut Children,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Model Query + RigidBody Query
pub struct QModelPhysics {
    pub entity: Entity,
    pub model: &'static Model,
    pub children: &'static Children,
    pub body: &'static BodyHandle,
}

/*  ---------------------

    Filters

*/

#[derive(QueryFilter)]
/// On Model Add Filter
pub struct FModelAdd {
    _c: Added<Model>,
}

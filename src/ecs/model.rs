use bevy_ecs::prelude::*;
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_platform::collections::HashSet;
use core::fmt;
use petgraph::prelude::UnGraphMap;
use std::fmt::Debug;

use crate::ecs::physics::BodyHandle;

#[derive(Component)]
pub struct Model {
    pub set: HashSet<Entity>,
    pub graph: UnGraphMap<Entity, ()>,
    pub anchored: HashSet<Entity>,
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
pub struct QModel {
    pub entity: Entity,
    pub model: &'static Model,
    pub children: &'static Children,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct QModelUpdate {
    pub entity: Entity,
    pub model: &'static mut Model,
    pub children: &'static mut Children,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
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
pub struct FModelAdd {
    _c: Added<Model>,
}

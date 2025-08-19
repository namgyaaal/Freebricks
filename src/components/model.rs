use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use petgraph::matrix_graph::UnMatrix;
use petgraph::prelude::UnGraphMap;
use rapier3d::prelude::RigidBodyHandle;
use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use crate::components::physics::BodyHandle;

#[derive(Eq, PartialEq)]
pub enum ModelEnd {
    Destructure, 
    Delete, 
}

#[derive(Component)]
pub struct Model {
    pub set: HashSet<Entity>,
    pub graph: UnGraphMap<Entity, ()>,    
    pub anchored: HashSet<Entity> 
}

// UnMatrix doesn't impl Debug :(
impl Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Model")
            .field("graph", &"UnMatrix")
            .finish()
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct ModelQuery {
    pub entity: Entity, 
    pub model: &'static Model,
    pub children: &'static Children,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct ModelPhysicsQuery {
    pub entity: Entity, 
    pub model: &'static Model,
    pub children: &'static Children,
    pub body: &'static BodyHandle
}


#[derive(Event)]
pub struct ModelCleanup {
    pub handle: RigidBodyHandle,
    pub children: Vec<u32>,
    pub mode: ModelEnd
}

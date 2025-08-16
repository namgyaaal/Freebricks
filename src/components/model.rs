use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use petgraph::matrix_graph::UnMatrix;
use core::fmt;
use std::collections::HashSet;
use std::fmt::Debug;


#[derive(Component)]
pub struct Model {
    pub set: HashSet<Entity>,
    pub graph: UnMatrix<Entity, ()>
}

// UnMatrix doesn't impl Debug :(
impl Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Model")
            .field("set", &self.set) 
            .field("graph", &"UnMatrix")
            .finish()
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct ModelQuery {
    pub entity: Entity, 
    pub model: &'static Model
}
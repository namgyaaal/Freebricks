use bevy_ecs::prelude::*;
use std::collections::HashSet;

#[derive(Component, Debug)]
pub struct Model {
    pub set: HashSet<Entity>,
}


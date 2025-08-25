use std::ops::DerefMut;

use bevy_ecs::prelude::*;

use crate::{
    ecs::physics::{BodyHandle, ShapeHandle},
    physics::{AnchorMap, physics_state::PhysicsState},
};

pub fn handle_shape_removal(
    trigger: Trigger<OnRemove, ShapeHandle>,
    mut state: ResMut<PhysicsState>,
    mut anchor_map: ResMut<AnchorMap>,
    shapes: Query<&ShapeHandle>,
) {
    let entity = trigger.target();

    let state = state.deref_mut();
    let handle = shapes
        .get(entity)
        .expect("Couldn't get shape in handle_shape_removal");

    state.colliders.remove(
        handle.0,
        &mut state.island_manager,
        &mut state.rigid_bodies,
        true,
    );

    let anchor_map = anchor_map.deref_mut();
    let anchors = &mut anchor_map.anchors;

    if anchors.contains_key(&entity) {
        anchor_map.delete_queue.push_back(entity);
    }
}

pub fn handle_body_removal(
    trigger: Trigger<OnRemove, BodyHandle>,
    mut state: ResMut<PhysicsState>,
    bodies: Query<&BodyHandle>,
    children: Query<&Children>,
) {
    let state = state.deref_mut();
    let handle = bodies
        .get(trigger.target())
        .expect("Couldn't get body in handle_body_removal");

    let remove_colliders = children.get(trigger.target()).is_ok();

    let _ = state.rigid_bodies.remove(
        handle.0,
        &mut state.island_manager,
        &mut state.colliders,
        &mut state.impulse_joint_set,
        &mut state.multibody_joint_set,
        remove_colliders,
    );
}

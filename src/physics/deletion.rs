use std::ops::DerefMut;

use bevy_ecs::prelude::*;

use crate::{
    ecs::physics::{Anchored, BodyHandle, ShapeHandle},
    physics::{AnchorMap, physics_state::PhysicsState},
};

/// Observer function for when a shape is removed. Removes it from the colliderset.
/// If it's an anchor, this function pushes the removed entity onto anchormap's deletion queue to be processed
///     when handle_anchor_queue() is called.
/// If it's anchored, this function will remove itself from the relevant anchormap keys.
pub fn handle_shape_removal(
    trigger: Trigger<OnRemove, ShapeHandle>,
    mut state: ResMut<PhysicsState>,
    mut anchor_map: ResMut<AnchorMap>,
    shapes: Query<&ShapeHandle>,
    anchoreds: Query<&Anchored>,
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

    // Handle if anchor
    if anchors.contains_key(&entity) {
        anchor_map.delete_queue.push_back(entity);
    }
    // Handle if anchored
    if let Ok(anchored) = anchoreds.get(entity) {
        for anchor in &anchored.0 {
            anchors.entry(*anchor).and_modify(|set| {
                set.remove(&entity);
            });

            if let Some(set) = anchors.get(anchor)
                && set.is_empty()
            {
                anchors.remove(anchor);
            }
        }
    }
}

/// Observer function for when a rigid body is removed.
/// Has specific logic for handling removing a model by keeping it's children or not by whether or not the
///     Children component exists.
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

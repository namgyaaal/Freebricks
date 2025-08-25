use std::ops::DerefMut;

use bevy_ecs::prelude::*;
use bevy_platform::collections::{HashMap, HashSet};
use rapier3d::{na::Isometry, prelude::*};

use crate::{
    ecs::{
        model::{FModelAdd, Model, QModel},
        physics::{Anchor, Anchored, BodyHandle, ShapeHandle},
    },
    physics::{AnchorMap, PhysicsState},
};

pub fn handle_subpart(
    mut commands: Commands,
    mut state: ResMut<PhysicsState>,
    mut freed: RemovedComponents<ChildOf>,
    shapes: Query<&ShapeHandle>,
    new_parent: Query<&ChildOf>,
    anchored: Query<&Anchored>,
) -> Result<()> {
    let state = state.deref_mut();

    for part_id in freed.read() {
        if shapes.get(part_id).is_err() || new_parent.get(part_id).is_ok() {
            continue;
        }
        let shape_handle = shapes.get(part_id).unwrap().0;

        let builder = {
            if anchored.get(part_id).is_ok() {
                RigidBodyBuilder::fixed()
            } else {
                RigidBodyBuilder::dynamic()
            }
        };

        let (pos, (yaw, pitch, roll)) = {
            let shape = state
                .colliders
                .get(shape_handle)
                .ok_or("Couldn't get collider")?;
            (
                shape.translation().to_owned(),
                shape.rotation().euler_angles(),
            )
        };

        let new_body = builder
            .user_data(part_id.to_bits() as u128)
            .translation(pos)
            .rotation(vector![yaw, pitch, roll])
            .build();
        let new_handle = state.rigid_bodies.insert(new_body);
        state
            .colliders
            .set_parent(shape_handle, Some(new_handle), &mut state.rigid_bodies);
        let shape = state
            .colliders
            .get_mut(shape_handle)
            .ok_or("Couldn't get collider")?;
        shape.set_position(Isometry::identity());

        commands.entity(part_id).insert(BodyHandle(new_handle));
    }

    Ok(())
}

pub fn handle_submodel(
    mut commands: Commands,
    mut state: ResMut<PhysicsState>,
    models: Query<QModel, FModelAdd>,
    bodies: Query<&BodyHandle>,
    mut shapes: Query<&mut ShapeHandle>,
) -> Result<()> {
    let state = state.deref_mut();

    for item in models {
        if bodies.get(item.entity).is_ok() {
            continue;
        }

        let new_body = {
            if item.model.anchors.is_empty() {
                RigidBodyBuilder::dynamic()
            } else {
                RigidBodyBuilder::fixed()
            }
        }
        .user_data(item.entity.to_bits() as u128)
        .build();

        let new_handle = state.rigid_bodies.insert(new_body);

        for &child in item.children {
            let mut shape_handle = shapes.get_mut(child)?;
            let shape = state
                .colliders
                .remove(
                    shape_handle.0,
                    &mut state.island_manager,
                    &mut state.rigid_bodies,
                    true,
                )
                .ok_or("Couldn't get collider")?;

            let handle =
                state
                    .colliders
                    .insert_with_parent(shape, new_handle, &mut state.rigid_bodies);
            *shape_handle = ShapeHandle(handle);
        }
        commands.entity(item.entity).insert(BodyHandle(new_handle));
    }

    Ok(())
}

pub fn handle_anchor_queue(
    mut commands: Commands,
    mut anchor_map: ResMut<AnchorMap>,
    child: Query<&ChildOf>,
    mut anchoreds: Query<&mut Anchored>,
    mut models: Query<&mut Model>,
) -> Result<()> {
    if !anchor_map.is_changed() || anchor_map.delete_queue.is_empty() {
        return Ok(());
    }
    let anchor_map = anchor_map.deref_mut();

    let mut changed_parts = HashMap::new();

    let anchors: Vec<Entity> = anchor_map.delete_queue.drain(..).collect();
    for anchor_id in anchors {
        let anchored_ids = anchor_map
            .anchors
            .remove(&anchor_id)
            .ok_or("Couldn't get anchored")?;

        for anchored_id in anchored_ids {
            anchoreds.get_mut(anchored_id)?.0.remove(&anchor_id);
            let set = changed_parts.entry(anchored_id).or_insert(HashSet::new());
            set.insert(anchor_id);
        }
    }

    for (changed_part_id, anchors) in changed_parts {
        let anchored = anchoreds.get_mut(changed_part_id)?;
        if anchored.0.len() != 0 {
            continue;
        }
        commands.entity(changed_part_id).remove::<Anchored>();

        let Ok(child_of) = child.get(changed_part_id) else {
            continue;
        };
        let parent_id = child_of.0;

        let model_item = &mut models.get_mut(parent_id)?;
        for anchor in anchors {
            model_item.anchors.remove(&anchor);
        }
    }

    Ok(())
}

/// Only handle parts that aren't under modes
pub fn handle_part_unanchor(
    mut state: ResMut<PhysicsState>,
    mut removed: RemovedComponents<Anchor>,
    bodies: Query<&BodyHandle, Without<ChildOf>>,
) -> Result<()> {
    for part_id in removed.read() {
        let Ok(body_handle) = bodies.get(part_id) else {
            continue;
        };

        let body = state
            .rigid_bodies
            .get_mut(body_handle.0)
            .ok_or("Couldn't get rigid body")?;

        if body.body_type() == RigidBodyType::Fixed {
            body.set_body_type(RigidBodyType::Dynamic, true);
        }
    }
    Ok(())
}

pub fn handle_model_unanchor(
    mut state: ResMut<PhysicsState>,
    modified_models: Query<(Entity, &Model), Changed<Model>>,
    bodies: Query<&BodyHandle>,
) -> Result<()> {
    for (model_id, model) in modified_models {
        if !model.anchors.is_empty() {
            continue;
        }
        let body_handle = bodies.get(model_id)?;
        let body = state
            .rigid_bodies
            .get_mut(body_handle.0)
            .ok_or("Couldn't get rigid body")?;

        if body.body_type() == RigidBodyType::Fixed {
            body.set_body_type(RigidBodyType::Dynamic, true);
        }
    }

    Ok(())
}

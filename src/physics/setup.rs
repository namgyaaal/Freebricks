use std::ops::DerefMut;

use crate::{
    ecs::{
        model::{FModelAdd, QModel},
        parts::{FPartAdd, Part},
        physics::{
            Anchor, Anchored, BodyHandle, QPhysics, QPhysicsItem, QPhysicsReadOnly,
            QPhysicsReadOnlyItem, ShapeHandle,
        },
    },
    physics::physics_state::PhysicsState,
};
use bevy_ecs::prelude::*;
use rapier3d::prelude::*;

/// Build physics information for parts not under a model.
/// There are three types of parts we need to worry about.
///     (1) Unanchored parts connected to anchors
///     (2) Unanchored parts not connnected to anchors
///     (3) Anchored parts
/// Argument snapped handles (1) while unsnapped handles (2) and (3).
///
/// Because rapier3d supports rigidbody-less colliders, we give (3) colliders only while (1) and (2) get rigid bodies alongside
///     colliders. This saves performance if we have a scene with a lot of parts that are anchored, since they don't need rigid bodies.
pub fn setup_parts(
    mut commands: Commands,
    mut state: ResMut<PhysicsState>,
    snapped: Query<QPhysics, (Without<ChildOf>, With<Anchored>, FPartAdd)>,
    unsnapped: Query<QPhysics, (Without<ChildOf>, Without<Anchored>, FPartAdd)>,
    is_anchor: Query<&Anchor>,
) {
    let state = state.deref_mut();

    // Anchored
    for part in &snapped {
        build_body(&mut commands, state, part, RigidBodyBuilder::fixed());
    }
    // Anchors and unanchored
    for part in &unsnapped {
        if is_anchor.get(part.entity).is_ok() {
            build_shape(&mut commands, state, part);
        } else {
            build_body(&mut commands, state, part, RigidBodyBuilder::dynamic());
        }
    }
}

pub fn setup_models(
    mut commands: Commands,
    mut state: ResMut<PhysicsState>,
    models: Query<QModel, FModelAdd>,
    parts: Query<QPhysics>,
) -> Result<()> {
    let state = state.deref_mut();

    for item in models {
        let mut shapes = Vec::new();
        for &child_id in item.children {
            let child = parts.get(child_id)?;
            let shape = get_shape(&child, true);
            shapes.push((child_id, shape));
        }
        let body = if item.model.anchored.is_empty() {
            RigidBodyBuilder::dynamic()
        } else {
            RigidBodyBuilder::fixed()
        }
        .user_data(item.entity.to_bits() as u128)
        .build();

        let body_handle = state.rigid_bodies.insert(body);
        for (child, shape) in shapes {
            let shape_handle =
                state
                    .colliders
                    .insert_with_parent(shape, body_handle, &mut state.rigid_bodies);
            commands.entity(child).insert(ShapeHandle(shape_handle));
        }
        commands.entity(item.entity).insert(BodyHandle(body_handle));
    }
    Ok(())
}

/*
    Helper functions
*/

/// Shorthand util to get collider with relevant data in it
fn get_shape(part: &QPhysicsReadOnlyItem, full: bool) -> Collider {
    let size = part.size.0 / 2.0;

    let mut builder = ColliderBuilder::cuboid(size.x, size.y, size.z).restitution(0.4);
    if full {
        let pos = part.position;
        let (yaw, pitch, roll) = {
            let (axis, angle) = part.rotation.to_axis_angle();
            (axis.x * angle, axis.y * angle, axis.z * angle)
        };
        builder = builder
            .translation(vector![pos.x, pos.y, pos.z])
            .rotation(vector![yaw, pitch, roll])
    }
    builder.build()
}

/// Add component for collision shape for a given part
fn build_shape(commands: &mut Commands, state: &mut PhysicsState, part: QPhysicsReadOnlyItem) {
    let shape = get_shape(&part, true);
    let shape_handle = state.colliders.insert(shape);
    commands
        .entity(part.entity)
        .insert(ShapeHandle(shape_handle));
}

/// Add components for rigid body and collision shape for a given part
fn build_body(
    commands: &mut Commands,
    state: &mut PhysicsState,
    part: QPhysicsReadOnlyItem,
    builder: RigidBodyBuilder,
) {
    let pos = part.position;
    let (yaw, pitch, roll) = {
        // Quat -> Euler memery
        let (axis, angle) = part.rotation.to_axis_angle();
        (axis.x * angle, axis.y * angle, axis.z * angle)
    };

    let shape = get_shape(&part, false);

    let body = builder
        .translation(vector![pos.x, pos.y, pos.z])
        .rotation(vector![yaw, pitch, roll])
        .user_data(part.entity.to_bits() as u128)
        .build();

    let body_handle = state.rigid_bodies.insert(body);
    let shape_handle =
        state
            .colliders
            .insert_with_parent(shape, body_handle, &mut state.rigid_bodies);

    commands
        .entity(part.entity)
        .insert(BodyHandle(body_handle))
        .insert(ShapeHandle(shape_handle));
}

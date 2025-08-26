use glam::Vec3;
use rapier3d::prelude::*;
mod test_utils;
use crate::test_utils::*;

#[test]
pub fn one_brick() {
    let message = "Testing one brick";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = spawn_p(&mut world, false, Vec3::new(0.0, 0.0, 0.0));
    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, false, false, false, false, true, true,
    );
    collider_check(&mut world, message, entity);
    body_check(&mut world, message, entity, RigidBodyType::Dynamic);
}

#[test]
pub fn one_anchored_brick() {
    let message = "Testing one brick";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = spawn_p(&mut world, true, Vec3::new(0.0, 0.0, 0.0));
    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, false, true, false, false, true, false,
    );
    collider_check(&mut world, message, entity);
}

#[test]
pub fn brick_touching_anchor() {
    let message = "Testing one brick";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = spawn_p(&mut world, false, Vec3::new(0.0, 0.0, 0.0));
    let anchor = spawn_p(&mut world, true, Vec3::new(0.0, 1.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, true, false, false, false, true, true,
    );
    body_check(&mut world, message, entity, RigidBodyType::Fixed);

    guarantee(
        &mut world, message, anchor, false, true, false, false, true, false,
    );
    collider_check(&mut world, message, entity);
}

#[test]
pub fn delete_anchor() {
    let message = "Testing deleting an anchor a brick is anchored to";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = spawn_p(&mut world, false, Vec3::new(0.0, 0.0, 0.0));
    let anchor = spawn_p(&mut world, true, Vec3::new(0.0, 1.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    world.despawn(anchor);

    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, false, false, false, false, true, true,
    );
    body_check(&mut world, message, entity, RigidBodyType::Dynamic);
}

#[test]
pub fn multi_anchor_delete() {
    let message = "Testing deleting two anchors a brick is anchored to";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = spawn_p(&mut world, false, Vec3::new(0.0, 0.0, 0.0));
    let anchor1 = spawn_p(&mut world, true, Vec3::new(0.0, 1.0, 1.0));
    let anchor2 = spawn_p(&mut world, true, Vec3::new(0.0, 1.0, -1.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    world.despawn(anchor1);

    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, true, false, false, false, true, true,
    );
    body_check(&mut world, message, entity, RigidBodyType::Fixed);

    world.despawn(anchor2);
    sched_update.run(&mut world);

    guarantee(
        &mut world, message, entity, false, false, false, false, true, true,
    );
    body_check(&mut world, message, entity, RigidBodyType::Dynamic);
}

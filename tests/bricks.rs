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

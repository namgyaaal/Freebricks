use bevy_ecs::prelude::*;
use freebricks::ecs::parts::{StudInfo, StudType};
use glam::Vec3;
use rapier3d::prelude::*;
mod test_utils;
use crate::test_utils::*;

#[test]
pub fn shouldnt_snap() {
    let message = "Testing bricks not aligned with eachother";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 1.0, 0.0)];

    let _: Vec<Entity> = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 0, "{} - Models shouldn't exist", message);
}

#[test]
pub fn shouldnt_snap_intersecting() {
    let message = "Testing bricks not aligned with eachother because of intersection";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.5, 0.0)];

    let _: Vec<Entity> = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 0, "{} - Models shouldn't exist", message);
}

#[test]
pub fn smooth_bricks() {
    let message = "Testing bricks with smooth surfaces";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];

    let _: Vec<Entity> = positions
        .iter()
        .map(|position| {
            spawn_pstuds(
                &mut world,
                false,
                *position,
                StudInfo {
                    top: StudType::Flat,
                    bottom: StudType::Flat,
                },
            )
        })
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 0, "{} - Models shouldn't exist", message);
}

#[test]
pub fn disjoint_bricks() {
    let message = "Testing bricks with inlet surfaces";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];

    let _: Vec<Entity> = positions
        .iter()
        .map(|position| {
            spawn_pstuds(
                &mut world,
                false,
                *position,
                StudInfo {
                    top: StudType::Inlet,
                    bottom: StudType::Inlet,
                },
            )
        })
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 0, "{} - Models shouldn't exist", message);
}

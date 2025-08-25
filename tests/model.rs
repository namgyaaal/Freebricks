use bevy_ecs::prelude::*;
use freebricks::ecs::model::Model;
use glam::Vec3;
use rapier3d::prelude::*;
mod test_utils;
use crate::test_utils::*;
#[test]
pub fn separate_bricks() {
    let message = "Testing multiple separate bricks";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.0, 4.0, 0.0),
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(8.0, 0.0, 0.0),
        Vec3::new(8.0, 2.0, 0.0),
        Vec3::new(0.0, 4.0, 8.0),
        Vec3::new(0.0, 6.0, 8.0),
    ];

    let entities: Vec<Entity> = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(
            &mut world, message, entity, false, false, false, false, true, true,
        );
        collider_check(&mut world, message, entity);
        body_check(&mut world, message, entity, RigidBodyType::Dynamic);
    }

    assert_eq!(
        get_models(&mut world).len(),
        0,
        "{} - Models exist",
        message
    );
}

#[test]
pub fn connected_bricks() {
    let message = "Testing multiple separate bricks";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 4.0, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(8.0, 0.0, 0.0),
        Vec3::new(8.0, 1.0, 0.0),
        Vec3::new(0.0, 4.0, 8.0),
        Vec3::new(0.0, 5.0, 8.0),
    ];

    let entities: Vec<Entity> = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(
            &mut world, message, entity, false, false, true, false, true, false,
        );
        collider_check(&mut world, message, entity);
    }

    let models = get_models(&mut world);

    assert_eq!(models.len(), 4, "{} - Four models don't exist", message);
    for model_id in models {
        guarantee_model(&mut world, message, model_id, 2, 0, 1);
    }
}

#[test]
pub fn model_touches_anchor() {
    let message = "Testing multiple separate bricks";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];

    let _ = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();
    spawn_p(&mut world, true, Vec3::new(0.0, 3.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();
    guarantee_model(&mut world, message, model_id, 3, 1, 2);
    body_check(&mut world, message, model_id, RigidBodyType::Fixed);
}

#[test]
pub fn model_anchor_deletion() {
    let message = "Testing multiple separate bricks";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];

    let _ = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();
    let anchor_id = spawn_p(&mut world, true, Vec3::new(0.0, 3.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();
    guarantee_model(&mut world, message, model_id, 3, 1, 2);
    body_check(&mut world, message, model_id, RigidBodyType::Fixed);

    world.despawn(anchor_id);

    sched_update.run(&mut world);

    guarantee_model(&mut world, message, model_id, 3, 0, 2);
    body_check(&mut world, message, model_id, RigidBodyType::Dynamic);
}

#[test]
pub fn model_into_parts() {
    let message = "Testing popping a model into parts";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];

    let children = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();

    guarantee_model(&mut world, message, model_id, 3, 0, 2);
    body_check(&mut world, message, model_id, RigidBodyType::Dynamic);

    // Now test deleting it
    world.entity_mut(model_id).remove_children(&children);
    world.despawn(model_id);

    sched_update.run(&mut world);

    let models = get_models(&mut world);
    assert_eq!(models.len(), 0, "{} - Models shouldn't exist", message);

    for child in children {
        guarantee(
            &mut world, message, child, false, false, false, false, true, true,
        );
        body_check(&mut world, message, child, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn model_into_models() {
    let message = "Testing cutting a model into submodels";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(0.0, 4.0, 0.0),
    ];
    let _ = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();

    let cut_id = spawn_p(&mut world, false, Vec3::new(0.0, 2.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();

    guarantee_model(&mut world, message, model_id, 5, 0, 4);
    body_check(&mut world, message, model_id, RigidBodyType::Dynamic);

    // Now test deleting it
    world.despawn(cut_id);

    sched_update.run(&mut world);

    let models = get_models(&mut world);
    assert_eq!(models.len(), 2, "{} - There aren't two models", message);

    for model_id in models {
        guarantee_model(&mut world, message, model_id, 2, 0, 1);
        body_check(&mut world, message, model_id, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn model_into_mixed() {
    let message = "Testing cutting a model into a model and part";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 3.0, 0.0),
    ];
    let _ = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();

    let cut_id = spawn_p(&mut world, false, Vec3::new(0.0, 2.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();

    guarantee_model(&mut world, message, model_id, 4, 0, 3);
    body_check(&mut world, message, model_id, RigidBodyType::Dynamic);

    // Now test deleting it
    world.despawn(cut_id);

    sched_update.run(&mut world);

    let models = get_models(&mut world);
    assert_eq!(models.len(), 1, "{} - There isn't one model", message);

    for model_id in models {
        guarantee_model(&mut world, message, model_id, 2, 0, 1);
        body_check(&mut world, message, model_id, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn model_into_models_with_anchor() {
    let message = "Testing cutting a model into submodels where one is anchored";
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    spawn_p(&mut world, true, Vec3::new(0.0, 5.0, 0.0));
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(0.0, 4.0, 0.0),
    ];
    let _ = positions
        .iter()
        .map(|position| spawn_p(&mut world, false, *position))
        .collect::<Vec<Entity>>();

    let cut_id = spawn_p(&mut world, false, Vec3::new(0.0, 2.0, 0.0));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let models = get_models(&mut world);

    assert_eq!(models.len(), 1, "{} - Model doesn't exist", message);
    let model_id = *models.first().unwrap();

    guarantee_model(&mut world, message, model_id, 5, 1, 4);
    body_check(&mut world, message, model_id, RigidBodyType::Fixed);

    // Now test deleting it
    world.despawn(cut_id);

    sched_update.run(&mut world);

    let models = get_models(&mut world);
    assert_eq!(models.len(), 2, "{} - There aren't two models", message);

    let mut model_query = world.query::<&Model>();
    for model_id in models {
        let model = model_query
            .get(&world, model_id)
            .expect("Couldn't get model");

        if model.anchors.is_empty() {
            guarantee_model(&mut world, message, model_id, 2, 0, 1);
            body_check(&mut world, message, model_id, RigidBodyType::Dynamic);
        } else {
            guarantee_model(&mut world, message, model_id, 2, 1, 1);
            body_check(&mut world, message, model_id, RigidBodyType::Fixed);
        }
    }
}

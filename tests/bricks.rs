use bevy_ecs::prelude::*;
use freebricks::{
    common::{model_graph::build_models, state::State},
    ecs::{
        common::{Position, Size},
        model::Model,
        parts::*,
        physics::{Anchor, Anchored, BodyHandle, Physical, PhysicsCleanup, ShapeHandle},
    },
    physics::PhysicsState,
};
use glam::Vec3;
use rapier3d::prelude::RigidBodyType;

/*
    Quick and easy tagging for system functions
*/

#[derive(Component)]
struct Tag1;

#[derive(Component)]
struct Tag2;

/*
    Helper functions
*/

fn util_setup() -> (World, Schedule, Schedule) {
    let mut world = World::new();
    let state = PhysicsState::new();
    PhysicsState::consume(&mut world, state);

    let mut init_schedule = Schedule::default();
    let mut update_schedule = Schedule::default();
    init_schedule.add_systems((build_models, PhysicsState::setup_system()).chain());

    update_schedule.add_systems((PhysicsState::update_system(false)).chain());

    return (world, init_schedule, update_schedule);
}

fn guarantee(
    world: &mut World,
    entity: Entity,
    anchored: bool,
    anchor: bool,
    child: bool,
    parent: bool,
    shape: bool,
    body: bool,
) {
    // Anchoring
    let mut query = world.query::<&Anchored>();
    assert_eq!(query.get(world, entity).is_ok(), anchored);

    let mut query = world.query::<&Anchor>();
    assert_eq!(query.get(world, entity).is_ok(), anchor);

    // Model
    let mut query = world.query::<&ChildOf>();
    assert_eq!(query.get(world, entity).is_ok(), child);

    let mut query = world.query::<&Children>();
    assert_eq!(query.get(world, entity).is_ok(), parent);

    // Physics
    let mut query = world.query::<&ShapeHandle>();
    assert_eq!(query.get(world, entity).is_ok(), shape);

    let mut query = world.query::<&BodyHandle>();
    assert_eq!(query.get(world, entity).is_ok(), body);
}

fn collider_check(world: &mut World, entity: Entity) {
    let handle = {
        let mut query = world.query::<&ShapeHandle>();
        query.get(world, entity).expect("Can't get shape query")
    };
    let _ = {
        let state = world
            .get_resource::<PhysicsState>()
            .expect("Couldn't get physics state");
        state
            .colliders
            .get(handle.0)
            .expect("Couldn't get collider")
    };
}

fn body_check(world: &mut World, entity: Entity, body_type: RigidBodyType) {
    let handle = {
        let mut query = world.query::<&BodyHandle>();
        query.get(world, entity).expect("Can't get body query")
    };
    let body = {
        let state = world
            .get_resource::<PhysicsState>()
            .expect("Couldn't get physics state");
        state
            .rigid_bodies
            .get(handle.0)
            .expect("Couldn't get rigid body")
    };
    assert_eq!(body.body_type(), body_type);
}

fn get_models(world: &mut World) -> Vec<(Entity, &Model)> {
    let mut query = world.query::<(Entity, &Model)>();
    query.iter(world).collect()
}

fn handle_deletion<T: Component>(
    mut commands: Commands,
    mut ew: EventWriter<PhysicsCleanup>,
    query: Query<(Entity, &T)>,
    bodies: Query<&BodyHandle>,
    shapes: Query<&ShapeHandle>,
    parent: Query<&ChildOf>,
) {
    let (e, _) = query.iter().next().expect("Couldn't get anchored brick");

    commands.entity(e).despawn();

    let pc = {
        PhysicsCleanup {
            entity: e,
            body: bodies.get(e).ok().map(|x| *x),
            shape: shapes.get(e).ok().map(|x| *x),
            parent: parent.get(e).ok().map(|x| x.0),
        }
    };
    ew.write(pc);
}

/*
    UNIT TESTS START
*/

#[test]
pub fn one_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = world.spawn((Part::default(), Physical)).id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, entity, false, false, false, false, true, true);
    collider_check(&mut world, entity);
    body_check(&mut world, entity, RigidBodyType::Dynamic);
}

#[test]
pub fn one_anchored_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = world.spawn((Part::default(), Physical, Anchor)).id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, entity, false, true, false, false, true, false);
    collider_check(&mut world, entity);
}

#[test]
pub fn two_separate_bricks() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
    ];
    let entities = world
        .spawn_batch(bricks.into_iter())
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(&mut world, entity, false, false, false, false, true, true);
        collider_check(&mut world, entity);
        body_check(&mut world, entity, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn two_snapped_bricks() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
    ];
    let entities = world
        .spawn_batch(bricks.into_iter())
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(&mut world, entity, false, false, true, false, true, false);
        collider_check(&mut world, entity);
    }
    let model = {
        let models = get_models(&mut world);

        assert_eq!(models.len(), 1);
        models.iter().next().expect("Couldn't find model").1
    };

    assert_eq!(model.graph.node_count(), 2);
    assert_eq!(model.anchored.len(), 0);
    assert_eq!(model.set.len(), 2);
}

#[test]
pub fn two_smooth_bricks_touching() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            StudInfo {
                top: StudType::Flat,
                bottom: StudType::Flat,
            },
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            StudInfo {
                top: StudType::Flat,
                bottom: StudType::Flat,
            },
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
    ];
    let entities = world
        .spawn_batch(bricks.into_iter())
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(&mut world, entity, false, false, false, false, true, true);
        collider_check(&mut world, entity);
        body_check(&mut world, entity, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn two_models() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(10.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(10.0, 1.0, 0.0)),
        ),
    ];
    let entities = world
        .spawn_batch(bricks.into_iter())
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    for entity in entities {
        guarantee(&mut world, entity, false, false, true, false, true, false);
        collider_check(&mut world, entity);
    }

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 2);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 2);
            e
        })
        .collect();
    assert_eq!(entities.len(), 2);

    for entity in entities {
        body_check(&mut world, entity, RigidBodyType::Dynamic);
    }
}

#[test]
pub fn brick_touching_anchor() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let anchor = world
        .spawn((
            Part::default(),
            Physical,
            Anchor,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ))
        .id();

    let anchored = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ))
        .id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, anchor, false, true, false, false, true, false);
    guarantee(&mut world, anchored, true, false, false, false, true, true);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 1);
    assert_eq!(
        state
            .anchor_sources
            .get(&anchor)
            .expect("Anchor not in anchor sources")
            .len(),
        1
    );
}

#[test]
pub fn model_touching_anchor() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
    ];
    world.spawn((
        Part::default(),
        Physical,
        Anchor,
        Position(Vec3::new(0.0, 2.0, 0.0)),
    ));
    let _ = world
        .spawn_batch(bricks.into_iter())
        .collect::<Vec<Entity>>();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 2);
            assert_eq!(m.anchored.len(), 1);
            assert_eq!(m.set.len(), 2);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);

    body_check(&mut world, *entities.first().unwrap(), RigidBodyType::Fixed);
}

#[test]
pub fn anchor_deletion_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    world.add_observer(
        |handle: Trigger<OnRemove, ShapeHandle>, state: ResMut<PhysicsState>| {
            println!("Sup, WHat's up");
        },
    );

    let anchor = world
        .spawn((
            Part::default(),
            Physical,
            Anchor,
            Position(Vec3::new(0.0, 0.0, 0.0)),
            Tag1,
        ))
        .id();

    let anchored = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ))
        .id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, anchor, false, true, false, false, true, false);
    guarantee(&mut world, anchored, true, false, false, false, true, true);
    body_check(&mut world, anchored, RigidBodyType::Fixed);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 1);
    assert_eq!(
        state
            .anchor_sources
            .get(&anchor)
            .expect("Anchor not in anchor sources")
            .len(),
        1
    );

    /*
       Delete anchored brick
    */
    let delete = world.register_system(handle_deletion::<Tag1>);
    world.run_system(delete).expect("Error handling deletion");
    sched_update.run(&mut world);
    // Simulate another physics step
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 0);
    guarantee(&mut world, anchored, false, false, false, false, true, true);
    body_check(&mut world, anchored, RigidBodyType::Dynamic);
}

#[test]
pub fn two_anchors_one_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let anchor_one = world
        .spawn((
            Part::default(),
            Position(Vec3::new(2.0, 10.0, 0.0)),
            Size(Vec3::new(4.0, 2.0, 4.0)),
            Physical,
            Anchor,
            Tag1,
        ))
        .id();
    let anchor_two = world
        .spawn((
            Part::default(),
            Position(Vec3::new(2.0, 10.0, 0.0)),
            Size(Vec3::new(4.0, 2.0, 4.0)),
            Physical,
            Anchor,
            Tag2,
        ))
        .id();
    let anchored = world
        .spawn((
            Part::default(),
            Position(Vec3::new(0.0, 8.0, 0.0)),
            Size(Vec3::new(5.0, 2.0, 5.0)),
            Physical,
        ))
        .id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(
        &mut world, anchor_one, false, true, false, false, true, false,
    );
    guarantee(
        &mut world, anchor_two, false, true, false, false, true, false,
    );
    collider_check(&mut world, anchor_one);
    collider_check(&mut world, anchor_two);

    guarantee(&mut world, anchored, true, false, false, false, true, true);
    body_check(&mut world, anchored, RigidBodyType::Fixed);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 2);
    assert_eq!(
        state
            .anchor_sources
            .get(&anchor_one)
            .expect("Anchor not in anchor sources")
            .len(),
        1
    );
    assert_eq!(
        state
            .anchor_sources
            .get(&anchor_two)
            .expect("Anchor not in anchor sources")
            .len(),
        1
    );

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    let delete_two = world.register_system(handle_deletion::<Tag2>);

    // Delete one anchor
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 1);
    guarantee(&mut world, anchored, true, false, false, false, true, true);
    body_check(&mut world, anchored, RigidBodyType::Fixed);

    // Delete other anchor
    world
        .run_system(delete_two)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(state.anchor_sources.len(), 0);
    guarantee(&mut world, anchored, false, false, false, false, true, true);
    body_check(&mut world, anchored, RigidBodyType::Dynamic);
}

#[test]
pub fn anchor_deletion_model() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
    ];
    let _ = world.spawn_batch(bricks.into_iter());

    world.spawn((
        Part::default(),
        Physical,
        Anchor,
        Position(Vec3::new(0.0, 2.0, 0.0)),
        Tag1,
    ));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 2);
            assert_eq!(m.anchored.len(), 1);
            assert_eq!(m.set.len(), 2);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);

    let model_entity = *entities.first().unwrap();
    body_check(&mut world, model_entity, RigidBodyType::Fixed);

    let delete_one = world.register_system(handle_deletion::<Tag1>);

    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);
    body_check(&mut world, model_entity, RigidBodyType::Dynamic);

    world
        .query::<&Anchored>()
        .get(&world, model_entity)
        .expect_err("Model shouldn't have anchored_to component");
}

#[test]
pub fn model_leaf_deletion() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 0.0, 0.0)),
        Tag1,
    ));
    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 1.0, 0.0)),
    ));
    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 2.0, 0.0)),
    ));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 3);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 3);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 2);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 2);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);
}

#[test]
pub fn model_cut_deletion_5() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let _brick_zero = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, -1.0, 0.0)),
        ))
        .id();
    let _brick_one = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ))
        .id();
    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 1.0, 0.0)),
        Tag1,
    ));
    let _brick_two = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 2.0, 0.0)),
        ))
        .id();
    let _brick_three = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 3.0, 0.0)),
        ))
        .id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 5);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 5);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world).iter().map(|&(e, _)| e).collect();
    assert_eq!(entities.len(), 2);
}

#[test]
pub fn model_cut_deletion_3() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let brick_one = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ))
        .id();
    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 1.0, 0.0)),
        Tag1,
    ));
    let brick_two = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 2.0, 0.0)),
        ))
        .id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 3);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 3);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    guarantee(
        &mut world, brick_one, false, false, false, false, true, true,
    );
    guarantee(
        &mut world, brick_two, false, false, false, false, true, true,
    );
    assert!(world.query::<&Model>().iter(&world).next().is_none());
    body_check(&mut world, brick_one, RigidBodyType::Dynamic);
    body_check(&mut world, brick_two, RigidBodyType::Dynamic);
}

#[test]
pub fn model_leaf_anchored_deletion() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ),
        (
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 1.0, 0.0)),
        ),
    ];
    let _ = world.spawn((
        Part::default(),
        Physical,
        Anchor,
        Position(Vec3::new(0.0, 3.0, 0.0)),
    ));
    let _ = world.spawn_batch(bricks);
    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 2.0, 0.0)),
        Tag1,
    ));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 3);
            assert_eq!(m.anchored.len(), 1);
            assert_eq!(m.set.len(), 3);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);
    body_check(&mut world, *entities.first().unwrap(), RigidBodyType::Fixed);

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 2);
            assert_eq!(m.anchored.len(), 0);
            assert_eq!(m.set.len(), 2);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);
    body_check(
        &mut world,
        *entities.first().unwrap(),
        RigidBodyType::Dynamic,
    );
}

#[test]
pub fn model_cut_anchored_deletion() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let brick_one = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ))
        .id();

    let brick_two = world
        .spawn((
            Part::default(),
            Physical,
            Position(Vec3::new(0.0, 2.0, 0.0)),
        ))
        .id();

    let _ = world
        .spawn((
            Part::default(),
            Physical,
            Anchor,
            Position(Vec3::new(0.0, 3.0, 0.0)),
        ))
        .id();

    let _ = world.spawn((
        Part::default(),
        Physical,
        Position(Vec3::new(0.0, 1.0, 0.0)),
        Tag1,
    ));

    sched_start.run(&mut world);
    sched_update.run(&mut world);

    let entities: Vec<Entity> = get_models(&mut world)
        .iter()
        .map(|&(e, m)| {
            assert_eq!(m.graph.node_count(), 3);
            assert_eq!(m.anchored.len(), 1);
            assert_eq!(m.set.len(), 3);
            e
        })
        .collect();

    assert_eq!(entities.len(), 1);
    body_check(&mut world, *entities.first().unwrap(), RigidBodyType::Fixed);

    let delete_one = world.register_system(handle_deletion::<Tag1>);
    world
        .run_system(delete_one)
        .expect("Error handling deletion");
    sched_update.run(&mut world);

    guarantee(
        &mut world, brick_one, false, false, false, false, true, true,
    );
    guarantee(&mut world, brick_two, true, false, false, false, true, true);
    assert!(world.query::<&Model>().iter(&world).next().is_none());
    body_check(&mut world, brick_one, RigidBodyType::Dynamic);
    body_check(&mut world, brick_two, RigidBodyType::Fixed);
}

use bevy_ecs::prelude::*;
use freebricks::{common::{model_builder::build_models, state::State}, components::{bricks::*, common::Position, model::Model, physics::{AnchorSource, AnchoredTo, BodyHandle, Physical, ShapeHandle}}, physics::PhysicsState};
use glam::Vec3;
use rapier3d::prelude::RigidBodyType;


fn util_setup() -> (World, Schedule, Schedule){
    let mut world = World::new();
    let state = PhysicsState::new();
    PhysicsState::consume(&mut world, state);


    let mut init_schedule = Schedule::default();
    let mut update_schedule = Schedule::default();
    init_schedule.add_systems((
        build_models,
        PhysicsState::setup_solo_bricks,
        PhysicsState::setup_model_bricks        
    ).chain());

    update_schedule.add_systems((
        PhysicsState::step, 
        PhysicsState::add_bricks, 
        PhysicsState::handle_deletion, 
        PhysicsState::update_bricks
    ).chain());

    return (world, init_schedule, update_schedule);
}

fn guarantee(
        world: &mut World,
        entity: Entity, 
        anchored: bool, anchor: bool, 
        child: bool, parent: bool, 
        shape: bool, body: bool) {
            
    // Anchoring 
    let mut query = world.query::<&AnchoredTo>();
    assert_eq!(query.get(world, entity).is_ok(), anchored);

    let mut query = world.query::<&AnchorSource>();
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
        let state = world.get_resource::<PhysicsState>()
            .expect("Couldn't get physics state");
        state.colliders.get(handle.0)
            .expect("Couldn't get collider")
    }; 
}

fn body_check(world: &mut World, entity: Entity, body_type: RigidBodyType) {
    let handle = {
        let mut query = world.query::<&BodyHandle>();
        query.get(world, entity).expect("Can't get body query")
    };
    let body = {
        let state = world.get_resource::<PhysicsState>() 
            .expect("Couldn't get physics state");
        state.rigid_bodies.get(handle.0)
            .expect("Couldn't get rigid body")
    };
    assert_eq!(body.body_type(), body_type);
}

fn get_models(world: &mut World) -> Vec<&Model> {
    let mut query = world.query::<&Model>();
    query.iter(world).collect()
}

/*
    UNIT TESTS START
*/

#[test]
pub fn one_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = world.spawn((
        Brick::default(),
        Physical::dynamic()
    )).id();
    
    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, entity, false, false, false, false, true, true);
    collider_check(&mut world, entity);
    body_check(&mut world, entity, RigidBodyType::Dynamic);
}

#[test]
pub fn one_anchored_brick() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let entity = world.spawn((
        Brick::default(),
        Physical::anchored()
    )).id();
    
    sched_start.run(&mut world);
    sched_update.run(&mut world);

    guarantee(&mut world, entity, false, false, false, false, true, false);
    collider_check(&mut world, entity);
}

#[test]
pub fn two_separate_bricks() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let bricks = vec![
        (
            Brick::default(),
            Physical::dynamic(),
            Position(Vec3::new(0.0, 0.0, 0.0))
        ), 
        (
            Brick::default(),
            Physical::dynamic(),
            Position(Vec3::new(0.0, 0.0, 0.0))
        )
    ];
    let entities = world.spawn_batch(bricks.into_iter())
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
            Brick::default(),
            Physical::dynamic(),
            Position(Vec3::new(0.0, 0.0, 0.0))
        ), 
        (
            Brick::default(),
            Physical::dynamic(),
            Position(Vec3::new(0.0, 1.0, 0.0))
        )
    ];
    let entities = world.spawn_batch(bricks.into_iter())
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
        *models.iter().next().expect("Couldn't find model")
    };
    
    assert_eq!(model.graph.node_count(), 2);
    assert_eq!(model.anchored.len(), 0);
    assert_eq!(model.set.len(), 2);
}

#[test]
pub fn touching_anchor() {
    let (mut world, mut sched_start, mut sched_update) = util_setup();

    let anchor = world.spawn((
        Brick::default(),
        Physical::anchored(),
        Position(Vec3::new(0.0, 0.0, 0.0))
    )).id();


    let anchored = world.spawn((
        Brick::default(),
        Physical::dynamic(),
        Position(Vec3::new(0.0, 1.0, 0.0))
    )).id();

    sched_start.run(&mut world);
    sched_update.run(&mut world);
        
    guarantee(&mut world, anchor, false, true, false, false, true, false);
    guarantee(&mut world, anchored, true, false, false, false, true, true);
}
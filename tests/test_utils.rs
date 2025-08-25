use bevy_ecs::prelude::*;
use freebricks::{
    common::{
        model_graph::{build_models, handle_model_transform, handle_part_of_model_deletion},
        state::State,
    },
    ecs::{
        common::{Position, Size},
        model::Model,
        parts::Part,
        physics::{Anchor, Anchored, BodyHandle, Physical, ShapeHandle},
    },
    physics::PhysicsState,
};
use glam::Vec3;
use rapier3d::prelude::RigidBodyType;

#[derive(Component)]
struct Tag1;

#[derive(Component)]
struct Tag2;

#[derive(Component)]
struct Tag3;

#[allow(dead_code)]
pub fn util_setup() -> (World, Schedule, Schedule) {
    let mut world = World::new();
    let state = PhysicsState::new();
    PhysicsState::consume(&mut world, state);

    let mut init_schedule = Schedule::default();
    let mut update_schedule = Schedule::default();
    init_schedule.add_systems((build_models, PhysicsState::setup_system()).chain());

    update_schedule
        .add_systems((handle_model_transform, PhysicsState::update_system(false)).chain());
    world.add_observer(handle_part_of_model_deletion);
    return (world, init_schedule, update_schedule);
}

#[allow(dead_code)]
pub fn guarantee(
    world: &mut World,
    message: &str,
    entity: Entity,
    anchored: bool,
    anchor: bool,
    child: bool,
    parent: bool,
    shape: bool,
    body: bool,
) {
    // anchoring
    let mut query = world.query::<&Anchored>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        anchored,
        "{} - is entity anchored?",
        message
    );

    let mut query = world.query::<&Anchor>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        anchor,
        "{} - Is entity an anchor?",
        message
    );

    // Model
    let mut query = world.query::<&ChildOf>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        child,
        "{} - is entity a child?",
        message
    );

    let mut query = world.query::<&Children>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        parent,
        "{} - is entity a parent?",
        message
    );

    // Physics
    let mut query = world.query::<&ShapeHandle>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        shape,
        "{} - does entity have a collider?",
        message
    );

    let mut query = world.query::<&BodyHandle>();
    assert_eq!(
        query.get(world, entity).is_ok(),
        body,
        "{} - does entity have a rigid body?",
        message
    );
}

#[allow(dead_code)]
pub fn collider_check(world: &mut World, message: &str, entity: Entity) {
    let handle = {
        let mut query = world.query::<&ShapeHandle>();
        query
            .get(world, entity)
            .expect(&format!("{} - Can't get shape query", message))
    };
    let _ = {
        let state = world
            .get_resource::<PhysicsState>()
            .expect(&format!("{} - Couldn't get physics state", message));
        state
            .colliders
            .get(handle.0)
            .expect(&format!("{} - Couldn't get collider", message))
    };
}

#[allow(dead_code)]
pub fn body_check(world: &mut World, message: &str, entity: Entity, body_type: RigidBodyType) {
    let handle = {
        let mut query = world.query::<&BodyHandle>();
        query
            .get(world, entity)
            .expect(&format!("{} - Can't get body query", message))
    };
    let body = {
        let state = world
            .get_resource::<PhysicsState>()
            .expect(&format!("{} - Couldn't get physics state", message));
        state
            .rigid_bodies
            .get(handle.0)
            .expect(&format!("{} - Couldn't get rigid body", message))
    };
    assert_eq!(
        body.body_type(),
        body_type,
        "{} - Checking if rigid body type of {:?} is {:?}",
        message,
        body.body_type(),
        body_type
    );
}

#[allow(dead_code)]
pub fn guarantee_model(
    world: &mut World,
    message: &str,
    entity: Entity,
    num_children: usize,
    num_anchors: usize,
    num_edges: usize,
) {
    let actual_num_children = {
        world
            .query::<&Children>()
            .get(world, entity)
            .expect(&format!("{} - Couldn't find children", message))
            .iter()
            .len()
    };

    let model = world
        .query::<&Model>()
        .get(world, entity)
        .expect(&format!("{} - Couldn't find model", message));

    assert_eq!(
        model.graph.node_count(),
        actual_num_children,
        "{} - Graph nodes don't match Children size",
        message
    );
    assert_eq!(
        num_children, actual_num_children,
        "{} - Children size doesn't match children specified by argument",
        message
    );
    assert_eq!(
        model.graph.edge_count(),
        num_edges,
        "{} - Edge counts don't match",
        message
    );
    assert_eq!(
        model.anchors.len(),
        num_anchors,
        "{} - Anchor counts don't match",
        message
    );
}

#[allow(dead_code)]
pub fn get_models(world: &mut World) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Model)>();
    query.iter(world).map(|(e, _)| e).collect()
}

#[allow(dead_code)]
pub fn spawn_p(world: &mut World, anchor: bool, position: Vec3) -> Entity {
    if anchor {
        world
            .spawn((Part::default(), Anchor, Position(position)))
            .id()
    } else {
        world
            .spawn((Part::default(), Physical, Position(position)))
            .id()
    }
}

#[allow(dead_code)]
pub fn spawn_ps(world: &mut World, anchor: bool, position: Vec3, size: Vec3) -> Entity {
    if anchor {
        world
            .spawn((Part::default(), Anchor, Position(position), Size(size)))
            .id()
    } else {
        world
            .spawn((Part::default(), Physical, Position(position), Size(size)))
            .id()
    }
}

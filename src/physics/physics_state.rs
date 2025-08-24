use bevy_platform::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::ops::DerefMut;

use crate::ecs::common::{Position, Rotation};
use crate::ecs::model::{Model, QModel};
use crate::{
    common::state::*,
    ecs::{parts::*, physics::*},
    render::debug_draw::*,
};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;
use glam::{Vec3, quat};
use petgraph::prelude::UnGraphMap;
use petgraph::visit::{Bfs, NodeIndexable};
use rapier3d::na::Isometry;
use rapier3d::pipeline::DebugRenderPipeline;
use rapier3d::prelude::*;

use super::{deletion::*, setup::*};

#[derive(Resource)]
pub struct PhysicsState {
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,

    parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    debug_render: DebugRenderPipeline,

    pub anchor_sources: HashMap<Entity, HashSet<Entity>>,
    pub collider_sources: HashMap<ColliderHandle, Entity>,
}

impl State<PhysicsState> for PhysicsState {
    fn consume(world: &mut World, state: PhysicsState) {
        world.insert_resource(state);
        world.insert_resource(Events::<PhysicsCleanup>::default());
        world.add_observer(handle_body_removal);
    }
}

impl PhysicsState {
    /// Initialize physics scene. Doesn't require anything before it.
    pub fn new() -> Self {
        let rigid_bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();

        let parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();

        let debug_render =
            DebugRenderPipeline::new(DebugRenderStyle::default(), DebugRenderMode::default());
        PhysicsState {
            rigid_bodies: rigid_bodies,
            colliders: colliders,
            parameters: parameters,
            physics_pipeline: physics_pipeline,
            island_manager: island_manager,
            broad_phase: broad_phase,
            narrow_phase: narrow_phase,
            impulse_joint_set: impulse_joint_set,
            multibody_joint_set: multibody_joint_set,
            ccd_solver: ccd_solver,
            query_pipeline: query_pipeline,
            debug_render: debug_render,

            anchor_sources: HashMap::new(),
            collider_sources: HashMap::new(),
        }
    }

    /// Add schedulers
    pub fn setup_system() -> ScheduleConfigs<ScheduleSystem> {
        (setup_parts, setup_models).chain()
    }

    pub fn update_system(debug_draw: bool) -> ScheduleConfigs<ScheduleSystem> {
        (
            Self::step,
            Self::write_debug.run_if(move || -> bool { debug_draw }),
            Self::add_bricks,
            Self::handle_deletion,
            Self::handle_deletion_two,
            Self::handle_pop,
            Self::handle_model_mutations,
            Self::handle_new_collider,
            Self::handle_deleted_collider,
            Self::update_bricks,
        )
            .chain()
    }

    /// Update physics state
    pub fn step(mut state: ResMut<PhysicsState>) {
        let state = state.deref_mut();

        let physics_hooks = ();
        let event_handler = ();

        state.physics_pipeline.step(
            &Vector::<Real>::new(0.0, -9.8, 0.0),
            &state.parameters,
            &mut state.island_manager,
            &mut state.broad_phase,
            &mut state.narrow_phase,
            &mut state.rigid_bodies,
            &mut state.colliders,
            &mut state.impulse_joint_set,
            &mut state.multibody_joint_set,
            &mut state.ccd_solver,
            Some(&mut state.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    }

    /*
       BRICK AND MODEL RELATED FUNCTIONS
    */

    /// Handle solitary bricks
    ///
    /*
    pub fn setup_solo_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        anchored_bricks: Query<QPhysics, (Without<ChildOf>, With<AnchoredTo>)>,
        bricks: Query<QPhysics, (Without<ChildOf>, Without<AnchoredTo>)>,
    ) {
        let state = state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;

        let mut build_brick = |brick: QPhysicsItem,
                               builder: RigidBodyBuilder,
                               commands: &mut Commands,
                               colliders: &mut ColliderSet| {
            let size = brick.size.0 / 2.0;
            let pos = brick.position;
            let (yaw, pitch, roll) = {
                let (axis, angle) = brick.rotation.to_axis_angle();
                (axis.x * angle, axis.y * angle, axis.z * angle)
            };

            let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                .restitution(0.4)
                .build();

            let body = builder
                .translation(vector![pos.x, pos.y, pos.z])
                .rotation(vector![yaw, pitch, roll])
                .user_data(brick.entity.to_bits() as u128)
                .build();

            let body_handle = rigid_bodies.insert(body);
            let shape_handle = colliders.insert_with_parent(shape, body_handle, rigid_bodies);

            commands
                .entity(brick.entity)
                .insert(BodyHandle(body_handle))
                .insert(ShapeHandle(shape_handle));
        };

        for brick in anchored_bricks {
            build_brick(brick, RigidBodyBuilder::fixed(), &mut commands, colliders);
        }
        // Some might be anchored, some might not be
        for brick in bricks {
            if brick.physical.anchored {
                // rapier3d supports collider-onlies
                let size = brick.size.0 / 2.0;
                let pos = brick.position;
                let (yaw, pitch, roll) = {
                    let (axis, angle) = brick.rotation.to_axis_angle();
                    (axis.x * angle, axis.y * angle, axis.z * angle)
                };

                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                    .translation(vector![pos.x, pos.y, pos.z])
                    .rotation(vector![yaw, pitch, roll])
                    .restitution(0.4)
                    .build();

                let shape_handle = colliders.insert(shape);
                commands
                    .entity(brick.entity)
                    .insert(ShapeHandle(shape_handle));
            } else {
                build_brick(brick, RigidBodyBuilder::dynamic(), &mut commands, colliders);
            }
        }
    }

    pub fn setup_model_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        models: Query<QModel>,
        bricks: Query<QPhysics>,
    ) {
        let state = state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;

        for item in models {
            let mut shapes = Vec::new();

            for &child in item.children {
                let Ok(child_item) = bricks.get(child) else {
                    continue;
                };

                let size = child_item.size.0 / 2.0;
                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                    .translation(vector![
                        child_item.position.x,
                        child_item.position.y,
                        child_item.position.z
                    ])
                    .restitution(0.4)
                    .build();
                shapes.push((child, shape));
            }

            let body = {
                if item.model.anchored.is_empty() {
                    RigidBodyBuilder::dynamic()
                } else {
                    RigidBodyBuilder::fixed()
                }
            }
            .user_data(item.entity.to_bits() as u128);
            let body_handle = rigid_bodies.insert(body);
            for (child, shape) in shapes {
                let handle = colliders.insert_with_parent(shape, body_handle, rigid_bodies);
                commands.entity(child).insert(ShapeHandle(handle));
            }
            commands.entity(item.entity).insert(BodyHandle(body_handle));
        }
    }
    */
    pub fn handle_new_collider(
        mut state: ResMut<PhysicsState>,
        added: Query<(Entity, &ShapeHandle), Added<ShapeHandle>>,
    ) {
        for (e, handle) in added {
            state.collider_sources.insert(handle.0, e);
        }
    }

    pub fn handle_deleted_collider(
        mut state: ResMut<PhysicsState>,
        mut removed: RemovedComponents<ShapeHandle>,
    ) {
        if removed.is_empty() {
            return;
        }

        let entities: HashSet<Entity> = {
            let x = removed.read();
            HashSet::from_iter(x)
        };

        state.collider_sources.retain(|_, v| entities.contains(v));
        println!("{:?}", state.collider_sources);
    }

    pub fn handle_deletion_two(
        mut state: ResMut<PhysicsState>,
        mut deleted: EventReader<PhysicsCleanup>,
        mut models: Query<(&mut Model, &BodyHandle)>,
    ) {
        let deleted: Vec<&PhysicsCleanup> = deleted
            .read()
            .filter(|&item| item.parent.is_some())
            .collect();

        if deleted.is_empty() {
            return;
        }

        let state = state.deref_mut();
        let rigid_bodies = &mut state.rigid_bodies;
        let islands = &mut state.island_manager;
        let colliders = &mut state.colliders;
        let _impulse_joints = &mut state.impulse_joint_set;
        let _multibody_joints = &mut state.multibody_joint_set;

        for deletion in deleted {
            let model_id = deletion.parent.unwrap();
            let shape_handle = deletion.shape.unwrap();
            let (mut model, _) = models
                .get_mut(model_id)
                .expect("Couldn't get model and handle");

            colliders.remove(shape_handle.0, islands, rigid_bodies, false);

            model.anchored.remove(&deletion.entity);
            model.graph.remove_node(deletion.entity);
            model.set.remove(&deletion.entity);
        }
    }

    pub fn handle_deletion(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        mut anchored_solo_bricks: Query<(&BodyHandle, &mut Anchored), Without<ChildOf>>,
        mut anchored_model_bricks: Query<(&mut Anchored, &ChildOf)>,

        mut models: Query<(&mut Model, &BodyHandle)>,
        mut deleted: EventReader<PhysicsCleanup>,
    ) {
        if deleted.is_empty() {
            return;
        }
        let state = state.deref_mut();
        let rigid_bodies = &mut state.rigid_bodies;
        let islands = &mut state.island_manager;
        let colliders = &mut state.colliders;
        let _impulse_joints = &mut state.impulse_joint_set;
        let _multibody_joints = &mut state.multibody_joint_set;

        let anchor_sources = &mut state.anchor_sources;

        for cleanup in deleted.read() {
            if !anchor_sources.contains_key(&cleanup.entity) {
                continue;
            }

            let connected = anchor_sources.remove(&cleanup.entity).unwrap();

            // Go through bricks connected to this one
            for anchored in connected {
                // Solo Bricks
                if let Ok((handle, mut set)) = anchored_solo_bricks.get_mut(anchored) {
                    set.0.remove(&cleanup.entity);

                    if set.0.len() > 0 {
                        continue;
                    }
                    let Some(body) = rigid_bodies.get_mut(handle.0) else {
                        continue;
                    };

                    body.set_body_type(RigidBodyType::Dynamic, true);
                    commands.entity(anchored).remove::<Anchored>();

                // Model Bricks
                } else if let Ok((mut set, parent)) = anchored_model_bricks.get_mut(anchored) {
                    set.0.remove(&cleanup.entity);

                    if set.0.len() > 0 {
                        continue;
                    }

                    let (mut model, handle) = models.get_mut(parent.0).expect("???");

                    model.anchored.remove(&anchored);
                    commands.entity(anchored).remove::<Anchored>();

                    if !model.anchored.is_empty() {
                        continue;
                    }

                    let Some(body) = rigid_bodies.get_mut(handle.0) else {
                        continue;
                    };

                    body.set_body_type(RigidBodyType::Dynamic, true);
                }
            }

            let handle = cleanup
                .shape
                .expect("Removing anchored brick, expected collider handle");
            colliders.remove(handle.0, islands, rigid_bodies, false);
        }
    }

    pub fn handle_pop(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        mut popped_bricks: RemovedComponents<ChildOf>,
        mut replaced: Query<&ChildOf>,
        mut shapes: Query<&mut ShapeHandle>,
    ) {
        if popped_bricks.is_empty() {
            return;
        }

        let state = state.deref_mut();

        // Only choose bricks that don't despawn (RemovedComponents is called on despawn)
        let popped_bricks: Vec<Entity> = popped_bricks
            .read()
            .into_iter()
            .filter_map(|x| {
                // If entity exists
                commands.get_entity(x).ok().map(|_| x)
            })
            .filter(|&x| {
                // If entity has a new parent
                !replaced.get(x).is_ok()
            })
            .collect();

        for popped in popped_bricks {
            let shape_handle = shapes.get_mut(popped).unwrap().0;

            let shape = state.colliders.get_mut(shape_handle).unwrap();

            let pos = shape.translation().to_owned();
            let (yaw, pitch, roll) = shape.rotation().euler_angles();

            let new_body = RigidBodyBuilder::dynamic()
                .user_data(popped.to_bits() as u128)
                .translation(pos)
                .rotation(vector![yaw, pitch, roll])
                .build();
            let new_handle = state.rigid_bodies.insert(new_body);

            state
                .colliders
                .set_parent(shape_handle, Some(new_handle), &mut state.rigid_bodies);

            let shape = state.colliders.get_mut(shape_handle).unwrap();
            shape.set_position_wrt_parent(Isometry::identity());

            // Popping removes it from anchors
            commands.entity(popped).insert(BodyHandle(new_handle));
            commands.entity(popped).try_remove::<Anchored>();
            for (_, anchored) in &mut state.anchor_sources {
                anchored.remove(&popped);
            }
        }
    }

    pub fn handle_model_mutations(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        mut shapes: Query<&mut ShapeHandle>,
        bodies: Query<&BodyHandle>,
        changed_models: Query<(Entity, &Model, &BodyHandle), Changed<Model>>,
    ) {
        if changed_models.is_empty() {
            return;
        }

        let state = state.deref_mut();
        let rigid_bodies = &mut state.rigid_bodies;
        let islands = &mut state.island_manager;
        let colliders = &mut state.colliders;
        let impulse_joints = &mut state.impulse_joint_set;
        let multibody_joints = &mut state.multibody_joint_set;

        for (model_id, model, body_handle) in changed_models {
            // Do BFS to see if it's fully connected or notA
            let mut count = 0;

            if model.graph.node_count() != 0 {
                let mut bfs = Bfs::new(&model.graph, model.graph.from_index(0));
                while let Some(_) = bfs.next(&model.graph) {
                    count += 1;
                }
            }
            // If no subgraphs, continue
            if count == model.graph.node_count() {
                let body_handle = bodies.get(model_id).unwrap().0;
                let body = rigid_bodies.get_mut(body_handle).unwrap();

                // If only anchor was removed (otherwise wouldn't be fixed), unanchor model.
                if model.anchored.is_empty() && body.is_fixed() {
                    body.set_body_type(RigidBodyType::Dynamic, true);
                }
                // If no subgraphs, continue
                continue;
            }
            // Otherwise, we got to disassemble

            let mut collections = Vec::new();
            let mut dirty = HashSet::new();
            for node in model.graph.nodes() {
                if dirty.contains(&node) {
                    continue;
                }
                let mut subgraph: UnGraphMap<Entity, ()> = UnGraphMap::new();
                let mut subset = HashSet::new();
                let mut queue = VecDeque::from([node]);

                subgraph.add_node(node);
                let mut i = 0;
                while let Some(node) = queue.pop_front() {
                    if dirty.contains(&node) {
                        continue;
                    }

                    i += 1;

                    subset.insert(node);
                    dirty.insert(node);

                    for (_, other, ()) in model.graph.edges(node) {
                        subgraph.add_node(other);
                        subgraph.add_edge(node, other, ());
                        queue.push_back(other);
                    }
                }
                let subanchored: HashSet<Entity> = {
                    let x: Vec<Entity> = subset
                        .iter()
                        .cloned()
                        .filter(|&x| model.anchored.contains(&x))
                        .collect();

                    HashSet::from_iter(x.into_iter())
                };

                collections.push((subgraph, subset, subanchored));
            }

            // Pop old model of children since we're replacing them.
            commands
                .entity(model_id)
                .remove_children(&Vec::from_iter(model.set.clone()));

            // TODO, get linvel, angvel
            let old_body = rigid_bodies
                .remove(
                    body_handle.0,
                    islands,
                    colliders,
                    impulse_joints,
                    multibody_joints,
                    false,
                )
                .unwrap();

            let linvel = old_body.linvel().to_owned();
            let angvel = old_body.angvel().to_owned();

            for (subgraph, subset, subanchored) in collections {
                let anchored = !subanchored.is_empty();
                let builder = {
                    if anchored {
                        RigidBodyBuilder::fixed()
                    } else {
                        RigidBodyBuilder::dynamic()
                    }
                };
                if subset.len() == 1 {
                    let entity = subset.iter().next().unwrap();

                    let shape_handle = shapes.get(*entity).unwrap().0;

                    let (pos, (yaw, pitch, roll)) = {
                        let shape = colliders.get(shape_handle).unwrap();
                        (
                            shape.translation().to_owned(),
                            shape.rotation().euler_angles(),
                        )
                    };

                    let new_body = builder
                        .user_data(entity.to_bits() as u128)
                        .translation(pos)
                        .rotation(vector![yaw, pitch, roll])
                        .linvel(linvel)
                        .angvel(angvel)
                        .build();
                    let new_handle = rigid_bodies.insert(new_body);

                    colliders.set_parent(shape_handle, Some(new_handle), rigid_bodies);

                    let shape = colliders.get_mut(shape_handle).unwrap();
                    shape.set_position_wrt_parent(Isometry::identity());

                    commands.entity(*entity).insert(BodyHandle(new_handle));
                } else {
                    let new_model_id = commands
                        .spawn_empty()
                        .insert(Model {
                            set: subset.clone(),
                            graph: subgraph,
                            anchored: subanchored,
                        })
                        .id();

                    let new_body = builder
                        .user_data(new_model_id.to_bits() as u128)
                        .linvel(linvel)
                        .angvel(angvel)
                        .build();
                    let new_handle = rigid_bodies.insert(new_body);

                    for child in &subset {
                        let mut shape_handle = shapes.get(*child).unwrap().0;

                        // Do this instead of set parent
                        // Why? Doesn't account for collider positions as well
                        let shape = colliders
                            .remove(shape_handle, islands, rigid_bodies, false)
                            .unwrap();
                        shape_handle =
                            colliders.insert_with_parent(shape, new_handle, rigid_bodies);

                        *shapes.get_mut(*child).unwrap() = ShapeHandle(shape_handle);
                    }

                    commands
                        .entity(new_model_id)
                        .insert((BodyHandle(new_handle),))
                        .add_children(&Vec::from_iter(subset));
                }
            }

            commands.entity(model_id).despawn();
        }
    }

    // Called when bricks are added into the scene
    // Note: Bricks covered under setup_* are not handled under this
    pub fn add_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        new_bricks: Query<QPartWorldInit, (FPartAdd, Without<ShapeHandle>, Without<BodyHandle>)>,
        is_anchor: Query<&Anchor>,
    ) {
        let state = state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;

        for brick in new_bricks {
            let size = brick.size.0 / 2.0;
            let pos = brick.position;
            let (yaw, pitch, roll) = {
                let (axis, angle) = brick.rotation.to_axis_angle();
                (axis.x * angle, axis.y * angle, axis.z * angle)
            };

            let shape_builder = ColliderBuilder::cuboid(size.x, size.y, size.z).restitution(0.4);

            if is_anchor.get(brick.entity).is_ok() {
                let shape = shape_builder
                    .translation(vector![pos.x, pos.y, pos.z])
                    .rotation(vector![yaw, pitch, roll])
                    .build();

                let handle = colliders.insert(shape);
                commands.entity(brick.entity).insert(ShapeHandle(handle));
            } else {
                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                    .restitution(0.4)
                    .build();

                let body = RigidBodyBuilder::dynamic()
                    .translation(vector![pos.x, pos.y, pos.z])
                    .rotation(vector![yaw, pitch, roll])
                    .user_data(brick.entity.to_bits() as u128)
                    .build();

                let body_handle = rigid_bodies.insert(body);
                let shape_handle = colliders.insert_with_parent(shape, body_handle, rigid_bodies);

                commands
                    .entity(brick.entity)
                    .insert(BodyHandle(body_handle))
                    .insert(ShapeHandle(shape_handle));
            }
        }
    }

    /// Push updated physics state onto the relevant components
    /// Works best after PhysicsState::step
    pub fn update_bricks(
        state: Res<PhysicsState>,
        models: Query<QModel>,
        mut solo_bricks: Query<QPartWorldUpdate, Without<ChildOf>>,
        mut model_bricks: Query<(&mut Position, &mut Rotation, &ShapeHandle), With<ChildOf>>,
    ) {
        let rigid_bodies = &state.rigid_bodies;
        let colliders = &state.colliders;

        for (_handle, body) in rigid_bodies.iter() {
            if body.is_sleeping() {
                continue;
            }

            let e = Entity::from_bits(body.user_data as u64);

            if let Ok(mut brick) = solo_bricks.get_mut(e) {
                let pos = body.position().translation;
                let rot = body.position().rotation;

                brick.position.0 = Vec3::new(pos.x, pos.y, pos.z);
                brick.rotation.0 = quat(rot.i, rot.j, rot.k, rot.w);
            } else if let Ok(model) = models.get(e) {
                for &child in model.children {
                    if let Ok((mut p, mut r, h)) = model_bricks.get_mut(child) {
                        let collider = colliders.get(h.0).unwrap();

                        let pos = collider.translation();
                        let rot = colliders.get(h.0).unwrap().rotation();

                        p.0 = Vec3::new(pos.x, pos.y, pos.z);
                        r.0 = quat(rot.i, rot.j, rot.k, rot.w);
                    }
                }
            } else {
                // This shouldn't happen
            }
        }
    }

    /// Write lines for debug drawing
    pub fn write_debug(mut state: ResMut<PhysicsState>, mut debug_draw: ResMut<DebugDraw>) {
        let state = state.deref_mut();

        state.debug_render.render(
            debug_draw.as_mut(),
            &state.rigid_bodies,
            &state.colliders,
            &state.impulse_joint_set,
            &state.multibody_joint_set,
            &state.narrow_phase,
        );
    }
}

use std::collections::{HashMap, HashSet};
use std::ops::DerefMut;

use bevy_ecs::prelude::*;
use glam::{quat, Vec3};
use rapier3d::na::Vector3;
use rapier3d::{prelude::*};
use rapier3d::pipeline::DebugRenderPipeline;
use crate::components::common::{Position, Rotation};
use crate::components::model::{Model, ModelCleanup, ModelEnd, ModelPhysicsQuery, ModelQuery};
use crate::{
    common::state::*,
    render::debug_draw::*,
    components::{bricks::*, physics::*},
};

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
    query_pipeline: QueryPipeline,
    debug_render: DebugRenderPipeline,

    pub anchor_sources: HashMap<Entity, HashSet<Entity>>
}

impl State<PhysicsState> for PhysicsState {
    fn consume(world: &mut World, state: PhysicsState) {
        world.insert_resource(state);
        world.insert_resource(Events::<PhysicsCleanup>::default());
    }
}

impl PhysicsState {
    /// Initialize physics scene. Doesn't require anything before it. 
    pub fn new() -> Self {
        let rigid_bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();

        let parameters= IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();

        let debug_render = DebugRenderPipeline::new(
            DebugRenderStyle::default(),
            DebugRenderMode::default()
        );
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

            anchor_sources: HashMap::new()
        }
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
            &event_handler
        );
    }

    /*
        BRICK AND MODEL RELATED FUNCTIONS
     */

    /// Handle solitary bricks 
    pub fn setup_solo_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>, 
        anchored_bricks: Query<PhysicsQuery, (Without<ChildOf>, With<AnchoredTo>)>,
        bricks: Query<PhysicsQuery, (Without<ChildOf>, Without<AnchoredTo>)>)
    {
        let state= state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;


        let mut build_brick = 
            |brick: PhysicsQueryItem, builder: RigidBodyBuilder, commands: &mut Commands, colliders: &mut ColliderSet| 
        {
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
                .user_data(brick.entity.index() as u128)
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
                commands.entity(brick.entity).insert(ShapeHandle(shape_handle));
            } else {
                build_brick(brick, RigidBodyBuilder::dynamic(), &mut commands, colliders);
            }
        }
    }

    pub fn setup_model_bricks(
        mut commands: Commands, 
        mut state: ResMut<PhysicsState>,
        models: Query<ModelQuery>,
        bricks: Query<PhysicsQuery>) 
    {
        let state= state.deref_mut();
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
                    .translation(vector![child_item.position.x, child_item.position.y, child_item.position.z])
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
            }.user_data(item.entity.index() as u128);
            let body_handle = rigid_bodies.insert(body);
            for (child, shape) in shapes {
                let handle = colliders.insert_with_parent(shape, body_handle, rigid_bodies);
                commands.entity(child).insert(ShapeHandle(handle));
            }
            commands.entity(item.entity).insert(BodyHandle(body_handle));
        }
    }


    pub fn handle_deletion(
        mut commands: Commands, 
        mut state: ResMut<PhysicsState>,
        mut anchored_solo_bricks: Query<(&BodyHandle, &mut AnchoredTo), Without<ChildOf>>, 
        mut anchored_model_bricks: Query<(&mut AnchoredTo, &ChildOf)>,
        mut models: Query<(&mut Model, &BodyHandle)>,
        mut deleted: EventReader<PhysicsCleanup>) 
    {
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
            let connected = anchor_sources
                .remove(&cleanup.entity)
                .unwrap();


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
                    commands.entity(anchored).remove::<AnchoredTo>();

                // Model Bricks 
                } else if let Ok((mut set, parent)) = anchored_model_bricks.get_mut(anchored) {
                    set.0.remove(&cleanup.entity);

                    if set.0.len() > 0 {
                        continue 
                    }

                    let (mut model, handle) = models
                        .get_mut(parent.0)
                        .expect("???");
                
                    model.anchored.remove(&anchored);
                    commands.entity(anchored).remove::<AnchoredTo>();

                    if !model.anchored.is_empty() {
                        continue
                    }

                    let Some(body) = rigid_bodies.get_mut(handle.0) else {
                        continue;
                    };

                    body.set_body_type(RigidBodyType::Dynamic, true);
                }

            }

            let handle = cleanup.shape.expect("Removing anchored brick, expected collider handle");
            colliders.remove(handle.0, islands, rigid_bodies, false);
        }
    } 

    // Called when bricks are added into the scene 
    // Note: Bricks covered under setup_* are not handled under this 
    pub fn add_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>, 
        new_bricks: Query<BrickPhysicsQuery, (BrickFilterAdded, Without<ShapeHandle>, Without<BodyHandle>)> 
    ) {
        
        let state= state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;


        for brick in new_bricks {
            let size = brick.size.0 / 2.0; 
            let pos = brick.position;
            let (yaw, pitch, roll) = {
                let (axis, angle) = brick.rotation.to_axis_angle();
                (axis.x * angle, axis.y * angle, axis.z * angle)
            };

            let shape_builder = ColliderBuilder::cuboid(size.x, size.y, size.z)
                .restitution(0.4);

            if brick.physical.anchored {
                let shape = shape_builder 
                    .translation(vector![pos.x, pos.y, pos.z])
                    .rotation(vector![yaw, pitch, roll])
                    .build();

                let handle = colliders.insert(shape);
                commands 
                    .entity(brick.entity)
                    .insert(ShapeHandle(handle));
            } else {

                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                    .restitution(0.4)
                    .build();

                let body = RigidBodyBuilder::dynamic() 
                    .translation(vector![pos.x, pos.y, pos.z])
                    .rotation(vector![yaw, pitch, roll])
                    .user_data(brick.entity.index() as u128)
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


    /// Process models being deleted or destructured 
    /* 
    pub fn process_components(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>,
        mut loose_bricks: RemovedComponents<ChildOf>,
        mut model_er: EventReader<ModelCleanup>,
        shapes: Query<&ShapeHandle>) 
    {
        if model_er.is_empty() {
            return 
        }

        let state = state.deref_mut();
        let rigid_bodies = &mut state.rigid_bodies;
        let islands = &mut state.island_manager;
        let colliders = &mut state.colliders;
        let impulse_joints = &mut state.impulse_joint_set;
        let multibody_joints = &mut state.multibody_joint_set;

        // Save linear and angular velocity to reflect on children 
        let mut lin_ang_map= HashMap::new();

        for e in model_er.read() {
            let handle = e.handle;
            let children = &e.children;
            let body = rigid_bodies.get(handle).expect("Couldn't get model rigid body handle");


            if e.mode == ModelEnd::Destructure {
                let linvel = body.linvel().clone();
                let angvel = body.angvel().clone();

                for child in children {
                    lin_ang_map.insert(child, (linvel, angvel));
                }
            }


            let remove_children= e.mode == ModelEnd::Delete; 
            _ = rigid_bodies.remove(
                handle,
                islands, 
                colliders, 
                impulse_joints, 
                multibody_joints,
               remove_children
            );
        }

        for e in loose_bricks.read() {
            // If brick is removed alongside parent. Remember, this is ALL removed child compponents. 
            if shapes.get(e).is_err() {
                continue 
            }

            let h = shapes.get(e).expect("Couldn't get shape").0;
            let c = colliders.get(h).expect("Couldn't get collider");
            let trans = c.position().translation;

            // Gimbal Lock trickery 
            let (axis, angle) = c
                .rotation()
                .axis_angle()
                .unwrap_or((Vector3::z_axis(), 0.0));
            let ang_vector = axis.into_inner() * angle;

            let lin_ang = {
                lin_ang_map.get(&e.index()).unwrap()
            };

            let body = RigidBodyBuilder::dynamic()
                .translation(vector![trans.x, trans.y, trans.z])
                .rotation(ang_vector)
                .linvel(lin_ang.0)
                .angvel(lin_ang.1)
                .user_data(e.index() as u128)
                .build();

            let body_handle = {
                rigid_bodies.insert(body)
            }; 
            colliders.set_parent(h, Some(body_handle), rigid_bodies);


            commands.entity(e).insert(BodyHandle(body_handle));
        }

    }
    */

    /// Push updated physics state onto the relevant components 
    /// Works best after PhysicsState::step 
    pub fn update_bricks(state: Res<PhysicsState>, 
        models: Query<ModelQuery>,
        mut solo_bricks: Query<BrickPhysicsUpdate, Without<ChildOf>>,
        mut model_bricks: Query<(&mut Position, &mut Rotation, &ShapeHandle), With<ChildOf>>)
    {

        let rigid_bodies = &state.rigid_bodies;
        let colliders = &state.colliders;


        for (_handle, body) in rigid_bodies.iter() {
            if body.is_sleeping() {
                continue 
            }

            let e = Entity::from_raw(body.user_data as u32);

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
            &state.narrow_phase
        );
    }
}
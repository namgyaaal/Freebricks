use std::ops::DerefMut;

use bevy_ecs::prelude::*;
use glam::{quat, Vec3};
use rapier3d::{prelude::*};
use rapier3d::pipeline::DebugRenderPipeline;
use crate::components::common::{Position, Rotation};
use crate::components::model::{Model, ModelQuery};
use crate::{
    common::state::*,
    render::debug_draw::*,
    components::{bricks::*, physics::*},
};

#[derive(Resource)]
pub struct PhysicsState {
    rigid_bodies: RigidBodySet,
    colliders: ColliderSet,

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
}

impl State<PhysicsState> for PhysicsState {
    fn consume(world: &mut World, state: PhysicsState) {
        world.insert_resource(state);
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
        }
    }


    /// Initialize a scene
    /// Assumes that models and bricks are built beforehand such relationships are graphed correctly 
    pub fn init_scene(
        mut state: ResMut<PhysicsState>, 
        bricks: Query<BrickPhysicsQuery, (With<Brick>, Without<Owned>)>, 
        owned_bricks: Query<BrickPhysicsQuery, (With<Brick>, With<Owned>)>,
        models: Query<(Entity, &mut Model)>,
        mut commands: Commands) 
    {
        // Need to deref like this to get around borrow checker
        // Figure out *why* you need to do this so you don't shoot yourstate in the foot later.  
        let state= state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;


        // Handle Bricks not owned by models         
        for brick in bricks {
            let size = brick.size.0 / 2.0; 
            let pos = brick.position;
            let (yaw, pitch, roll) = {
                let (axis, angle) = brick.rotation.to_axis_angle();

                (axis.x * angle, axis.y * angle, axis.z * angle)
            };

            if brick.physical.anchored {
                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z) 
                    .translation(vector![brick.position.x, brick.position.y, brick.position.z])
                    .rotation(vector![yaw, pitch, roll])
                    .restitution(0.4)
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

                let body_handle = {
                    rigid_bodies.insert(body)
                };
                let shape_handle = {
                    colliders
                        .insert_with_parent(shape, body_handle, rigid_bodies)
                };
                commands 
                    .entity(brick.entity)
                    .insert(BodyHandle(body_handle)) 
                    .insert(ShapeHandle(shape_handle));
            }
        }

        // Handle models & bricks owned by them 
        for (model_id, model) in models.iter() {

            let mut brick_shapes= Vec::new();

            for e in &model.set { 
                if let Ok(brick) = owned_bricks.get(*e) {


                    let size = brick.size.0 / 2.0;
                    let shape = ColliderBuilder::cuboid(size.x, size.y, size.z)
                        .translation(vector![brick.position.x, brick.position.y, brick.position.z])
                        .restitution(0.4)
                        .build();
                    brick_shapes.push((e, shape));
                }
            }

            let body = RigidBodyBuilder::dynamic()
                .user_data(model_id.index() as u128)
                .build();

            let body_handle = rigid_bodies.insert(body);

            for (e, shape) in brick_shapes {
                let handle = colliders.insert_with_parent(shape, body_handle, rigid_bodies);
                commands.entity(*e).insert(ShapeHandle(handle));
            }
            commands.entity(model_id).insert(BodyHandle(body_handle));
        }



    }

    // Called when bricks are added into the scene 
    // Note: Bricks covered under "init_scene" are not included under this (unlike SceneTree)
    //      This is because models aren't handled for now for new bricks after the initial scene 
    pub fn add_bricks(
        mut commands: Commands,
        mut state: ResMut<PhysicsState>, 
        query: Query<BrickPhysicsQuery, (BrickFilterAdded, Without<ShapeHandle>)>
    ) {
        
        let state= state.deref_mut();
        let colliders = &mut state.colliders;
        let rigid_bodies = &mut state.rigid_bodies;

        for brick in query.iter() {
            let size = brick.size.0 / 2.0; 
            let pos = brick.position;
            let (yaw, pitch, roll) = {
                let (axis, angle) = brick.rotation.to_axis_angle();

                (axis.x * angle, axis.y * angle, axis.z * angle)
            };
            // rapier3d supports rigidbody-less collisionshapes that are still solid. 
            if brick.physical.anchored {
                let shape = ColliderBuilder::cuboid(size.x, size.y, size.z) 
                    .translation(vector![brick.position.x, brick.position.y, brick.position.z])
                    .rotation(vector![yaw, pitch, roll])
                    .restitution(0.4)
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

                let body_handle = {
                    rigid_bodies.insert(body)
                };
                let shape_handle = {
                    colliders
                        .insert_with_parent(shape, body_handle, rigid_bodies)
                };
                commands 
                    .entity(brick.entity)
                    .insert(BodyHandle(body_handle)) 
                    .insert(ShapeHandle(shape_handle));
            }
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

    /// Push updated physics state onto the relevant components 
    /// Works best after PhysicsState::step 
    pub fn update_components(state: Res<PhysicsState>, 
        mut query: Query<BrickPhysicsUpdate, Without<Owned>>, 
        models: Query<ModelQuery>,
        mut owned_bricks: Query<(&mut Position, &mut Rotation, &ShapeHandle), With<Owned>>) {
        let rigid_bodies = &state.rigid_bodies;
        let colliders = &state.colliders;


        for (_handle, body) in rigid_bodies.iter() {
            if body.is_sleeping() {
                continue 
            }

            let e = Entity::from_raw(body.user_data as u32);

            if let Ok(mut brick) = query.get_mut(e) {
                let pos = body.position().translation;
                let rot = body.position().rotation;

                brick.position.0 = Vec3::new(pos.x, pos.y, pos.z);
                brick.rotation.0 = quat(rot.i, rot.j, rot.k, rot.w);
            } else if let Ok(model) = models.get(e) {
                for brick_index in &model.model.set{
                    if let Ok((mut p, mut r, h)) =owned_bricks.get_mut(*brick_index) {
                        let pos = colliders.get(h.0).unwrap().translation();
                        let rot = colliders.get(h.0).unwrap().rotation();
                        r.0 = quat(rot.i, rot.j, rot.k, rot.w);
                        p.0 = Vec3::new(pos.x, pos.y, pos.z);
                    }
                }
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
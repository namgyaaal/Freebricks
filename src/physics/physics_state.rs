use bevy_platform::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::ops::DerefMut;

use crate::ecs::common::{Position, Rotation};
use crate::ecs::model::QModel;
use crate::physics::transform::{
    handle_anchor_queue, handle_model_unanchor, handle_part_unanchor, handle_submodel,
    handle_subpart,
};
use crate::{
    common::state::*,
    ecs::{parts::*, physics::*},
    render::debug_draw::*,
};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;
use glam::{Vec3, quat};
use rapier3d::pipeline::DebugRenderPipeline;
use rapier3d::prelude::*;

use super::{deletion::*, setup::*};

#[derive(Resource)]
/// Used until bevy-ecs implements many-to-many
/// Keeps track of anchors having anchored relations
pub struct AnchorMap {
    pub anchors: HashMap<Entity, HashSet<Entity>>,
    pub delete_queue: VecDeque<Entity>,
}

#[derive(Resource)]
pub struct PhysicsState {
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,

    parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    debug_render: DebugRenderPipeline,
}

impl State<PhysicsState> for PhysicsState {
    fn consume(world: &mut World, state: PhysicsState) {
        world.insert_resource(state);
        world.insert_resource(AnchorMap {
            anchors: HashMap::new(),
            delete_queue: VecDeque::new(),
        });
        world.add_observer(handle_shape_removal);
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
            handle_subpart,
            handle_submodel,
            handle_anchor_queue,
            handle_part_unanchor,
            handle_model_unanchor,
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

use crate::{
    common::{asset_cache::AssetCache, model_graph::*, state::State},
    ecs::{common::*, parts::*, physics::*},
    physics::PhysicsState,
    render::{
        camera::Camera,
        debug_draw::DebugDraw,
        parts::part_renderer::PartRenderer,
        render_state::{FrameInfo, RenderState},
    },
};
use anyhow::Result;
use bevy_ecs::prelude::*;
use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::sync::Arc;
use tracing::error;
use winit::{dpi::PhysicalSize, window::Window};

#[derive(Component)]
pub struct Tag1;

#[allow(unused)]
pub fn foobar(
    mut commands: Commands,
    mut count: Local<u64>,
    mut test: Query<(Entity, &Part)>,
    mut storage: Local<Vec<Vec3>>,
) {
    //let mut rng = SmallRng::seed_from_u64(*count);
    /*
    let mut rng = SmallRng::seed_from_u64(*count);
    if *count % 30 == 0 {
        commands.spawn((
            Part::Ball,
            Position(Vec3::new(
                f32::cos(rng.random()),
                20.0,
                f32::cos(rng.random()),
            )),
            Physical,
            Size(Vec3::new(2.0, 2.0, 2.0)),
            Color([rng.random(), rng.random(), rng.random(), rng.random()]),
        ));
    }
    *count += 1;*/
}

pub struct Game {
    //pub render_state: RenderState,
    pub world: World,
    pub update: Schedule,
    pub post_update: Schedule,
    pub render: Schedule,
}

impl Game {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let render_state = RenderState::new(window.clone())
            .await
            .expect("Game::new(), couldn't create Render State");

        let mut world = World::new();

        let asset_cache = AssetCache::init("assets").expect("Unable to load from asset directory");

        world.insert_resource(asset_cache);

        let physics_state = PhysicsState::new();
        let mut init_schedule = Schedule::default();
        let mut update_schedule = Schedule::default();

        let mut post_update_schedule = Schedule::default();
        let mut render_schedule = Schedule::default();

        RenderState::consume(&mut world, render_state);
        PhysicsState::consume(&mut world, physics_state);

        world.add_observer(handle_part_of_model_deletion);
        /*
            Game scene updating.

        */

        init_schedule.add_systems(
            (
                Camera::init,
                PartRenderer::init,
                DebugDraw::init,
                build_models,
                //PhysicsState::setup_system(),
            )
                .chain(),
        );
        update_schedule.add_systems((foobar, Camera::update).chain());

        post_update_schedule.add_systems(
            (
                handle_model_transform,
                PhysicsState::update_system(true),
                PartRenderer::remove_bricks,
                PartRenderer::add_bricks,
                PartRenderer::update_bricks,
            )
                .chain(),
        );

        render_schedule.add_systems(
            (
                PartRenderer::write_buffers,
                RenderState::begin_pass,
                PartRenderer::render,
                DebugDraw::render,
                RenderState::flush,
                PartRenderer::recall,
            )
                .chain(),
        );

        let mut rng = SmallRng::seed_from_u64(42);

        world.spawn((
            Part::default(),
            Position(Vec3::new(0.0, 0.0, 0.0)),
            Size(Vec3::new(100.0, 1.0, 100.0)),
            Color([rng.random(), rng.random(), rng.random(), 255]),
            Anchor,
        ));

        let mut parts = Vec::new();

        for x in -50..50 {
            for y in 0..40 {
                let part_type = match rng.random::<u8>() % 3 {
                    0 => Part::Brick,
                    1 => Part::Ball,
                    2 => Part::Wedge,
                    _ => Part::Brick,
                };

                parts.push((
                    part_type,
                    Position(Vec3::new(x as f32, 3.0, y as f32)),
                    Size(Vec3::new(1.0, 1.0, 1.0)),
                    Color([rng.random(), rng.random(), rng.random(), 128]),
                ))
            }
        }

        let _ = world.spawn_batch(parts).collect::<Vec<Entity>>();

        // Initialize states and globals, don't need it further and we only pass on update and render
        init_schedule.run(&mut world);
        Ok(Self {
            world: world,
            update: update_schedule,
            post_update: post_update_schedule,
            render: render_schedule,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let mut state = self.world.get_resource_mut::<RenderState>().unwrap();

        state.resize(size.width, size.height);
        Ok(())
    }

    pub fn update(&mut self) {
        // Update
        self.update.run(&mut self.world);
        self.post_update.run(&mut self.world);

        // We don't really need to do ECS for rendering, all relevant information should be passed to proper globals
        // e.g., ResMut<SceneTree> should have buffers generated by now here.
        let mut state = self.world.get_resource_mut::<RenderState>().unwrap();
        match state.begin_frame() {
            Ok(None) => {}
            Ok(Some(frame)) => {
                let mut old_frame = self.world.get_resource_mut::<FrameInfo>().unwrap();
                *old_frame = frame;
                self.render.run(&mut self.world);
            }
            Err(e) => {
                error!("Error rendering: {e}");
            }
        }

        self.world.clear_trackers();
    }
}

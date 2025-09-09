use crate::{
    common::{asset_cache::AssetCache, model_graph::*, state::State},
    ecs::{common::*, parts::*, physics::*},
    physics::PhysicsState,
    render::{
        camera::Camera,
        debug_draw::DebugDraw,
        render_state::{FrameInfo, RenderState},
        scene_tree::SceneTree,
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
    return;
    if *count < 60 {
        *count += 1;
        return;
    }

    for (e, mut c) in test.iter_mut() {
        commands.entity(e).despawn();
        *count = 0;
        return;
        /*

        let e_index = e.index() as usize;

        if *count == 0 {
            if e_index >= storage.len() {
                storage.resize(e_index + 1, Vec3::ZERO);
            }
            storage[e_index] = c.0;
        }

        c.x = storage[e_index].x + (*count as f32 / 120.0).cos() * 100.0;
        c.z = storage[e_index].z + (*count as f32 / 120.0).sin() * 100.0;
        //if rng.random_bool(0.01) {
        //    *c = Color([rng.random(), rng.random(), rng.random(), 255]);
        //}*/
    }
    *count += 1;
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
                SceneTree::init,
                DebugDraw::init,
                build_models,
                PhysicsState::setup_system(),
            )
                .chain(),
        );
        update_schedule.add_systems((foobar, Camera::update).chain());

        post_update_schedule.add_systems(
            (
                handle_model_transform,
                PhysicsState::update_system(false),
                SceneTree::remove_bricks,
                SceneTree::add_bricks,
                SceneTree::update_bricks,
            )
                .chain(),
        );

        render_schedule.add_systems(
            (
                SceneTree::write_buffers,
                RenderState::begin_pass,
                SceneTree::render,
                DebugDraw::render,
                RenderState::flush,
                SceneTree::recall,
            )
                .chain(),
        );

        let mut parts = Vec::new();

        let mut rng = SmallRng::seed_from_u64(42);

        world.spawn((
            Part::default(),
            Position(Vec3::new(0.0, 0.0, 0.0)),
            Size(Vec3::new(100.0, 1.0, 100.0)),
            Color([rng.random(), rng.random(), rng.random(), 255]),
            Anchor,
        ));

        for i in -32..32 {
            for j in -4..4 {
                let x: f32 = (rand::random::<u8>() % 4) as f32;
                let y: f32 = (rand::random::<u8>() % 4) as f32;

                parts.push((
                    Part::Brick,
                    Position(Vec3::new(
                        (i * 6) as f32,
                        y + 3.0 + (rng.random_range(0..20) as f32),
                        (j * 6) as f32,
                    )),
                    Size(Vec3::new(1.0 + x, 1.0 + x, 1.0 + x)),
                    Color([rand::random(), rand::random(), rand::random(), 255]),
                    Physical,
                ));
            }
        }
        /*
        // Do anything here
        let mut parts = Vec::new();
        world.spawn((
            Part::default(),
            Position(Vec3::new(0.0, -7.0, 0.0)),
            Size(Vec3::new(20.0, 1.0, 20.0)),
            Tag1,
        ));

        parts.push((
            Part::default(),
            Position(Vec3::new(0.0, -10.0, 0.0)),
            Color([rand::random(), rand::random(), rand::random(), 255]),
        ));

        parts.push((
            Part::default(),
            Position(Vec3::new(0.0, -8.0, 0.0)),
            Color([rand::random(), rand::random(), rand::random(), 255]),
        ));
        /*
        parts.push((
            Part::default(),
            Position(Vec3::new(0.0, -7.0, 0.0)),
            Physical,
            Color([rand::random(), rand::random(), rand::random(), 255]),
        ));
        */
        parts.push((
            Part::default(),
            Position(Vec3::new(0.0, -11.0, 0.0)),
            Color([rand::random(), rand::random(), rand::random(), 255]),
        ));*/

        /*
        world.spawn((
            Part::default(),
            Position(Vec3::new(0.0, -9.0, 0.0)),
            Physical,
            Color([rand::random(), rand::random(), rand::random(), 255]),
        ));*/

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

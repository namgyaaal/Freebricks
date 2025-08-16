use bevy_ecs::{prelude::*};
use rand::Rng;
use winit::{dpi::PhysicalSize, window::Window};
use std::sync::Arc;
use anyhow::Result;
use tracing::{error};
use glam::{Vec3};


pub fn foobar(mut commands: Commands, mut count: Local<u64>) {
    let mut rng = rand::rng();

    if *count % 30 == 0  && *count > 120 {
        commands.spawn((
            Brick::default(), 
            Position(Vec3::new(
                rng.random_range(-50.0..=50.0), 
                rng.random_range(2.0..=50.0),
                rng.random_range(-50.0..=50.0))
            ),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical{ anchored: false },
            Color([rand::random(), rand::random(), rand::random(), 255])            
        ));
    }

    *count += 1; 
}


use crate::{
    common::{asset_cache::AssetCache, state::{State}, model_builder::*}, components::{bricks::Brick, common::*, physics::*}, physics::physics_state::PhysicsState, render::{
        camera::Camera, debug_draw::DebugDraw, render_state::{RenderOptions, RenderPassInfo, RenderState}, scene_tree::SceneTree
    }
};
pub struct Game {
    //pub render_state: RenderState,
    pub world: World,
    pub update: Schedule,
    pub post_update: Schedule,
    pub render: Schedule
}

impl Game {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let render_state = RenderState::new(
            window.clone(),
            RenderOptions::RenderTimestamps.into()
        ).await.expect("Game::new(), couldn't create Render State");


        let mut world = World::new();
        
        let asset_cache = AssetCache::init("assets")
            .expect("Unable to load from asset directory");        

        world.insert_resource(asset_cache);

        let physics_state = PhysicsState::new();
        let mut init_schedule = Schedule::default();
        let mut update_schedule = Schedule::default();

        let mut post_update_schedule = Schedule::default();
        let mut render_schedule = Schedule::default();

        
        RenderState::consume(&mut world, render_state);
        PhysicsState::consume(&mut world, physics_state);
        
        /*
            Game scene updating.

        */
        init_schedule.add_systems(
            (   
                Camera::init,
                SceneTree::init,
                DebugDraw::init,
                build_models,
                PhysicsState::init_scene, 
            ).chain()
        );
        update_schedule.add_systems(
            (
                PhysicsState::add_bricks,
                foobar,
            ).chain()
        );

        post_update_schedule.add_systems((
            PhysicsState::step,  
            PhysicsState::write_debug.after(PhysicsState::step), 
            PhysicsState::update_components.after(PhysicsState::step), 
            SceneTree::add_bricks.before(SceneTree::update_bricks), 
            SceneTree::update_bricks,
        ));

        render_schedule.add_systems((
            SceneTree::render.before(RenderState::flush),
            DebugDraw::render.before(RenderState::flush),
            RenderState::flush
        ));

        // Do anything here 
        let mut bricks = Vec::new();
        bricks.push((
            Brick::default(), 
            Position::default(),
            Size(Vec3::new(100.0, 1.0, 100.0)),
            Physical::default(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));


        for i in 50..70 {
            let y = i as f32 * 2.0; 

            bricks.push((
                Brick::default(), 
                Position(Vec3::new(0.0, y, 0.0)),
                Size::default(),
                Physical::dynamic(),
                Color([rand::random(), rand::random(), rand::random(), 255])
            ));
        }

        for i in 15..25 {
            let y = i as f32; 

            bricks.push((
                Brick::default(), 
                Position(Vec3::new(0.0, y, 0.0)),
                Size(Vec3::new(10.0 - (20.0 - y).abs() as f32, 1.0, 10.0 - (20.0 - y).abs())),
                Physical::dynamic(),
                Color([rand::random(), rand::random(), rand::random(), 255])
            ));
        }


        for i in 35..45 {
            let y = i as f32; 

            bricks.push((
                Brick::default(), 
                Position(Vec3::new(0.0, y, 0.0)),
                Size(Vec3::new(10.0 - (40.0 - y).abs() as f32, 1.0, 10.0 - (40.0 - y).abs())),
                Physical::dynamic(),
                Color([rand::random(), rand::random(), rand::random(), 255])
            ));
        }

       let _ = world.spawn_batch(bricks).collect::<Vec<Entity>>();



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
        let mut state = self.world
            .get_resource_mut::<RenderState>()
            .unwrap();

        state.resize(size.width, size.height);
        Ok(())
    }

    pub fn update(&mut self) {
        // Update 
        self.update.run(&mut self.world);
        self.post_update.run(&mut self.world);

        // We don't really need to do ECS for rendering, all relevant information should be passed to proper globals 
        // e.g., ResMut<SceneTree> should have buffers generated by now here. 
        let mut state = self.world
            .get_resource_mut::<RenderState>()
            .unwrap();
        match state.begin_pass() {
            Ok(None) => {},
            Ok(Some(new_info)) => {
                let mut info = self.world
                    .get_resource_mut::<RenderPassInfo>()
                    .unwrap();

                *info = new_info;
                self.render.run(&mut self.world);
            },
            Err(e) => {
                error!("Error rendering: {e}");
            }
        }
    }

}
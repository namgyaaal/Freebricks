use bevy_ecs::{prelude::*};
use winit::{dpi::PhysicalSize, window::Window};
use std::sync::Arc;
use anyhow::Result;
use tracing::{error};
use glam::{Vec3};
use crate::{
    common::{asset_cache::AssetCache, model_builder::*, state::State}, 
    components::{bricks::{Brick}, 
    common::*, 
    model::{ModelCleanup, ModelPhysicsQuery, ModelEnd, ModelPhysicsQueryItem}, 
    physics::*, render::{BufferIndex, RenderCleanup}}, 
    physics::physics_state::PhysicsState, 
    render::{
        camera::Camera, 
        debug_draw::DebugDraw, 
        render_state::{RenderOptions, RenderPassInfo, RenderState}, 
        scene_tree::SceneTree
    }
};



pub fn destructure_model(commands: &mut Commands, model: &ModelPhysicsQueryItem) -> ModelCleanup {
    let mut children = Vec::new();
    for &child in model.children {
        children.push(child.index());
    }

    let mut ec = commands.entity(model.entity);

    ec.remove_children(model.children);
    ec.despawn();
    ModelCleanup { handle: model.body.0, children: children, mode: ModelEnd::Destructure }
}

pub fn delete_model(commands: &mut Commands, model: &ModelPhysicsQueryItem, indices: Query<&BufferIndex>)
 -> (ModelCleanup, Vec<RenderCleanup>) {
    let mut render_cleanup= Vec::new();
    let mut children = Vec::new();
    for &child in model.children {
        children.push(child.index());
        if let Ok(index) = indices.get(child) {
            render_cleanup.push(RenderCleanup {buffer_index: *index});
        }
    }

    let mut ec = commands.entity(model.entity);
    ec.despawn();

    let cleanup = ModelCleanup { handle: model.body.0, children: children, mode: ModelEnd::Delete };

    (cleanup, render_cleanup)
}

pub fn foobar(mut commands: Commands, 
              mut count: Local<u64>, 
              models: Query<ModelPhysicsQuery>, 
              mut ew_model: EventWriter<ModelCleanup>,
              mut ew_render: EventWriter<RenderCleanup>,
              indices: Query<&BufferIndex>) 
{

    if *count == 480 {
        let mut i = 0;
        for model in models.iter() {

            if i % 2 != 0 {
                let mc = destructure_model(&mut commands, &model);
                ew_model.write(mc);
            } else {

                let (mc, rc) = delete_model(&mut commands, &model, indices);

                ew_model.write(mc);
                ew_render.write_batch(rc.into_iter());
            }
            i += 1;
        }

    }

    *count += 1; 
}



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

        world.insert_resource(Events::<ModelCleanup>::default());
        world.insert_resource(Events::<RenderCleanup>::default());

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
                foobar,
                PhysicsState::add_bricks,
            ).chain()
        );

        post_update_schedule.add_systems((
            PhysicsState::step,  
            PhysicsState::write_debug.after(PhysicsState::step), 

            PhysicsState::process_components.before(PhysicsState::update_components),
            PhysicsState::update_components.after(PhysicsState::step), 

            SceneTree::remove_bricks.before(SceneTree::add_bricks), 
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
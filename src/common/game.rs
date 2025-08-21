use bevy_ecs::{prelude::*};
use winit::{dpi::PhysicalSize, window::Window};
use std::{ops::Range, sync::Arc};
use anyhow::Result;
use tracing::{error};
use glam::{Vec2, Vec3};
use crate::{
    common::{asset_cache::AssetCache, model_graph::*, state::State}, 
    ecs::{bricks::{Brick, BrickCleanup}, common::*, model::Model, physics::*, render::{BufferIndex, RenderCleanup}}, 
    physics::physics_state::PhysicsState, 
    render::{
        camera::Camera, 
        debug_draw::DebugDraw, 
        render_state::{RenderOptions, RenderPassInfo, RenderState}, 
        scene_tree::SceneTree
    }
};

#[derive(Component)]
pub struct Tag1;

/* 
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
*/

pub fn foobar(mut commands: Commands, 
              mut count: Local<u64>, 
              test: Query<(Entity, &Tag1)>,
              mut ew_render: EventWriter<RenderCleanup>,
              mut ew_physics: EventWriter<PhysicsCleanup>,
              indices: Query<&BufferIndex>,
              shapes: Query<&ShapeHandle>,
              bodies: Query<&BodyHandle>, 
              parents: Query<&ChildOf>,
              mut models: Query<&mut Model>) 
{
    if *count == 180 {
        for (e, _) in test {

            let parent_id = parents.get(e).unwrap().0;
            let mut model = models.get_mut(parent_id).unwrap();

            
            model.anchored.remove(&e);
            model.graph.remove_node(e);
            model.set.remove(&e);

            commands.entity(parent_id).remove_children(&[e]);

            /* 
            commands.entity(e).despawn();

            let rc = RenderCleanup {buffer_index: *indices.get(e).unwrap() };
            let pc = PhysicsCleanup {
                entity: e,
                shape: shapes.get(e).ok().map(|x| *x),
                body: bodies.get(e).ok().map(|x| *x),
                parent: parents.get(e).ok().map(|x| x.0)
            };
            ew_render.write(rc);
            ew_physics.write(pc);       
            */

            /* 
            commands.entity(t.0).despawn();


            let rc = RenderCleanup {buffer_index: *indices.get(t.0).unwrap() };
            let pc = PhysicsCleanup {
                entity: t.0,
                shape: shapes.get(t.0).ok().map(|x| *x),
                body: bodies.get(t.0).ok().map(|x| *x),
                parent: parent.get(t.0).ok().map(|x| x.0)
            };
            ew_render.write(rc);
            ew_physics.write(pc);
            break;
            */
        }
    }
    *count += 1;
    /* 
    if *count == 180 {
        let mut i = 0;
        for model in models.iter() {

            if i % 3 == 0 {
                let mc = destructure_model(&mut commands, &model);
                ew_model.write(mc);
            } else if i % 3 == 1{

                let (mc, rc) = delete_model(&mut commands, &model, indices);

                ew_model.write(mc);
                ew_render.write_batch(rc.into_iter());
            }
            i += 1;
        }

    }

            bricks.push((
            Brick::default(), 
            Position(Vec3::new(0.0, 0.0, 0.0)),
            Size(Vec3::new(20.0, 2.0, 20.0)),
            Physical::default(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));

    *count += 1; */
    //if *count == 100 {
    //    commands.spawn((
    //        Brick::default(),
    //        Position(Vec3::new(0.0, 20.0, 0.0)),
    //        Physical::dynamic()
    //    ));
    //}
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

        world.insert_resource(Events::<BrickCleanup>::default());
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
                PhysicsState::setup_solo_bricks, 
                PhysicsState::setup_model_bricks, 
            ).chain()
        );
        update_schedule.add_systems(
            (
                foobar,
            ).chain()
        );

        post_update_schedule.add_systems((
            (   
                PhysicsState::step,
                PhysicsState::write_debug,
                PhysicsState::add_bricks,
                PhysicsState::handle_deletion, 
                PhysicsState::handle_deletion_two,
                PhysicsState::handle_pop, 
                PhysicsState::handle_model_mutations,
                PhysicsState::update_bricks
            ).chain(),
            (
                SceneTree::remove_bricks, 
                SceneTree::add_bricks, 
                SceneTree::update_bricks
            ).chain()
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
            Position(Vec3::new(0.0, -20.0, 0.0)),
            Size(Vec3::new(20.0, 2.0, 20.0)),
            Physical::default(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));
        
        /* 
        bricks.push((
            Brick::default(), 
            Position(Vec3::new(2.0, 5.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::dynamic(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));
        */
        bricks.push((
            Brick::default(), 
            Position(Vec3::new(2.0, 6.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::dynamic(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));

        bricks.push((
            Brick::default(), 
            Position(Vec3::new(2.0, 7.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::dynamic(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));

        bricks.push((
            Brick::default(), 
            Position(Vec3::new(2.0, 8.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::dynamic(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));
       

        world.spawn((
            Brick::default(), 
            Position(Vec3::new(2.0, 9.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::dynamic(),
            Color([rand::random(), rand::random(), rand::random(), 255]),
            Tag1
        ));

        bricks.push((
            Brick::default(), 
            Position(Vec3::new(2.0, 10.0, 0.0)),
            Size(Vec3::new(4.0, 1.0, 4.0)),
            Physical::anchored(),
            Color([rand::random(), rand::random(), rand::random(), 255])
        ));
       



        let mut _make_model = |range: Range<u32>, xz: Vec2| {
            for i in range {
                let y = i as f32; 

                bricks.push((
                    Brick::default(),
                    Position(Vec3::new(xz.x, y, xz.y)),
                    Size(Vec3::new(4.0, 1.0, 4.0)),
                    Physical::dynamic(), 
                    Color([rand::random(), rand::random(), rand::random(), 255]),
                ));
            }

        };



        //make_model(15..25, Vec2::new(0.0, 0.0));

        //make_model(30..40, Vec2::new(5.0, 5.0));

        //make_model(45..50, Vec2::new(5.0, 5.0));

        /* 
        make_model(15..25, Vec2::new(10.0, 0.0));

        make_model(30..40, Vec2::new(15.0, 5.0));

        make_model(45..50, Vec2::new(10.0, 5.0));

        */

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
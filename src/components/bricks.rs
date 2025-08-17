use bevy_ecs::{prelude::*, query::QueryFilter};
use bevy_ecs::query::QueryData;
use crate::components::{common::*, render::*, physics::*};


#[derive(Debug)]
/// Handling flat, outlet and inlet for now.
/// In theory should support 16 possible types 
pub enum StudType {
    Flat = 0x00,
    Outlet = 0x01, 
    Inlet = 0x02 
}

#[derive(Component, Debug)]
#[require(Position, Rotation, Color, Size, BufferIndex, RenderMode, Physical)]
/// Component for studtype for bottom and top (also hints at being a Brick)
pub struct Brick {
    pub top: StudType, 
    pub bottom: StudType 
}

#[derive(Component, Debug)]
/// Is it owned by a model?
/// TODO: Look into relationships 
pub struct Owned {}

impl Default for Brick {
    fn default() -> Self {
        Brick {
            top: StudType::Outlet,
            bottom: StudType::Inlet
        }
    }
}

impl Command for Brick {
    fn apply(self, world: &mut World) {

        world.spawn({(
           self,
           RenderMode(RenderModeOption::Instanced)
        )});
    }
}

/*  ---------------------

    Possible QueryDatas 

*/ 

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Default Brick Query for all attributes
pub struct BrickQuery {
    pub entity: Entity, 
    pub brick: &'static Brick, 
    pub position: &'static Position, 
    pub rotation: &'static Rotation, 
    pub size: &'static Size, 
    pub color: &'static Color,
    pub buffer_index: &'static BufferIndex,
    pub render_mode: &'static RenderMode
}


#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Just for getting physics-related attributes 
pub struct BrickPhysicsQuery {
    pub entity: Entity, 
    pub position: &'static Position, 
    pub rotation: &'static Rotation,  
    pub size: &'static Size,
    pub physical: &'static Physical,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
/// Just for getting physics-related attributes and updating them
pub struct BrickPhysicsUpdate {
    pub entity: Entity, 
    pub position: &'static mut Position, 
    pub rotation: &'static mut Rotation,
    pub body_handle: &'static BodyHandle,
}


#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
/// Query is used for changing buffer index and which uniform or instance buffer owns it 
pub struct BrickQueryReorder {
    pub entity: Entity, 
    pub brick: &'static Brick, 
    pub position: &'static Position, 
    pub rotation: &'static Rotation, 
    pub size: &'static Size, 
    pub color: &'static Color,
    pub buffer_index: &'static mut BufferIndex,
    pub render_mode: &'static mut RenderMode
}

/*  ---------------------

    Possible QueryFilters  

*/ 

#[derive(QueryFilter)]
/// Query when a brick is added to the scene 
pub struct BrickFilterAdded {
    _c: Added<Brick>,
}

#[derive(QueryFilter)]
// For non-initial bricks added 
pub struct BrickFilterPhysicsAdded {
    _c: Added<Brick>,
    _d: Without<BodyHandle>
}


#[derive(QueryFilter)] 
/// Query when any render information has changed 
pub struct BrickFilterUpdate {
    _c: With<Brick>,
    _or: Or<(
        Changed<Brick>,
        Changed<Position>,
        Changed<Rotation>,
        Changed<Size>,
        Changed<Color>
    )>
}

/*---------------------

    Event stuff

*/

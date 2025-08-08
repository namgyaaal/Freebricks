use bevy_ecs::{prelude::*, query::QueryFilter};
use bevy_ecs::query::QueryData;
use crate::components::{common::*, render::*, physics::*};


/*
    Handling flat, outlet and inlet for now 
    In theory should support 16 possible types 
*/
#[derive(Debug)]
pub enum StudType {
    Flat = 0x00,
    Outlet = 0x01, 
    Inlet = 0x02 
}

#[derive(Component, Debug)]
#[require(Position, Rotation, Color, Size, BufferIndex, RenderMode, Physical)]
pub struct Brick {
    pub top: StudType, 
    pub bottom: StudType 
}

#[derive(Component, Debug)]
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


/*
    Default Query for all attributes
*/
#[derive(QueryData)]
#[query_data(derive(Debug))]
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

/*
    Just for getting physics-related attributes and updating them 
*/

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct BrickPhysicsQuery {
    pub entity: Entity, 
    pub position: &'static Position, 
    pub rotation: &'static Rotation,  
    pub size: &'static Size,
    pub physical: &'static Physical,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct BrickPhysicsUpdate {
    pub entity: Entity, 
    pub position: &'static mut Position, 
    pub rotation: &'static mut Rotation,
    pub body_handle: &'static BodyHandle,
}


/*
    Query for changing where it is in terms of render layout 
*/
#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
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




/*
    Query for when it has changed and needs to update buffers 
*/
#[derive(QueryFilter)]
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
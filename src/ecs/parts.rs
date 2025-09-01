use crate::ecs::{common::*, physics::*, render::*};
use bevy_ecs::query::QueryData;
use bevy_ecs::{prelude::*, query::QueryFilter};

#[derive(Component, Debug, Default, PartialEq, Eq, Clone, Copy)]
#[require(StudInfo, Position, Rotation, Color, Size, BufferIndex)]
// Encompasses Brick, Wedge, Ball and Mesh
pub enum Part {
    #[default]
    Brick = 0,
    // TODO ----
    Wedge = 1,
    Ball = 2,
    Mesh = 3,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Handling flat, outlet and inlet for now.
/// In theory should support 16 possible types
pub enum StudType {
    Flat = 0x00,
    Outlet = 0x01,
    Inlet = 0x02,
}

#[derive(Component, Debug)]
/// Component for studtype for bottom and top (also hints at being a Brick)
pub struct StudInfo {
    pub top: StudType,
    pub bottom: StudType,
}

impl Default for StudInfo {
    fn default() -> Self {
        StudInfo {
            top: StudType::Outlet,
            bottom: StudType::Inlet,
        }
    }
}

/*  ---------------------

    Queries

*/

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Standard Part Query
pub struct QPart {
    pub entity: Entity,
    pub part: &'static Part,
    pub studs: &'static StudInfo,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub color: &'static Color,
    pub buffer_index: &'static BufferIndex,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
/// Part Query needed for setup functionality on scene start.
pub struct QPartWorldInit {
    pub entity: Entity,
    pub part: &'static Part,
    pub studs: &'static StudInfo,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub physical: &'static Physical,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
/// Query is used for changing buffer index and which uniform or instance buffer owns it
pub struct QPartRenderUpdate {
    pub entity: Entity,
    pub part: &'static Part,
    pub studs: &'static StudInfo,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub color: &'static Color,
    pub buffer_index: &'static mut BufferIndex,
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
/// Query with mutable position and rotation. Used for physics/anything else to push updates onto the position and rotation components
pub struct QPartWorldUpdate {
    pub entity: Entity,
    pub position: &'static mut Position,
    pub rotation: &'static mut Rotation,
}

/*  ---------------------

    Filters

*/

#[derive(QueryFilter)]
/// On Part Add Filter  
pub struct FPartAdd {
    _c: Added<Part>,
}

#[derive(QueryFilter)]
/// Filter for when any parts relevant to rendering change
pub struct FPartChange {
    _c: With<Part>,
    _or: Or<(
        Changed<StudInfo>,
        Changed<Position>,
        Changed<Rotation>,
        Changed<Size>,
        Changed<Color>,
    )>,
}

#[derive(QueryFilter)]
pub struct FPartChangeTransform {
    _c: With<Part>,
    _or: Or<(Changed<Position>, Changed<Rotation>)>,
}

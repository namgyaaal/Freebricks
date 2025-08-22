use crate::ecs::{common::*, physics::*, render::*};
use bevy_ecs::query::QueryData;
use bevy_ecs::{prelude::*, query::QueryFilter};

#[derive(Component, Debug, Default, PartialEq, Eq)]
#[require(StudInfo, Position, Rotation, Color, Size, BufferIndex, Physical)]
// Encompasses Brick, Wedge, Ball and Mesh
pub enum Part {
    #[default]
    Brick,
    // TODO ----
    Wedge,
    Ball,
    Mesh,
}

#[derive(Debug, PartialEq, Eq)]
/// Handling flat, outlet and inlet for now.
/// In theory should support 16 possible types
pub enum StudType {
    Flat = 0x00,
    Outlet = 0x01,
    Inlet = 0x02,
}

#[derive(Component, Debug)]
#[require(Position, Rotation, Color, Size, BufferIndex, RenderMode, Physical)]
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
pub struct QPart {
    pub entity: Entity,
    pub part: &'static Part,
    pub studs: &'static StudInfo,
    pub position: &'static Position,
    pub rotation: &'static Rotation,
    pub size: &'static Size,
    pub color: &'static Color,
    pub buffer_index: &'static BufferIndex,
    pub physical: &'static Physical,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
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
pub struct QPartWorldUpdate {
    pub entity: Entity,
    pub position: &'static mut Position,
    pub rotation: &'static mut Rotation,
}

/*  ---------------------

    Filters

*/

#[derive(QueryFilter)]
pub struct FPartAdd {
    _c: Added<Part>,
}

#[derive(QueryFilter)]
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

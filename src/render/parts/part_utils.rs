use crate::{
    ecs::parts::Part,
    render::spatial_map::{SpatialCell, SpatialKey},
};

/// Return Brick, Wedge, Ball vectors where it returns flattened cells
///     Note that it performs a copy on the data for it to be contiguous
///
/// Panics if meshes are used
pub fn split_and_flatten_cells<'a, const SIZE: usize, U: Copy>(
    keys_and_cells: Vec<(&SpatialKey<SIZE>, &'a SpatialCell<U>)>,
) -> (Vec<U>, Vec<U>, Vec<U>) {
    let mut bricks = Vec::new();
    let mut wedges = Vec::new();
    let mut balls = Vec::new();

    for (k, c) in keys_and_cells {
        match k.part {
            Part::Brick => bricks.extend(&c.buffers),
            Part::Wedge => wedges.extend(&c.buffers),
            Part::Ball => balls.extend(&c.buffers),
            _ => panic!("Part type not implemented for partition_cells"),
        }
    }
    (bricks, wedges, balls)
}

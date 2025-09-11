use crate::{common::types::HasPosition, ecs::common::Size};
use bevy_platform::collections::HashSet;
use rapier3d::prelude::*;

pub fn reduce_to_scaled_hull<T: HasPosition>(vertices: &[T], size: &Size) -> Vec<Point<Real>> {
    let mut set = HashSet::new();

    for vertex in vertices {
        let mut pos = *vertex.get_position();
        pos[0] *= size.0.x;
        pos[1] *= size.0.y;
        pos[2] *= size.0.z;

        set.insert([pos[0].to_bits(), pos[1].to_bits(), pos[2].to_bits()]);
    }

    set.iter()
        .map(|u_vertex| {
            Point::new(
                f32::from_bits(u_vertex[0]),
                f32::from_bits(u_vertex[1]),
                f32::from_bits(u_vertex[2]),
            )
        })
        .collect()
}

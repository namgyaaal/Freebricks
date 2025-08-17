use std::collections::HashSet;

use bevy_ecs::prelude::*;
use petgraph::{matrix_graph::{UnMatrix}, visit::{Bfs, NodeIndexable}};
use crate::components::{bricks::*, common::{Position, Rotation, Size}, model::*};


/// Function for seeing if bricks snap together 
/// 
/// TODO: CHECK STUDS
fn touch_check(pos_a: &Position, size_a: &Size, _brick_a: &Brick, 
               pos_b: &Position, size_b: &Size, _brick_b: &Brick) -> bool {

    let a_min = pos_a.0 - (size_a.0 / 2.0);
    let a_max = pos_a.0 + (size_a.0 / 2.0);

    let b_min = pos_b.0 - (size_b.0 / 2.0);
    let b_max = pos_b.0 + (size_b.0 / 2.0);


    let touch = 
        a_min.x <= b_max.x && 
        a_max.x >= b_min.x && 
        a_min.y <= b_max.y && 
        a_max.y >= b_min.y && 
        a_min.z <= b_max.z && 
        a_max.z >= b_min.z;

    if !touch {
        return false;
    }


    return true;
}

/// Given a world with bricks, subdivide into owned and not owned and insert models 
pub fn build_models(world: &mut World) {
    let mut bricks = world.query::<(Entity, &Position, &Rotation, &Size, &Brick)>();
    
    let mut aabb_bricks = Vec::new();

    for (entity, position, rotation, size, brick) in bricks.iter(&world) {
        /*
            It would be an enhancement to allow for rotated bricks, I think it would just require some clever mathematics 
                (finding centroids and doing reverse transformations+rotations) to figure if they snap together 

            For now we don't handle this 
         */
        if !rotation.is_near_identity() {
            continue 
        }
        aabb_bricks.push(
            (entity, position, size, brick)
        )
    }   
    /*
    
        Seems like this only supports 2^16 nodes for now. 
     */
    let mut graph: UnMatrix<Entity, ()> = UnMatrix::with_capacity(aabb_bricks.len());
    let nodes: Vec<_> = aabb_bricks
        .iter() 
        .map(|x| graph.add_node(x.0))
        .collect();


    let mut dirty = vec![false; graph.node_count()];


    // O(n^2) for now, could use BVH down the line. 
    for i in 0..aabb_bricks.len() {
        for j in 0..aabb_bricks.len() {
            if i == j {
                continue 
            }
            let brick_a = aabb_bricks.get(i).unwrap();
            let brick_b = aabb_bricks.get(j).unwrap(); 
            let node_a = nodes.get(i).unwrap(); 
            let node_b = nodes.get(j).unwrap();

            if graph.has_edge(*node_a, *node_b) {
                continue 
            }

            let check = touch_check(
                brick_a.1, brick_a.2, brick_a.3, 
                brick_b.1, brick_b.2, brick_b.3
            );

            if check {
                graph.add_edge(*node_a, *node_b, ());
            }
            
        }
    }


    /*
        Traverse graphs and collect components 
     */

    let mut collections = Vec::new();
    for i in 0..graph.node_count() {
        // If already added to component, continue 
        if *dirty.get(i).unwrap() == true {
            continue 
        }

        let start_node = graph.from_index(i);

        // TODO: Graph format instead 

        for (a, b, ()) in graph.edges(start_node) {
            let _other_node = {
                if start_node == b {
                    a
                } else {
                    b
                }
            };




        }


        let mut collection = HashSet::new();
        collection.insert(graph[start_node]);

        let mut bfs= Bfs::new(&graph, start_node);
        while let Some(node) = bfs.next(&graph) {
            let i = node.index(); 
            if let Some(x) = dirty.get_mut(i) {
                *x = true;

                collection.insert(graph[node]);
            }
        }

        collections.push(collection);
    }

    // Given vector of hashsets: 
    // If len() == 1, Brick is both rigid and collision 
    // If len() >= 2, Brick needs to be under a model 

    for collection in collections {
        if collection.len() > 1 {
            for brick in &collection {
                world.entity_mut(*brick) 
                    .insert(Owned {});
            }

            let entities : Vec<Entity> = collection.clone().into_iter().collect();

            world.spawn((
                Model { graph: graph.clone() },
            )).add_children(&entities);
        
        }   
    }

}
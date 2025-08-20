use std::collections::{HashMap, HashSet, VecDeque};

use bevy_ecs::{prelude::*};
use petgraph::{graph::UnGraph, matrix_graph::{MatrixGraph, UnMatrix}, prelude::{StableUnGraph, UnGraphMap}, visit::{Bfs, NodeIndexable}};
use crate::{components::{bricks::*, common::{Position, Rotation, Size}, model::*, physics::{AnchorSource, AnchoredTo, Physical}}, physics::PhysicsState};


/// Function for seeing if bricks snap together 
/// 
/// TODO: CHECK STUDS
fn touch_check(pos_a: &Position, size_a: &Size, brick_a: &Brick, 
               pos_b: &Position, size_b: &Size, brick_b: &Brick) -> bool {

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


    // Previous checks collision, this more directly checks if they actually "snap" together. 
    // lol this is a mess 

    let a_b_snap = (f32::abs(a_min.y - b_max.y) < f32::EPSILON) && 
       ((brick_b.bottom == StudType::Inlet && brick_a.top == StudType::Outlet) || 
       (brick_b.bottom == StudType::Outlet && brick_a.top == StudType::Inlet));


    let b_a_snap = (f32::abs(a_max.y - b_min.y) < f32::EPSILON) &&
       ((brick_a.bottom == StudType::Inlet && brick_b.top == StudType::Outlet) || 
       (brick_a.bottom == StudType::Outlet && brick_b.top == StudType::Inlet));
    // Need some check if studs actually align 

    return a_b_snap || b_a_snap;
}

/// Given a world with bricks, subdivide into owned and not owned and insert models 
pub fn build_models(world: &mut World) {

    let mut bricks = world.query::<(Entity, &Position, &Rotation, &Size, &Brick, &Physical)>();
    let mut brick_info= Vec::new();

    for (entity, position, rotation, size, brick, physical) in bricks.iter(&world) {
        /*
            It would be an enhancement to allow for rotated bricks, I think it would just require some clever mathematics 
                (finding centroids and doing reverse transformations+rotations) to figure if they snap together 

            For now we don't handle this, I am sure this will come back to screw me up in the future. 
         */
        if !rotation.is_near_identity() {
            continue 
        }
        brick_info.push(
            (entity, position, size, brick, physical)
        )
    }   
    /*
    
        Seems like this only supports 2^16 nodes for now. 

        bool represents anchored edge
     */
    let mut graph: UnMatrix<Entity, bool> = UnMatrix::with_capacity(brick_info.len());
    let nodes: Vec<_> = brick_info 
        .iter() 
        .map(|x| graph.add_node(x.0))
        .collect();


    // This goes across entire scene and connects edges where bricks snap together 
    // O(n^2) for now, could use BVH down the line. 
    for i in 0..brick_info.len() {
        for j in 0..brick_info.len() {
            if i == j {
                continue 
            }   

            let brick_a = brick_info.get(i).unwrap();
            let brick_b = brick_info.get(j).unwrap(); 
            let node_a = nodes.get(i).unwrap(); 
            let node_b = nodes.get(j).unwrap();

            if graph.has_edge(*node_a, *node_b) {
                continue 
            }

            let check = touch_check(
                brick_a.1, brick_a.2, brick_a.3, 
                brick_b.1, brick_b.2, brick_b.3
            );

            if check && !(brick_a.4.anchored && brick_b.4.anchored) {
                graph.add_edge(*node_a, *node_b, brick_a.4.anchored || brick_b.4.anchored);
            }
            
        }
    }
    /*
        Traverse graphs and collect components.

        This is most complex part of build_models
     */

    // Have we visited before? 
    // Note: it needs to handle anchored bricks differently (because many-to-anchored relation)
    let mut dirty = vec![false; graph.node_count()];
    let mut collections = Vec::new();
    for i in 0..graph.node_count() {
        // If already added to component, continue 
        // anchored nodes are never dirtied, but are never used to iterate through graphs 

        if *dirty.get(i).unwrap() == true || brick_info.get(i).unwrap().4.anchored {
            continue 
        }

        let start_node = graph.from_index(i);


        let mut brick_set = HashSet::new();
        // New graph time and anchored bricks!
        let mut subgraph: UnGraphMap<Entity, ()> = UnGraphMap::new();
        // <K, [E]> s.t. K is anchored and not in component_graph
        // saved for when we make relationship between them 
        let mut anchor_map: HashMap<Entity, HashSet<Entity>> = HashMap::new();

        let mut queue = VecDeque::from([start_node]);
        // Graph iteration 
        while let Some(node) = queue.pop_front() {
            for (a, b, &anchored) in graph.edges(node) {
                let a_info = brick_info.get(a.index()).unwrap(); 
                let b_info = brick_info.get(b.index()).unwrap();
                
                *dirty.get_mut(a.index()).unwrap() = true;

                // If previous traversals didn't add: add them 
                if !subgraph.contains_node(a_info.0) {
                    subgraph.add_node(a_info.0);   
                    brick_set.insert(a_info.0);
                }
                if !subgraph.contains_node(b_info.0) && !anchored {
                    subgraph.add_node(b_info.0);
                    brick_set.insert(b_info.0);
                    queue.push_back(b);
                }

                if anchored {
                    // We don't add anchored bricks to the graph but instead keep track of them in a set
                    // nor are they considered in further traversals, we STOP at them and don't go further 
                    let set = anchor_map
                        .entry(b_info.0)
                        .or_insert(HashSet::new());
                    set.insert(a_info.0);

                } else {
                    // Handle non-anchored bricks
                    _ = subgraph.add_edge(a_info.0, b_info.0, ()); 
                }
            }
        }

        collections.push((brick_set, subgraph, anchor_map));
    }

    /*
        Given subgraphs, build them 
     */


    // for anchored bricks with bricks attached we save entities attached and add the components after doing everything 
    let mut anchor_sources: HashMap<Entity, HashSet<Entity>> = HashMap::new();

    for (set, graph, anchors) in collections {
        /*
            Handle solitary bricks 
         */
        if set.len() == 1 { // Brick is independent 
            let &e = set.iter().next().unwrap();
            if anchors.len() > 0 {
                // Handle anchored 
                let sources: HashSet<Entity> = anchors
                    .keys()
                    .cloned()
                    .collect();
                // Given part, given it anchor sources 
                world
                    .entity_mut(e)
                    .insert(AnchoredTo(sources.clone()));
            }
        /*
            Handle bricks under a model 
         */
        } else { // Brick is under a model
            let entities: Vec<Entity> = set
                .clone()
                .into_iter()
                .collect();

            // Collect our entities that are anchored. 
            let anchored: HashSet<Entity> = anchors
                .values() 
                .flat_map(|set| set.iter())
                .cloned()
                .collect();

            // For each entity connected to an anchored brick
            for &entity in &entities {
                let connected_anchors : HashSet<Entity> = anchors 
                    .iter()
                    .filter_map(|(&e, v)| {
                        if v.contains(&entity) {
                            Some(e)
                        } else {
                            None
                        }
                    })
                    .collect();
                
                if connected_anchors.len() == 0 {
                    continue 
                }

                world 
                    .entity_mut(entity)
                    .insert(AnchoredTo(connected_anchors));
            }


            world.spawn(
                Model {
                    set: set, 
                    graph: graph, 
                    anchored: anchored
            }).add_children(&entities);
        }
        // defer AnchorOf for later 
        for (anchor, set) in anchors {
            anchor_sources
                .entry(anchor)
                .or_insert(HashSet::new())
                .extend(set);
        }
    }

    println!("{:?} {}", anchor_sources, anchor_sources.len());


    for (&anchor, _) in &anchor_sources {
        world 
            .entity_mut(anchor)
            .insert(AnchorSource);
    }

    // Save anchor sources 
    world
        .get_resource_mut::<PhysicsState>()    
        .expect("Model Builder can't find physics state")
        .anchor_sources = anchor_sources;
}
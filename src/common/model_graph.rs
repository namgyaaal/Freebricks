use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    ecs::{
        common::{Position, Rotation, Size},
        model::*,
        parts::*,
        physics::{AnchorSource, AnchoredTo, Physical},
    },
    physics::PhysicsState,
};
use bevy_ecs::prelude::*;
use petgraph::{matrix_graph::UnMatrix, prelude::UnGraphMap, visit::NodeIndexable};

/// Function for seeing if bricks snap together
///
/// TODO: CHECK STUDS
fn touch_check(
    pos_a: &Position,
    size_a: &Size,
    part_a: &StudInfo,
    pos_b: &Position,
    size_b: &Size,
    part_b: &StudInfo,
) -> bool {
    let a_min = pos_a.0 - (size_a.0 / 2.0);
    let a_max = pos_a.0 + (size_a.0 / 2.0);

    let b_min = pos_b.0 - (size_b.0 / 2.0);
    let b_max = pos_b.0 + (size_b.0 / 2.0);

    let touch = a_min.x <= b_max.x
        && a_max.x >= b_min.x
        && a_min.y <= b_max.y
        && a_max.y >= b_min.y
        && a_min.z <= b_max.z
        && a_max.z >= b_min.z;

    if !touch {
        return false;
    }

    // Previous checks collision, this more directly checks if they actually "snap" together.
    // lol this is a mess

    let a_b_snap = (f32::abs(a_min.y - b_max.y) < f32::EPSILON)
        && ((part_b.bottom == StudType::Inlet && part_a.top == StudType::Outlet)
            || (part_b.bottom == StudType::Outlet && part_a.top == StudType::Inlet));

    let b_a_snap = (f32::abs(a_max.y - b_min.y) < f32::EPSILON)
        && ((part_a.bottom == StudType::Inlet && part_b.top == StudType::Outlet)
            || (part_a.bottom == StudType::Outlet && part_b.top == StudType::Inlet));
    // Need some check if studs actually align

    return a_b_snap || b_a_snap;
}

/// Given a world with bricks, subdivide into owned and not owned and insert models
pub fn build_models(
    mut commands: Commands,
    mut state: ResMut<PhysicsState>,
    parts: Query<QPartWorldInit>,
) {
    let part_info: Vec<_> = parts
        .iter()
        .filter_map(|part| {
            /*
               Would be an enhancement to allow for rotated bricks, but for now don't handle
               Just requires clever mathematics (getting centroids and reverse rotation/translating)
            */
            if !part.rotation.is_near_identity() {
                return None;
            } else {
                return Some((
                    part.entity,
                    part.position,
                    part.size,
                    part.studs,
                    part.physical,
                ));
            }
        })
        .collect();

    /*

       Seems like this only supports 2^16 nodes for now. OK

       bool represents anchored edge
    */
    let mut graph: UnMatrix<Entity, bool> = UnMatrix::with_capacity(part_info.len());
    let nodes: Vec<_> = part_info.iter().map(|x| graph.add_node(x.0)).collect();

    // This goes across entire scene and connects edges where bricks snap together
    // O(n^2) for now, could use BVH down the line.

    for i in 0..part_info.len() {
        for j in (0..i).chain((i + 1)..part_info.len()) {
            let part_a = part_info.get(i).unwrap();
            let part_b = part_info.get(j).unwrap();
            let node_a = nodes.get(i).unwrap();
            let node_b = nodes.get(j).unwrap();

            if graph.has_edge(*node_a, *node_b) {
                continue;
            }
            let check = touch_check(part_a.1, part_a.2, part_a.3, part_b.1, part_b.2, part_b.3);
            // We don't add edges to anchor<->anchor because they don't make models !
            if check && !(part_a.4.anchored && part_b.4.anchored) {
                graph.add_edge(*node_a, *node_b, part_a.4.anchored || part_b.4.anchored);
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
        // anchored nodes are never dirtied, but are also never used to iterate through graphs
        if *dirty.get(i).unwrap() == true || part_info.get(i).unwrap().4.anchored {
            continue;
        }

        let start_node = graph.from_index(i);

        let mut part_set = HashSet::new();
        // New graph time and anchored bricks!
        let mut subgraph: UnGraphMap<Entity, ()> = UnGraphMap::new();
        // <K, [E]> s.t. K is anchored and not in component_graph
        // saved for when we make relationship between them
        let mut anchor_map: HashMap<Entity, HashSet<Entity>> = HashMap::new();

        let mut queue = VecDeque::from([start_node]);
        // Graph traversal
        while let Some(node) = queue.pop_front() {
            for (a, b, &anchored) in graph.edges(node) {
                let a_info = part_info.get(a.index()).unwrap();
                let b_info = part_info.get(b.index()).unwrap();

                *dirty.get_mut(a.index()).unwrap() = true;

                // If previous traversals didn't add: add them
                if !subgraph.contains_node(a_info.0) {
                    subgraph.add_node(a_info.0);
                    part_set.insert(a_info.0);
                }
                if !subgraph.contains_node(b_info.0) && !anchored {
                    subgraph.add_node(b_info.0);
                    part_set.insert(b_info.0);
                    queue.push_back(b);
                }

                if anchored {
                    // If we have an anchored edge, with key=anchor add our models node
                    let set = anchor_map.entry(b_info.0).or_insert(HashSet::new());
                    set.insert(a_info.0);
                } else {
                    // Handle non-anchored bricks
                    subgraph.add_edge(a_info.0, b_info.0, ());
                }
            }
        }
        collections.push((part_set, subgraph, anchor_map));
    }

    // for anchored bricks with bricks attached we save entities attached and add the components after doing everything
    let mut anchor_sources: HashMap<Entity, HashSet<Entity>> = HashMap::new();

    // Stage where subgraphs are built after we've generated them through traversing the graph
    for (set, graph, anchors) in collections {
        /*
           Handle bricks under no model

           We only need to handle if it's attached to anchors
        */
        if set.len() == 1 && anchors.len() > 0 {
            let &entity = set.iter().next().unwrap();

            // Handle anchored
            let sources: HashSet<Entity> = anchors.keys().cloned().collect();
            // Given part, given it anchor sources if it's connected to an anchor
            commands.entity(entity).insert(AnchoredTo(sources.clone()));
        /*
           Handle bricks under a model
        */
        } else {
            let entities: Vec<Entity> = set.clone().into_iter().collect();

            // Collect our entities that attached to anchors
            let anchored: HashSet<Entity> = anchors
                .values()
                .flat_map(|set| set.iter())
                .cloned()
                .collect();

            for &part in &anchored {
                let connected_anchors: HashSet<Entity> = anchors
                    .iter()
                    .filter_map(|(&e, v)| if v.contains(&part) { Some(e) } else { None })
                    .collect();

                commands.entity(part).insert(AnchoredTo(connected_anchors));
            }

            commands
                .spawn(Model {
                    set: set,
                    graph: graph,
                    anchored: anchored,
                })
                .add_children(&entities);
        }
        // defer AnchorOf for later
        for (anchor, set) in anchors {
            anchor_sources
                .entry(anchor)
                .or_insert(HashSet::new())
                .extend(set);
        }
    }

    for (&anchor, _) in &anchor_sources {
        commands.entity(anchor).try_insert(AnchorSource);
    }
    // Save anchor sources
    state.anchor_sources = anchor_sources;
}

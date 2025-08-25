use petgraph::visit::{Bfs, IntoNeighbors, IntoNodeIdentifiers, NodeCount, Visitable};

pub fn is_connected<G>(graph: G) -> bool
where
    G: Visitable + IntoNeighbors + IntoNodeIdentifiers + NodeCount,
{
    if graph.node_count() == 0 {
        return true;
    }

    let start = match graph.node_identifiers().next() {
        Some(node) => node,
        None => return true,
    };

    let mut bfs = Bfs::new(graph, start);
    let mut count = 0;
    while bfs.next(graph).is_some() {
        count += 1;
    }

    count == graph.node_count()
}

use std::{
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
};

use rayon::iter::{IndexedParallelIterator, ParallelIterator};

use petgraph::{Direction::Incoming, graph::NodeIndex};

use crate::{LinkGraph, NodeRef, analysis::algo::ShortestPaths};

type Graph<N = ()> = petgraph::graph::DiGraph<N, ()>;

pub struct AnalysisGraph {
    graph: Graph,
    shortest_paths: ShortestPaths,
    cc: algo::ConnectedComponents,
    scc: algo::Scc,
}

impl fmt::Debug for AnalysisGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnalysisGraph").finish_non_exhaustive()
    }
}

impl Deref for AnalysisGraph {
    type Target = Graph;
    fn deref(&self) -> &Graph {
        &self.graph
    }
}

impl DerefMut for AnalysisGraph {
    fn deref_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }
}

pub mod algo {
    use std::collections::HashMap;

    pub use petgraph::algo::*;

    use petgraph::{graph::NodeIndex, unionfind::UnionFind, visit::EdgeRef};
    use serde::{Deserialize, Serialize};

    use crate::NodeRef;

    use super::Graph;

    #[derive(Clone, Serialize, Deserialize, Default)]
    #[serde(transparent)]
    pub struct ShortestPaths {
        square: Vec<Vec<usize>>,
    }

    impl ShortestPaths {
        #[tracing::instrument(level = "info", skip(g))]
        pub fn new(g: &Graph) -> ShortestPaths {
            let n = g.node_count();
            let mut square = Vec::with_capacity(n * n);
            for i in 0..n {
                let run = dijkstra(&g, NodeIndex::new(i), None, |_| 1usize);
                if run.len() <= 1 {
                    square.push(Vec::new());
                    continue;
                }
                let mut row = Vec::with_capacity(n);
                for j in 0..n {
                    row.push(run.get(&NodeIndex::new(j)).copied().unwrap_or(0));
                }
                square.push(row);
            }

            ShortestPaths { square }
        }

        pub fn lenths_from(&self, a: NodeRef) -> &[usize] {
            &self.square[usize::from(a)]
        }

        pub fn length(&self, a: NodeRef, b: NodeRef) -> Option<usize> {
            if a == b {
                return Some(0);
            }

            self.lenths_from(a).get(usize::from(b)).copied()
        }

        pub fn into_square(self) -> Vec<Vec<usize>> {
            self.square
        }
    }

    pub struct ConnectedComponents {
        labelling: Vec<NodeIndex>,
        component_indices: Vec<NodeIndex>,
        components: HashMap<NodeIndex, Graph<NodeIndex>>,
    }

    impl ConnectedComponents {
        #[tracing::instrument(level = "info", skip(g))]
        pub fn new(g: &Graph) -> ConnectedComponents {
            let mut component_sets = UnionFind::new(g.node_count());
            for edge in g.edge_references() {
                component_sets.union(edge.source(), edge.target());
            }

            let labelling = component_sets.into_labeling();
            let mut component_indices = Vec::with_capacity(labelling.len());

            let mut components = HashMap::new();

            for i in 0..g.node_count() {
                let i = NodeIndex::new(i);
                component_indices.push(
                    components
                        .entry(labelling[i.index()])
                        .or_insert_with(|| Graph::new())
                        .add_node(i),
                );
            }

            for edge in g.edge_references() {
                let i = labelling[edge.source().index()];
                let a = component_indices[edge.source().index()];
                let b = component_indices[edge.target().index()];
                components.get_mut(&i).unwrap().add_edge(a, b, ());
            }

            let this = ConnectedComponents {
                components,
                labelling,
                component_indices,
            };

            for i in 0..g.node_count() {
                let i = NodeIndex::new(i);
                let (graph, node) = this.find(i);
                assert_eq!(graph[node], i);
            }

            this
        }

        pub fn find(&self, node: NodeIndex) -> (&Graph<NodeIndex>, NodeIndex) {
            (
                &self.components[&self.labelling[node.index()]],
                self.component_indices[node.index()],
            )
        }
    }

    pub struct Scc {
        components: Vec<Graph<NodeIndex>>,
        binding: Vec<(usize, NodeIndex)>,
    }

    impl Scc {
        #[tracing::instrument(level = "info", skip(g))]
        pub fn new(g: &Graph) -> Scc {
            let scc = kosaraju_scc(g);
            let mut binding = vec![(0, NodeIndex::new(0)); g.node_count()];

            for (i, com) in scc.iter().enumerate() {
                for (j, node) in com.iter().enumerate() {
                    binding[node.index()] = (i, NodeIndex::new(j));
                }
            }

            let mut components: Vec<Graph<NodeIndex>> = scc
                .into_iter()
                .map(|com| {
                    let mut subg = Graph::new();
                    for node in com {
                        subg.add_node(node);
                    }

                    subg
                })
                .collect();

            for edge in g.edge_references() {
                let (ac, a) = binding[edge.source().index()];
                let (bc, b) = binding[edge.target().index()];
                if ac == bc {
                    // both part of the same SCC so add edge:
                    components[ac].add_edge(a, b, ());
                }
            }

            let this = Scc {
                components,
                binding,
            };

            for i in 0..g.node_count() {
                let i = NodeIndex::new(i);
                let (graph, node) = this.find(i);
                assert_eq!(graph[node], i);
            }

            this
        }

        pub fn are_shared(&self, a: NodeIndex, b: NodeIndex) -> bool {
            self.binding[a.index()].0 == self.binding[b.index()].0
        }

        pub fn find(&self, node: NodeIndex) -> (&Graph<NodeIndex>, NodeIndex) {
            let (c, i) = self.binding[node.index()];
            (&self.components[c], i)
        }

        pub fn binding(&self) -> impl DoubleEndedIterator<Item = usize> {
            self.binding.iter().map(|&(c, _)| c)
        }
    }
}

impl AnalysisGraph {
    #[tracing::instrument(level = "info")]
    pub fn new(links: &LinkGraph) -> AnalysisGraph {
        let n = links.nodes.len();
        let mut graph = Graph::with_capacity(n, links.edges.len());
        for i in 0..n {
            assert_eq!(i, graph.add_node(()).index());
        }
        for &(a, b) in &links.edges {
            graph.update_edge(a.index(), b.index(), ());
        }

        AnalysisGraph {
            cc: algo::ConnectedComponents::new(&graph),
            scc: algo::Scc::new(&graph),
            shortest_paths: ShortestPaths::new(&graph),
            graph,
        }
    }

    fn count_shortest_paths(&self, a: NodeRef, b: NodeRef, mut f: impl FnMut(NodeRef)) -> usize {
        let Some(n) = self.shortest_paths.length(a, b) else {
            return 0;
        };

        let mut stack: HashMap<NodeRef, usize> = [(b, 1)].into_iter().collect();
        for n in (0..n).rev() {
            let mut stack2 = HashMap::new();
            for (i, count) in stack.into_iter().flat_map(|(i, count)| {
                self.neighbors_directed(i.index(), Incoming)
                    .filter(|&i| self.scc.are_shared(a.index(), i))
                    .map(|i| NodeRef::new(i.index()))
                    .filter(|&i| self.shortest_paths.length(a, i).unwrap() == n)
                    .map(move |i| (i, count))
            }) {
                f(i);
                *stack2.entry(i).or_default() += count
            }
            stack = stack2;
        }

        assert_eq!(stack.len(), 1, "{stack:?}");

        stack.into_values().next().unwrap()
    }

    fn betweenness_centrality(&self, node: NodeIndex) -> f32 {
        let (g, _) = self.scc.find(node);
        let mut total = 0.;
        for a in g.node_indices() {
            for b in g.node_indices() {
                let mut v = 0;
                let n = self.count_shortest_paths(g[a].into(), g[b].into(), |a| {
                    v += (a.index() == node) as usize
                });
                total += v as f32 / n as f32;
            }
        }

        let divisor = (self.node_count() - 1) as f32 * (self.node_count() - 2) as f32;
        total / divisor
    }

    fn harmonic_centrality(&self, node: NodeIndex) -> f32 {
        let (component, node2) = self.scc.find(node);
        let mut geodesics = algo::dijkstra(component, node2, None, |_| 1.);
        geodesics.remove(&node2);
        let centrality: f32 = geodesics.values().map(|&l| 1. / l).sum();
        centrality * (component.node_count() - 1) as f32
    }

    #[tracing::instrument(level = "info")]
    pub fn calculate_stats(&mut self, graph: &mut LinkGraph) {
        // let shortest_pairs = petgraph::algo::johnson::parallel_johnson(&**self, |_| 0.).unwrap();
        {
            let span = tracing::info_span!("page rank");
            let _g = span.enter();
            for (i, &rank) in algo::page_rank(&**self, 0.99, 5).iter().enumerate() {
                graph.nodes[i].stats.page_rank = rank;
            }
        }

        {
            let span = tracing::info_span!("harmonic centrality");
            let _g = span.enter();
            graph
                .nodes
                .par_values_mut()
                .enumerate()
                .for_each(|(i, node)| {
                    node.stats.harmonic_centrality = self.harmonic_centrality(NodeIndex::new(i));
                });
        }

        {
            let span = tracing::info_span!("betweenness centrality");
            let _g = span.enter();
            graph
                .nodes
                .par_values_mut()
                .enumerate()
                .for_each(|(i, node)| {
                    node.stats.betweenness_centrality =
                        self.betweenness_centrality(NodeIndex::new(i));
                });
        }
    }

    pub fn export_shortest_paths(&self) -> Vec<Vec<usize>> {
        self.shortest_paths.clone().into_square()
    }

    pub fn export_scc(&self) -> Vec<usize> {
        self.scc.binding().collect()
    }
}

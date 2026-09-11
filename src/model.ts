import type Graph from "graphology";

export type Manifest = LinkGraph & { time: string };

export interface LinkGraph {
    nodes: [string, LinkNode][],
    edges: [number, number][],
    shortest_paths: number[][],
    scc: number[],
}

export type LinkNode = LinkObject & LinkNodeInfo;

export interface WikiPage {
    words: number,
}

export type LinkObject = {
    external: null,
} | {
    file: { wiki_page: WikiPage } | { mystery: { found: boolean } }
};

export interface LinkNodeInfo {
    harmonic_centrality: number,
    page_rank: number,
}

export function sameScc(g: LinkGraph, a: string, b: string): boolean {
    return g.scc[parseInt(a)] === g.scc[parseInt(b)];
}

export function shortestPath(g: LinkGraph, a: string, b: string): number | null {
    if (a == b) return 0;
    const l = g.shortest_paths[parseInt(a)]?.[parseInt(b)];
    if (l === 0) return null;
    return l;
}

export function degreesOfSeparation(g: LinkGraph, target: string, a: string | null, b: string | null): number | null {
    if (target == a && target == b) return 0;
    const succ = a ? shortestPath(g, target, a) : Infinity;
    const pred = b ? shortestPath(g, b, target) : Infinity;
    const t = Math.min(pred ?? Infinity, succ ?? Infinity);
    return t !== Infinity ? t : null;
}

function toNode(node_: LinkNode) {
    const node = node_ as any;
    // const evil = "rgb(140, 140, 255)";
    const style = Object.hasOwn(node, "external")
        ? { color: "rgb(200, 200, 100)" }
        : node.file.hasOwnProperty("wiki_page")
            ? { color: "rgb(100, 100, 255)" }
            : { color: node.file.mystery.found ? "rgb(140, 255, 140)" : "rgb(255, 140, 140)" };
    return {
        ...style,
        harmonic_centrality: node.harmonic_centrality,
        betweenness_centrality: node.betweenness_centrality,
        page_rank: node.page_rank,
    };
}

const EXCLUDE = ["index"];
export function writeToGraph(links: LinkGraph, graph: Graph, includeExternal?: boolean) {
    graph.clear();

    for (let i = 0; i < links.nodes.length; i++) {
        const [key, node] = links.nodes[i];
        const label = key;
        if (EXCLUDE.includes(label)) continue;

        if (!includeExternal && Object.hasOwn(node, "external")) continue;

        graph.addNode(i, { i, x: 0, y: 0, label, ...toNode(node) });
    }

    for (const [a, b] of links.edges) {
        if (!graph.hasNode(a) || !graph.hasNode(b)) continue;

        graph.addDirectedEdge(a, b, { a, b });
    }

    for (let i = 0; i < links.nodes.length; i++) {
        if (!graph.hasNode(i)) continue;
        graph.updateNode(i, attrs => ({ ...attrs, importance: 5 + 2 * Math.sqrt(links.nodes[i][1].harmonic_centrality / 200) }));
    }
}

import Graph from "graphology";
import circular from "graphology-layout/circular";
import Sigma, { DEFAULT_STYLES } from "sigma";
import { extremityArrow, pathCurved, pathLine } from "sigma/rendering";
import { attachSettings, settings } from "./settings";
import { degreesOfSeparation, LinkGraph, shortestPath, writeToGraph } from "./model";
import LayoutSupervisor from "graphology-layout-physics/worker";

export type Renderer = ReturnType<typeof attachRenderer>;

export function attachRenderer(links: LinkGraph, graph: Graph, container: HTMLElement) {
    function settingsAwareBacon(selected: string, a: string, b: string) {
        if (a == b && a == selected) return 0;
        const n = settings.showSuccessors ? a : null;
        const m = settings.showPredecessors ? b : null;
        return degreesOfSeparation(links, selected, n, m);
    }

    function opacityFromDegrees(degree: number | null, min: number) {
        if (degree === null) return min;
        return min + Math.max(0, (1 - min) * (1 - Math.max(0, degree) * settings.highlightFalloff));
    }

    const renderer = new Sigma(graph, container, {
        customGraphState: {
            frozen: false,
            selected: null as string | null,
        },
        primitives: {
            edges: {
                paths: [pathLine(), pathCurved()],
                extremities: [extremityArrow()],
            }
        },
        styles: {
            edges: [
                DEFAULT_STYLES.edges,
                {
                    head: "arrow",
                    color: "#ddd",
                    path: "straight",
                    parallelPath: "curved",
                    parallelSpread: 0.05,
                    selfLoopPath: "loop",
                    opacity: 0.5,
                },
                {
                    when: (_attrs, _state, graph) => !!graph.selected,
                    then: {
                        opacity: (attrs, _b, graph) => {
                            const d = settingsAwareBacon(graph.selected!, attrs.a, attrs.b);
                            return 0.5 * opacityFromDegrees(d, 0.1);
                        }
                    }
                }
            ],
            nodes: [
                DEFAULT_STYLES.nodes,
                {
                    size: { attribute: "importance" },
                    color: { attribute: "color" },
                    labelColor: "#fff",
                    backdropColor: "#444",
                    labelBackgroundColor: "#444",
                },
                {
                    when: (_attrs, _state, graph) => !!graph.selected,
                    then: {
                        opacity: (attrs, _b, graph) => {
                            const d = settingsAwareBacon(graph.selected!, attrs.i, attrs.i);
                            return opacityFromDegrees(d !== null ? d - 0.5 : null, 0.2);
                        }
                    }
                }
            ],
        },
        settings: {
            itemSizesReference: "positions",
            autoRescale: true,
        }
    });

    let layout: null | LayoutSupervisor = null;
    const relayout = () => {
        if (layout) {
            layout.stop();
            layout.kill();
        }

        layout = new LayoutSupervisor(graph, {
            // settings: { gravity: 40, barnesHutOptimize: true, adjustSizes: true, linLogMode: true, slowDown: 4, outboundAttractionDistribution: true },
            settings: { damping: 0.4, timestep: 0.2, gravity: 0.015 },
            // iterations: 500,
        });
        layout.start();
    };

    renderer.rawEmitter.on("reflow", () => {
        writeToGraph(links, graph, settings.includeExternal);
        circular.assign(graph, { scale: 1 });
        relayout();
    });

    return renderer;
}

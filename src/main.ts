import Graph from "graphology";
import Sigma, { DEFAULT_STYLES } from "sigma";
import circular from "graphology-layout/circular";
import { shortestPath, writeToGraph, type Manifest } from "./model";
import { attachRenderer } from "./render";
import { attachSettings } from "./settings";

import graphUrl from "../graph.json?url";

let last = Date.now();
let timeoutFrame: null | number = null;

const load = new Promise((res) => window.onload = () => res(undefined));
const loadManifest = fetch(graphUrl).then((res) => res.json());
const [_, raw]: [unknown, Manifest] = await Promise.all([load, loadManifest]) as any;

const container = document.getElementById("sigma-container")! as HTMLElement;
const details = document.getElementById("details")! as HTMLElement;

const graph = new Graph({ multi: true, allowSelfLoops: true });
const renderer = attachRenderer(raw, graph, container);
attachSettings(renderer);

renderer.addListener("enterNode", ({ node }) => {
    const state = renderer.getGraphState();
    if (state.frozen) return;
    renderer.setGraphState({ selected: node });
});
renderer.addListener("leaveNode", ({ node }) => {
    const state = renderer.getGraphState();
    if (state.frozen) return;
    if (state.selected === node) renderer.setGraphState({ selected: null });
});
renderer.addListener("clickStage", () => selectNode(null));
renderer.rawEmitter.on("reflow", () => selectNode(null));

let ctrl = false;
window.addEventListener("keydown", ev => ctrl = ev.ctrlKey);
window.addEventListener("keyup", ev => ctrl = ev.ctrlKey);
window.addEventListener("focus", _ => ctrl = false);

window.addEventListener("keydown", ev => {
    const state = renderer.getGraphState();
    if (ev.key !== "ArrowLeft" && ev.key !== "ArrowRight") return;

    const dir = ev.key === "ArrowLeft" ? -1 : 1;
    let node = state.selected!;

    const nodes = graph.nodes();
    const i = nodes.findIndex((el) => el == node);
    if (dir === -1) {
        if (i <= 0) selectNode(nodes[nodes.length - 1]);
        else selectNode(nodes[i - 1]);
    } else {
        if (i >= nodes.length) selectNode(nodes[0]);
        else selectNode(nodes[i + 1]);
    }
});

renderer.addListener("clickNode", ({ node }) => {
    if (ctrl) return window.open(urlFor(node), "_blank")!.focus();

    selectNode(node);
});

const urlFor = (node: string) => `https://wiki.bedlamtheatre.co.uk/${graph.getNodeAttribute(node, "label")}`;

function selectNode(node: string | null) {
    const state = renderer.getGraphState();
    if (!node || (state.frozen && state.selected === node)) {
        details.classList.add("hidden");
        renderer.setGraphState({ frozen: false, selected: node });
        return;
    }

    details.classList.remove("hidden");
    renderer.setGraphState({ frozen: true, selected: node });
    if (!node) return;

    {
        const label = document.getElementById("node-label")! as HTMLAnchorElement;
        label.style.setProperty("--color", graph.getNodeAttribute(node, "color"));
        label.innerText = graph.getNodeAttribute(node, "label");

        label.href = urlFor(node);
    }
    {
        const stat = document.getElementById("stat-harmonic-centrality")!;
        stat.innerText = graph.getNodeAttribute(node, "harmonic_centrality").toPrecision(3);
    }
    {
        const stat = document.getElementById("stat-betweenness-centrality")!;
        stat.innerText = graph.getNodeAttribute(node, "betweenness_centrality").toPrecision(3);
    }
    {
        const stat = document.getElementById("stat-page-rank")!;
        stat.innerText = graph.getNodeAttribute(node, "page_rank").toPrecision(3);
    }
}

renderer.rawEmitter.emit("reflow");

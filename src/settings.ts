import type { Renderer } from "./render";

export const settings = {
    includeExternal: false,
    showPredecessors: true,
    showSuccessors: true,
    strongGravity: false,
    highlightFalloff: 0,
};

const settingsController: Record<string, (renderer: Renderer, a: HTMLInputElement) => void> = {
    "include-external": (_, el) => settings.includeExternal = el.checked,
    "show-pred": (_, el) => settings.showPredecessors = el.checked,
    "show-succ": (_, el) => settings.showSuccessors = el.checked,
    "highlight-falloff": (_, el) => {
        settings.highlightFalloff = el.valueAsNumber;
    },
    "reflow": renderer => renderer.rawEmitter.emit("reflow"),
};

export const attachSettings = (renderer: Renderer) => {
    for (const id in settingsController) {
        const el = document.getElementById(id);
        if (!el) continue;

        settingsController[id](renderer, el as any);

        const events = el.tagName === "INPUT" ? ["input", "change"] : ["click"];

        for (const event of events) {
            el.addEventListener(event, _ => {
                settingsController[id](renderer, el as any);

                renderer.refresh();
            });
        }
    }
};

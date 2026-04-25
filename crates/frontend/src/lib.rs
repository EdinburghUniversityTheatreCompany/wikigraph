#![cfg(target_arch = "wasm32")]
mod app;
mod model;
mod platform;
mod shapes;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use web_sys::{Request, Response};
use wikigraph_model::LinkGraph;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    // Fire and forget: kick off the async runner
    js_sys::futures::spawn_local(async {
        let _ = run().await;
    });
    Ok(())
}

#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id("egui")
        .ok_or_else(|| JsValue::from_str("canvas with id 'egui' not found"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("failed to cast to HtmlCanvasElement"))?;

    let text = platform::fetch_body("/wikigraph/graph.json").await?;

    let graph: LinkGraph =
        serde_json::from_str(&text).map_err(|err| JsValue::from_str(&err.to_string()))?;

    let web_options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(move |cc| Ok::<Box<dyn eframe::App>, _>(Box::new(app::App::new(cc, &graph)))),
        )
        .await?;
    Ok(())
}

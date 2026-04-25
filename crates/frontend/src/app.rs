use eframe::CreationContext;
use egui::{Color32, Pos2};
use egui_graphs::{
    FruchtermanReingoldWithCenterGravity, FruchtermanReingoldWithCenterGravityState,
    LayoutForceDirected,
};
use petgraph::{
    Direction::{Incoming, Outgoing},
    graph::NodeIndex,
    prelude::StableGraph,
};
use rand::Rng;
use wikigraph_model::*;

use crate::model::*;

pub struct App {
    graph: UiLinkGraph,
    fr_state: Option<FruchtermanReingoldWithCenterGravityState>,
    epoch: web_time::Instant,
}

impl App {
    pub fn new(_: &CreationContext, links: &LinkGraph) -> App {
        let mut graph = UiLinkGraph::new(StableGraph::new());

        let mut rng = rand::rng();
        for (key, obj) in &links.nodes {
            let activity = match key {
                ObjectKey::File(_) => ObjectActivity::Active,
                ObjectKey::External(_) => ObjectActivity::Active,
            };

            graph.add_node_custom(
                UiObject {
                    object: obj.clone(),
                    activity,
                    importance: 0.,
                    altriusm: 0.,
                },
                |n| {
                    n.set_location(Pos2 {
                        x: rng.random_range((0.)..1.),
                        y: rng.random_range((0.)..1.),
                    });
                    let label = match key {
                        ObjectKey::File(s) => format!("/{s}"),
                        ObjectKey::External(s) => s.to_owned(),
                    };
                    n.set_label(label);
                },
            );
        }

        for &(a, b) in &links.edges {
            let (a, b) = (NodeIndex::new(a), NodeIndex::new(b));
            if !graph.g().contains_edge(a, b) {
                graph.add_edge(a, b, UiLink {});
            }
        }

        for i in 0..graph.node_count() {
            let n = NodeIndex::new(i);
            let (mut out, mut inc, mut importance) = (0, 0, 0);

            for m in graph.g().neighbors_directed(n, Outgoing) {
                if let Object::File(FileObject::Markdown) = &graph.g()[m].payload().object {
                    importance += 1;
                }
                out += 1;
            }
            for m in graph.g().neighbors_directed(n, Incoming) {
                if let Object::File(FileObject::Markdown) = &graph.g()[m].payload().object {
                    importance += 1;
                }
                inc += 1;
            }

            let object = graph.g_mut().node_weight_mut(n).unwrap().payload_mut();
            object.importance = 0.5 * (6. * importance as f32 + 1.).sqrt();
            object.altriusm = (out as f32 - inc as f32) / (out as f32 + inc as f32);
        }

        let fr_state = Some(FruchtermanReingoldWithCenterGravityState {
            base: egui_graphs::FruchtermanReingoldState {
                // max_step: 0.0001,
                // dt: 0.0001,
                ..Default::default()
            },
            ..Default::default()
        });
        App {
            graph,
            fr_state,
            epoch: web_time::Instant::now(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let settings_navigation = &egui_graphs::SettingsNavigation::new()
                .with_zoom_and_pan_enabled(true)
                .with_fit_to_screen_enabled(false)
                .with_zoom_speed(0.1)
                .with_fit_to_screen_padding(0.2);
            let settings_style = egui_graphs::SettingsStyle::new()
                .with_labels_always(false)
                .with_edge_stroke_hook(|_, _, _, _| egui::Stroke {
                    width: 0.5,
                    color: Color32::from_rgb(120, 120, 120),
                });

            // let mut st: FruchtermanReingoldWithCenterGravityState =
            //     egui_graphs::get_layout_state(ui, None);

            // st.base.dt = 1. / std::f32::consts::E.powf(self.epoch.elapsed().as_secs_f32() / 4.);

            // egui_graphs::set_layout_state(ui, st, None);

            let mut view = egui_graphs::GraphView::<
                _,
                _,
                _,
                _,
                _,
                _,
                FruchtermanReingoldWithCenterGravityState,
                LayoutForceDirected<FruchtermanReingoldWithCenterGravity>,
            >::new(&mut self.graph)
            .with_navigations(settings_navigation)
            .with_styles(&settings_style);

            ui.add(&mut view);
        });
    }
}

use std::{borrow::Cow, cell::RefCell, f32, rc::Rc};

use eframe::CreationContext;
use egui::{Color32, Pos2};
use egui_graphs::{
    FruchtermanReingoldWithCenterGravity, FruchtermanReingoldWithCenterGravityState,
    LayoutForceDirected, events::Event,
};
use petgraph::{
    Direction::{Incoming, Outgoing},
    graph::NodeIndex,
    prelude::StableGraph,
};
use rand::seq::SliceRandom;
use wikigraph_model::*;

use crate::{model::*, platform};

pub struct App {
    graph: UiLinkGraph,
    fr_state: Option<FruchtermanReingoldWithCenterGravityState>,
    epoch: web_time::Instant,
    events_buf: Rc<RefCell<Vec<Event>>>,
}

impl App {
    pub fn new(_: &CreationContext, links: &LinkGraph) -> App {
        let mut graph = UiLinkGraph::new(StableGraph::new());

        let mut sort: Vec<usize> = (0..links.nodes.len()).collect();
        let mut rng = rand::rng();
        sort.shuffle(&mut rng);

        for (i, (key, obj)) in links.nodes.iter().enumerate() {
            let activity = match key {
                ObjectKey::File(_) => ObjectActivity::Active,
                ObjectKey::External(_) => ObjectActivity::Active,
            };

            graph.add_node_custom(
                UiObject {
                    key: key.clone(),
                    object: obj.clone(),
                    activity,
                    importance: 0.,
                    altriusm: 0.,
                },
                |n| {
                    let radius = 1000.;
                    let angle = f32::consts::TAU * (sort[i] as f32 / sort.len() as f32);

                    n.set_location(Pos2 {
                        x: angle.cos() * radius,
                        y: angle.sin() * radius,
                    });
                    let label = match key {
                        ObjectKey::File(s) => s.to_owned(),
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

            let mut importance = graph.g()[n].payload().object.is_markdown() as usize as f32;
            let (mut out, mut inc) = (0, 0);
            for m in graph.g().neighbors_directed(n, Outgoing) {
                if graph.g()[m].payload().object.is_markdown() {
                    importance += 1.;
                }
                out += 1;
            }
            for m in graph.g().neighbors_directed(n, Incoming) {
                if graph.g()[m].payload().object.is_markdown() {
                    importance += 1.;
                }
                inc += 1;
            }

            let object = graph.g_mut().node_weight_mut(n).unwrap().payload_mut();
            object.importance = 0.5 * (6. * importance + 1.).sqrt();
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
            events_buf: Rc::new(RefCell::new(Vec::new())),
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

            for ev in self.events_buf.borrow_mut().drain(..) {
                match ev {
                    Event::NodeDoubleClick(ev) => {
                        let i = NodeIndex::new(ev.id);
                        const ROOT: &str = "https://wiki.bedlamtheatre.co.uk/";
                        let url = match &self.graph.g()[i].payload().key {
                            ObjectKey::File(s) => Cow::Owned(format!("{ROOT}{s}")),
                            ObjectKey::External(s) => Cow::Borrowed(s.as_str()),
                        };
                        let _ = platform::open_link(&url);
                    }
                    _ => (),
                }
            }

            // let mut st: FruchtermanReingoldWithCenterGravityState =
            //     egui_graphs::get_layout_state(ui, None);

            // st.base.dt = 1. / std::f32::consts::E.powf(self.epoch.elapsed().as_secs_f32() / 4.);

            // egui_graphs::set_layout_state(ui, st, None);
            //

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
            .with_styles(&settings_style)
            .with_event_sink(&self.events_buf);

            ui.add(&mut view);
        });
    }
}

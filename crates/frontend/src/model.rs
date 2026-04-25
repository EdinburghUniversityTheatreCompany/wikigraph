use crate::shapes;
use egui_graphs::Graph;
use wikigraph_model::{Object, ObjectKey};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ObjectActivity {
    Active,
    DeEmphasized,
    Hidden,
}

#[derive(Clone)]
pub struct UiObject {
    pub key: ObjectKey,
    pub object: Object,
    pub activity: ObjectActivity,
    pub importance: f32,
    pub altriusm: f32,
}

#[derive(Clone)]
pub struct UiLink {}

pub type UiLinkGraph =
    Graph<UiObject, UiLink, petgraph::Directed, u32, shapes::ObjectShape, shapes::LinkShape>;

use egui::{
    Color32, FontFamily, FontId, Pos2, Shape, Stroke, Vec2,
    epaint::{CircleShape, TextShape},
};
use egui_graphs::{
    DisplayEdge, DisplayNode, DrawContext, EdgeProps, MetadataFrame, Node, NodeProps,
};
use petgraph::{EdgeType, csr::IndexType};
use wikigraph_model::{FileObject, Object};

use crate::model::{ObjectActivity, UiLink, UiObject};

#[derive(Debug, Copy, Clone)]
pub struct StablePos2 {
    old: Pos2,
    pos: Pos2,
}

impl StablePos2 {
    pub fn new(pos: Pos2) -> StablePos2 {
        StablePos2 { old: pos, pos }
    }

    pub fn update(&mut self, pos: Pos2) {
        // NB: `self.pos` is mean of last two, so we need to reconstruct last point used.
        self.old = (2. * self.pos - self.old).to_pos2();
        self.pos = (self.old + pos.to_vec2()) / 2.
    }

    pub fn pos(&self) -> Pos2 {
        self.pos
    }
}

#[derive(Clone)]
pub struct ObjectShape {
    stable_pos: StablePos2,
    activity: ObjectActivity,
    radius: f32,
    shade: Color32,
    label: String,
    last_scale: f32,
}

#[derive(Clone)]
pub struct LinkShape {}

impl From<NodeProps<UiObject>> for ObjectShape {
    fn from(props: NodeProps<UiObject>) -> Self {
        ObjectShape {
            stable_pos: StablePos2::new(props.location()),
            radius: ObjectShape::radius(props.payload.importance),
            activity: props.payload.activity,
            shade: ObjectShape::shade(&props.payload.object, props.payload.altriusm),
            label: props.label,
            last_scale: f32::NAN,
        }
    }
}

impl From<EdgeProps<UiLink>> for LinkShape {
    fn from(_props: EdgeProps<UiLink>) -> Self {
        LinkShape {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Third {
    First,
    Second,
    Third,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nineslice {
    pub x: Third,
    pub y: Third,
}

impl Nineslice {
    pub const CENTER: Nineslice = Nineslice {
        x: Third::Second,
        y: Third::Second,
    };

    pub fn may_connect_to(self, rhs: Nineslice) -> bool {
        self == Nineslice::CENTER
            || rhs == Nineslice::CENTER
            || (self.x != rhs.x && self.y != rhs.y)
    }
}

impl ObjectShape {
    pub fn pos(&self) -> Pos2 {
        self.stable_pos.pos()
    }

    pub fn radius(importance: f32) -> f32 {
        importance
        // (2. * importance - 1.).log2()
    }
    pub fn shade(object: &Object, altruism: f32) -> Color32 {
        match object {
            Object::File(FileObject::Markdown) => Color32::from_rgb(140, 140, 255)
                .lerp_to_gamma(Color32::from_rgb(100, 100, 255), altruism),
            Object::File(FileObject::Mystery { found: true }) => Color32::from_rgb(140, 255, 140),
            Object::File(FileObject::Mystery { found: false }) => Color32::from_rgb(255, 140, 140),
            Object::External => Color32::from_rgb(200, 200, 100),
        }
    }

    pub fn nineslice_in(&self, meta: &MetadataFrame) -> Nineslice {
        // FIXME: this only half-occlusion.. probably need to fork upstream :((.
        let radius = meta.canvas_to_screen_size(self.radius);
        let bb_br = meta.canvas_to_screen_pos(self.pos()) + Vec2::splat(radius);
        match (bb_br.x < 0., bb_br.y < 0.) {
            (true, true) => Nineslice {
                x: Third::First,
                y: Third::First,
            },
            (true, false) => Nineslice {
                x: Third::First,
                y: Third::Second,
            },
            (false, true) => Nineslice {
                x: Third::Second,
                y: Third::First,
            },
            (false, false) => Nineslice {
                x: Third::Second,
                y: Third::Second,
            },
        }
    }
}

impl<Ty: EdgeType, Ix: IndexType> DisplayNode<UiObject, UiLink, Ty, Ix> for ObjectShape {
    fn closest_boundary_point(&self, dir: Vec2) -> Pos2 {
        self.pos() + dir.normalized() * self.radius
    }

    fn shapes(&mut self, ctx: &DrawContext) -> Vec<Shape> {
        if let ObjectActivity::Hidden = self.activity {
            return Vec::new();
        }

        if self.nineslice_in(ctx.meta) != Nineslice::CENTER {
            return Vec::new();
        }

        let center = ctx.meta.canvas_to_screen_pos(self.pos());
        let radius = ctx.meta.canvas_to_screen_size(self.radius);

        let shape = Shape::Circle(CircleShape {
            center,
            radius,
            fill: self.shade,
            stroke: Stroke::NONE,
        });

        let is_zooming = ctx.meta.zoom != self.last_scale && !self.last_scale.is_nan();
        self.last_scale = ctx.meta.zoom;

        if radius >= 6. && !is_zooming {
            let galley = ctx.ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    self.label.clone(),
                    FontId::new(radius, FontFamily::Monospace),
                    self.shade,
                )
            });

            vec![
                shape,
                Shape::Text(TextShape::new(
                    Pos2 {
                        x: center.x - galley.size().x / 2.,
                        y: center.y - radius - galley.size().y,
                    },
                    galley,
                    self.shade,
                )),
            ]
        } else {
            vec![shape]
        }
    }

    fn update(&mut self, props: &NodeProps<UiObject>) {
        self.stable_pos.update(props.location());
        self.activity = props.payload.activity;
        self.radius = ObjectShape::radius(props.payload.importance);
        self.shade = ObjectShape::shade(&props.payload.object, props.payload.altriusm);
        if props.label != self.label {
            self.label = props.label.clone();
        };
    }

    fn is_inside(&self, pos: Pos2) -> bool {
        (pos - self.pos()).length() <= self.radius
    }
}

impl<Ty: EdgeType, Ix: IndexType> DisplayEdge<UiObject, UiLink, Ty, Ix, ObjectShape> for LinkShape {
    fn shapes(
        &mut self,
        start_node: &Node<UiObject, UiLink, Ty, Ix, ObjectShape>,
        end_node: &Node<UiObject, UiLink, Ty, Ix, ObjectShape>,
        ctx: &DrawContext,
    ) -> Vec<Shape> {
        if start_node.display().activity != ObjectActivity::Active
            || end_node.display().activity != ObjectActivity::Active
        {
            return Vec::new();
        }

        if !start_node
            .display()
            .nineslice_in(ctx.meta)
            .may_connect_to(end_node.display().nineslice_in(ctx.meta))
        {
            return Vec::new();
        }

        let start = start_node.display().pos();
        let end = end_node.display().pos();
        let dir = (end - start).normalized();
        let dirhat = dir.rot90() * 0.4;
        let tip = DisplayNode::<UiObject, UiLink, Ty, Ix>::closest_boundary_point(
            end_node.display(),
            -dir,
        );
        let base = tip - dir * 2.;
        let tipleft = base + dirhat * 2.;
        let tipright = base - dirhat * 2.;

        let color = if let Object::External = &end_node.payload().object {
            Color32::from_rgb(50, 50, 50)
        } else {
            Color32::from_rgb(120, 120, 120)
        };

        vec![
            Shape::LineSegment {
                points: [
                    ctx.meta.canvas_to_screen_pos(start),
                    ctx.meta.canvas_to_screen_pos(base),
                ],
                stroke: egui::Stroke {
                    width: ctx.meta.canvas_to_screen_size(0.5),
                    color,
                },
            },
            Shape::convex_polygon(
                vec![
                    ctx.meta.canvas_to_screen_pos(tip),
                    ctx.meta.canvas_to_screen_pos(tipleft),
                    ctx.meta.canvas_to_screen_pos(tipright),
                ],
                color,
                Stroke::NONE,
            ),
        ]
    }

    fn update(&mut self, _props: &EdgeProps<UiLink>) {}

    fn is_inside(
        &self,
        _start: &Node<UiObject, UiLink, Ty, Ix, ObjectShape>,
        _end: &Node<UiObject, UiLink, Ty, Ix, ObjectShape>,
        _pos: Pos2,
    ) -> bool {
        false
    }
}

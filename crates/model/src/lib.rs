use std::{borrow::Cow, fmt};

use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};

#[cfg(feature = "analysis")]
use petgraph::prelude::NodeIndex;

pub mod links;

#[cfg(feature = "analysis")]
pub mod analysis;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeRef(u32);

impl NodeRef {
    pub fn new(n: usize) -> NodeRef {
        (n as u32).into()
    }

    #[cfg(feature = "analysis")]
    pub fn index(self) -> NodeIndex {
        self.0.into()
    }
}

impl From<u32> for NodeRef {
    fn from(value: u32) -> Self {
        NodeRef(value)
    }
}

impl From<NodeRef> for u32 {
    fn from(value: NodeRef) -> Self {
        value.0
    }
}
impl From<NodeRef> for usize {
    fn from(value: NodeRef) -> Self {
        value.0 as usize
    }
}
#[cfg(feature = "analysis")]
impl From<NodeIndex> for NodeRef {
    fn from(value: NodeIndex) -> NodeRef {
        NodeRef::new(value.index())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct WikiPage {
    pub words: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileObject {
    WikiPage(WikiPage),
    Mystery { found: bool },
}

impl fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectKey::File(s) => s.fmt(f),
            ObjectKey::External(s) => s.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Node {
    #[serde(flatten)]
    pub object: Object,
    #[serde(flatten)]
    pub stats: NodeInfo,
}

impl From<Object> for Node {
    fn from(object: Object) -> Node {
        Node {
            object,
            stats: NodeInfo::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Object {
    File(FileObject),
    External,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct NodeInfo {
    pub betweenness_centrality: f32,
    pub harmonic_centrality: f32,
    pub page_rank: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKey {
    File(String),
    External(String),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct LinkGraph {
    #[serde(with = "nodes_serde")]
    pub nodes: IndexMap<ObjectKey, Node>,
    pub edges: Vec<(NodeRef, NodeRef)>,
    #[serde(default)]
    pub shortest_paths: Vec<Vec<usize>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scc: Vec<usize>,
}

impl fmt::Debug for LinkGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkGraph")
            .field("nodes", &self.nodes.len())
            .field("edges", &self.edges.len())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "manifest")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Manifest {
    pub time: jiff::Zoned,
    #[serde(flatten)]
    pub graph: LinkGraph,
}

mod nodes_serde {
    use std::{
        fmt::{self, Formatter},
        hash::BuildHasher,
        marker::PhantomData,
    };

    use indexmap::IndexMap;
    use serde::{
        Deserializer, Serializer,
        de::{SeqAccess, Visitor},
    };

    use crate::{Node, ObjectKey};

    type ObjectMap<S> = IndexMap<ObjectKey, Node, S>;

    pub fn serialize<S, T>(map: &ObjectMap<S>, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: Serializer,
    {
        serializer.collect_seq(map.iter().map(|(k, v)| (k.as_str(), v)))
    }

    struct SeqVisitor<S>(PhantomData<S>);

    impl<'de, S> Visitor<'de> for SeqVisitor<S>
    where
        S: Default + BuildHasher,
    {
        type Value = ObjectMap<S>;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            write!(formatter, "a sequenced map")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut map =
                IndexMap::with_capacity_and_hasher(seq.size_hint().unwrap_or(0), S::default());

            while let Some((key, node)) = seq.next_element()? {
                let node: Node = node;
                map.insert(node.object.materialize_key(key), node);
            }

            Ok(map)
        }
    }

    pub fn deserialize<'de, D, S>(deserializer: D) -> Result<ObjectMap<S>, D::Error>
    where
        D: Deserializer<'de>,
        S: Default + BuildHasher,
    {
        deserializer.deserialize_seq(SeqVisitor(PhantomData))
    }
}

impl Object {
    pub fn is_markdown(&self) -> bool {
        match self {
            Object::File(FileObject::WikiPage(_)) => true,
            _ => false,
        }
    }

    pub fn is_probably_live(&self) -> bool {
        matches!(
            self,
            Object::File(FileObject::WikiPage(_) | FileObject::Mystery { found: true })
                | Object::External
        )
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Object::File(FileObject::WikiPage { .. }) => "wiki page",
            Object::File(FileObject::Mystery { found: true }) => "internal attachment",
            Object::File(FileObject::Mystery { found: false }) => "dead link",
            Object::External => "external link",
        }
    }

    pub fn materialize_key(&self, key: String) -> ObjectKey {
        let key = key.into();
        match self {
            Object::File(_) => ObjectKey::File(key),
            Object::External => ObjectKey::External(key),
        }
    }
}

impl Object {
    pub fn influence_factor(&self) -> f32 {
        fn words_factor(words: u64) -> f32 {
            1. - (words as f32 / -4000.).exp2()
        }

        match self {
            Object::File(FileObject::WikiPage(page)) => 2. + 2. * words_factor(page.words),
            Object::File(FileObject::Mystery { found: _ }) => 2.,
            Object::External => 1.,
        }
    }
}

impl ObjectKey {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectKey::File(s) => s,
            ObjectKey::External(s) => s,
        }
    }

    pub fn to_url(&self, root: &str) -> Cow<'_, str> {
        match self {
            ObjectKey::File(s) => Cow::Owned(format!("{root}{s}")),
            ObjectKey::External(s) => Cow::Borrowed(s.as_str()),
        }
    }

    pub fn from_link_str(mut link_str: &str, location: &str) -> Option<ObjectKey> {
        if let Some(n) = link_str.find("#") {
            link_str = &link_str[..n];
        }

        if !link_str.starts_with("/") && link_str.contains(":") {
            return Some(ObjectKey::External(link_str.to_owned()));
        }

        if link_str.is_empty() {
            return None;
        }

        let Ok(s) = links::lexical_normalize_relative_to(link_str, location) else {
            return None;
        };

        Some(ObjectKey::File(s))
    }
}

impl LinkGraph {
    pub fn new() -> LinkGraph {
        LinkGraph {
            nodes: IndexMap::new(),
            edges: Vec::new(),
            scc: Vec::new(),
            shortest_paths: Vec::new(),
        }
    }

    fn insert_object(
        &mut self,
        key: ObjectKey,
        combinator: impl FnOnce(Option<&Object>) -> Object,
    ) -> NodeRef {
        let entry = match self.nodes.entry(key) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let obj = combinator(Some(&entry.get().object));
                entry.get_mut().object = obj;
                entry
            }
            indexmap::map::Entry::Vacant(entry) => entry.insert_entry(combinator(None).into()),
        };

        NodeRef::new(entry.index())
    }

    fn intern_file(
        &mut self,
        path: String,
        combinator: impl FnOnce(Option<&FileObject>) -> FileObject,
    ) -> NodeRef {
        self.insert_object(ObjectKey::File(path), |obj| match obj {
            Some(obj) => {
                let Object::File(obj) = obj else {
                    unreachable!()
                };
                Object::File(combinator(Some(obj)))
            }
            None => Object::File(combinator(None)),
        })
    }

    pub fn intern_external(&mut self, url: String) -> NodeRef {
        self.insert_object(ObjectKey::External(url), |_| Object::External)
    }

    pub fn intern_mystery_file(&mut self, path: String, found: bool) -> NodeRef {
        self.intern_file(path, |obj| {
            if let Some(FileObject::WikiPage(wiki)) = obj {
                FileObject::WikiPage(wiki.clone())
            } else {
                FileObject::Mystery { found }
            }
        })
    }

    /// Upgrades mystery files if necessary.
    pub fn intern_markdown(&mut self, path: String, page: WikiPage) -> NodeRef {
        self.intern_file(path, |_| FileObject::WikiPage(page))
    }

    pub fn update_markdown(&mut self, index: NodeRef, page: WikiPage) {
        match self.nodes.get_index_mut(index.into()).unwrap() {
            (
                _,
                Node {
                    object: Object::File(FileObject::WikiPage(old)),
                    ..
                },
            ) => *old = page,
            (key, _) => panic!("expected {key:?} to be a wiki page"),
        }
    }

    pub fn link(&mut self, from: NodeRef, to: NodeRef) {
        self.edges.push((from, to));
    }
}

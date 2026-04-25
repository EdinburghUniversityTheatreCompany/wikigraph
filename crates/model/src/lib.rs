use std::fmt;

use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};

pub mod links;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileObject {
    Markdown,
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
pub enum Object {
    File(FileObject),
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKey {
    File(String),
    External(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LinkGraph {
    #[serde(with = "indexmap::map::serde_seq")]
    pub nodes: IndexMap<ObjectKey, Object>,
    pub edges: Vec<(usize, usize)>,
}

impl ObjectKey {
    // pub fn parse_markdown_link(link_str: &str) -> Option<String> {
    //     let s = link_str.split("/");

    // }

    pub fn from_link_str(mut link_str: &str, location: &str) -> Option<ObjectKey> {
        if let Some(n) = link_str.find("#") {
            link_str = &link_str[..n];
        }
        if let Some(n) = link_str.find("?") {
            link_str = &link_str[..n];
        }

        let absolute = link_str.starts_with("/");
        let external = link_str.contains(":");
        if external {
            return Some(ObjectKey::External(link_str.to_owned()));
        }

        if link_str.is_empty() {
            return None;
        }

        let base = if absolute {
            link_str = link_str.strip_prefix("/").unwrap();
            ""
        } else {
            location
        };

        let Some(mut s) = links::normalize_relative_to(link_str, base) else {
            return None;
        };

        if s.ends_with(".md") {
            s.drain(..s.len() - 3);
        }

        // if !combo.exists() {
        //     let mut combo2 = combo.clone();
        //     combo2.push(".md");
        //     if combo2.exists() {
        //         combo = combo2;
        //     }
        // }

        // let path = (&combo)
        //     .strip_prefix(corpus_root)
        //     .map(Path::to_owned)
        //     .unwrap_or_else(|_| combo);

        Some(ObjectKey::File(s))
    }
}

impl LinkGraph {
    pub fn new() -> LinkGraph {
        LinkGraph {
            nodes: IndexMap::new(),
            edges: Vec::new(),
        }
    }

    fn insert_object(
        &mut self,
        key: ObjectKey,
        combinator: impl FnOnce(Option<&Object>) -> Object,
    ) -> usize {
        let entry = match self.nodes.entry(key) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let obj = combinator(Some(entry.get()));
                *entry.get_mut() = obj;
                entry
            }
            indexmap::map::Entry::Vacant(entry) => entry.insert_entry(combinator(None)),
        };

        entry.index()
    }

    fn intern_file(
        &mut self,
        path: String,
        combinator: impl FnOnce(Option<&FileObject>) -> FileObject,
    ) -> usize {
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

    pub fn intern_external(&mut self, url: String) -> usize {
        self.insert_object(ObjectKey::External(url), |_| Object::External)
    }

    pub fn intern_mystery_file(&mut self, path: String, found: bool) -> usize {
        self.intern_file(path, |obj| {
            if let Some(FileObject::Markdown) = obj {
                FileObject::Markdown
            } else {
                FileObject::Mystery { found }
            }
        })
    }

    /// Upgrades mystery files if necessary.
    pub fn intern_markdown(&mut self, path: String) -> usize {
        self.intern_file(path, |_| FileObject::Markdown)
    }

    pub fn link(&mut self, from: usize, to: usize) {
        self.edges.push((from, to));
    }
}

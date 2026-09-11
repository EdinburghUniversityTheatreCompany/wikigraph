mod markdown;

use std::{
    fs, io,
    ops::{ControlFlow, Deref, DerefMut},
    path::{Path, PathBuf},
};

use wikigraph_model::{LinkGraph, NodeRef, ObjectKey, WikiPage, links};

pub struct Corpus {
    path: PathBuf,
    url_root: Option<String>,
}

impl Corpus {
    pub fn from_args(args: &crate::Args) -> io::Result<Corpus> {
        let url_root = args
            .url_root
            .as_deref()
            .map(|r| r.trim_end_matches("/").to_owned());
        Ok(Corpus {
            path: args.input_path.canonicalize()?,
            url_root,
        })
    }

    pub fn walk(&self) -> io::Result<LinkGraph> {
        let mut wlkr = CorpusWalker {
            corpus: self,
            g: LinkGraph::new(),
        };
        wlkr.build_graph_from_walking_files(&self.path)?;
        Ok(wlkr.g)
    }

    fn strip_url_root<'a>(&'a self, s: &'a str) -> &'a str {
        self.url_root
            .as_deref()
            .and_then(|root| s.strip_prefix(root))
            .unwrap_or(s)
    }
}

struct CorpusWalker<'a> {
    corpus: &'a Corpus,
    g: LinkGraph,
}

impl Deref for CorpusWalker<'_> {
    type Target = LinkGraph;

    fn deref(&self) -> &LinkGraph {
        &self.g
    }
}

impl DerefMut for CorpusWalker<'_> {
    fn deref_mut(&mut self) -> &mut LinkGraph {
        &mut self.g
    }
}

impl CorpusWalker<'_> {
    fn path(&self) -> &Path {
        &self.corpus.path
    }

    pub fn intern_unknown(&mut self, key: ObjectKey) -> NodeRef {
        match key {
            ObjectKey::File(path) => {
                let on_disk = links::extend_path_with_link(self.path(), self.path(), &path);
                let found = links::permute_markdown_aliases(on_disk, |p| {
                    if p.is_file() {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                })
                .is_break();
                self.intern_mystery_file(path, found)
            }
            ObjectKey::External(link_str) => self.intern_external(link_str),
        }
    }

    // NB: no symlinks or i'll die.
    fn build_graph_from_walking_files(&mut self, path: &Path) -> io::Result<()> {
        for res in fs::read_dir(path)? {
            let entry = res?;
            let path = entry.path();

            let ignored = path
                .file_name()
                .and_then(|s| s.as_encoded_bytes().first().copied())
                .is_none_or(|s| s == b'.');

            if ignored {
                continue;
            }

            if entry.metadata()?.is_dir() {
                self.build_graph_from_walking_files(&entry.path())?;
            } else {
                let Some(b"md") = path.extension().map(|s| s.as_encoded_bytes()) else {
                    continue;
                };

                let file = fs::read_to_string(&path)?;
                let mut canonical_path = path.canonicalize()?;
                canonical_path.set_extension("");
                let path = canonical_path.strip_prefix(self.path()).unwrap();
                let Ok(location) =
                    links::lexical_normalize_relative_to(&links::path_to_link(path).unwrap(), "")
                else {
                    continue;
                };
                self.process_markdown_file(&file, &location);
            }
        }

        Ok(())
    }

    fn process_markdown_file(&mut self, content: &str, location: &str) {
        let from = self.intern_markdown(location.to_owned(), WikiPage::default());

        let mut links = markdown::walk(self.corpus, location, content);
        for (_, key) in links.by_ref() {
            let to = self.intern_unknown(key);
            self.link(from, to);
        }

        self.update_markdown(from, WikiPage::from(links));
    }
}

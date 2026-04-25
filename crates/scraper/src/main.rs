use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::*,
};
use wikigraph_model::*;

struct CorpusWalker {
    corpus: PathBuf,
    g: LinkGraph,
}

impl Deref for CorpusWalker {
    type Target = LinkGraph;

    fn deref(&self) -> &LinkGraph {
        &self.g
    }
}

impl DerefMut for CorpusWalker {
    fn deref_mut(&mut self) -> &mut LinkGraph {
        &mut self.g
    }
}

impl CorpusWalker {
    pub fn intern_unknown(&mut self, key: ObjectKey) -> usize {
        match key {
            ObjectKey::File(path) => {
                let mut combo = self.corpus.to_owned();
                combo.push(&path);
                let found = combo.exists();
                self.intern_mystery_file(path, found)
            }
            ObjectKey::External(link_str) => self.intern_external(link_str),
        }
    }

    fn walk<P: AsRef<Path>>(corpus: P) -> io::Result<LinkGraph> {
        let corpus = corpus.as_ref();
        let mut wlkr = CorpusWalker {
            corpus: corpus.canonicalize()?,
            g: LinkGraph::new(),
        };
        wlkr.build_graph_from_walking_files(corpus)?;
        Ok(wlkr.g)
    }

    /// NB: no symlinks or i'll die.
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
                let path = canonical_path.strip_prefix(&self.corpus).unwrap();
                let Some(location) =
                    links::normalize_relative_to(&links::path_to_link(path).unwrap(), "")
                else {
                    continue;
                };
                self.process_markdown_file(&file, &location);
            }
        }

        Ok(())
    }

    fn process_markdown_file(&mut self, file: &str, location: &str) {
        let from = self.intern_markdown(location.to_owned());

        let it = pulldown_cmark::Parser::new(file);

        for ev in it {
            match ev {
                pulldown_cmark::Event::Start(tag) => match tag {
                    pulldown_cmark::Tag::Link {
                        link_type: _,
                        dest_url,
                        title: _,
                        id: _,
                    } => {
                        if let Some(key) = ObjectKey::from_link_str(&dest_url, location) {
                            let to = self.intern_unknown(key);
                            self.link(from, to);
                        }
                    }
                    _ => (),
                },
                pulldown_cmark::Event::End(_) => (),
                pulldown_cmark::Event::Text(_) => (),
                pulldown_cmark::Event::Code(_) => (),
                pulldown_cmark::Event::InlineMath(_) => (),
                pulldown_cmark::Event::DisplayMath(_) => (),
                pulldown_cmark::Event::Html(_) => (),
                pulldown_cmark::Event::InlineHtml(_) => (),
                pulldown_cmark::Event::FootnoteReference(_) => (),
                pulldown_cmark::Event::SoftBreak => (),
                pulldown_cmark::Event::HardBreak => (),
                pulldown_cmark::Event::Rule => (),
                pulldown_cmark::Event::TaskListMarker(_) => (),
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let corpus: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("expected a path"))?
        .into();
    let graph = CorpusWalker::walk(&corpus)?;

    let json = serde_json::to_string(&graph)?;
    fs::write("./graph.json", &json)?;

    Ok(())
}

use pulldown_cmark::Event as Ev;
use wikigraph_model::{ObjectKey, WikiPage};

use crate::walker::Corpus;

pub struct LinkWalker<'a> {
    corpus: &'a Corpus,
    location: &'a str,
    page: WikiPage,
    it: pulldown_cmark::Parser<'a>,
}

impl Iterator for LinkWalker<'_> {
    type Item = (pulldown_cmark::LinkType, ObjectKey);

    fn next(&mut self) -> Option<(pulldown_cmark::LinkType, ObjectKey)> {
        loop {
            match self.it.next()? {
                Ev::Start(tag) => match tag {
                    pulldown_cmark::Tag::Link {
                        link_type,
                        dest_url,
                        title: _,
                        id: _,
                    } => {
                        let dest_url = self.corpus.strip_url_root(&dest_url);
                        if let Some(key) = ObjectKey::from_link_str(dest_url, self.location) {
                            return Some((link_type, key));
                        }
                    }
                    _ => (),
                },
                Ev::End(_) => (),
                Ev::Text(text) => self.page.words += text.split_whitespace().count() as u64,
                Ev::Code(_) => (),
                Ev::InlineMath(_) => (),
                Ev::DisplayMath(_) => (),
                Ev::Html(_) => (),
                Ev::InlineHtml(_) => (),
                Ev::FootnoteReference(_) => (),
                Ev::SoftBreak => (),
                Ev::HardBreak => (),
                Ev::Rule => (),
                Ev::TaskListMarker(_) => (),
            }
        }
    }
}

impl From<LinkWalker<'_>> for WikiPage {
    fn from(value: LinkWalker<'_>) -> Self {
        value.page
    }
}

pub fn walk<'a>(corpus: &'a Corpus, location: &'a str, content: &'a str) -> LinkWalker<'a> {
    let it = pulldown_cmark::Parser::new(content);
    LinkWalker {
        corpus,
        location,
        page: WikiPage::default(),
        it,
    }
}

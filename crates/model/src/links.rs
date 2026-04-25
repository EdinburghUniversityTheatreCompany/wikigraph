use std::{
    ops::ControlFlow,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub enum LinkComponent<'a> {
    Root,
    Current,
    Parent,
    Text(&'a str),
}

pub struct LinkComponents<'a> {
    s: &'a str,
    uprooted: bool,
}

impl<'a> LinkComponents<'a> {
    pub fn next_component(&mut self) -> (usize, Option<LinkComponent<'a>>) {
        let (extra, comp) = match self.s.char_indices().find(|&(_, c)| c == '/') {
            Some((i, _)) => (1, &self.s[..i]),
            None => (0, self.s),
        };
        let n = comp.len() + extra;
        let parsed = match comp {
            ".." => Some(LinkComponent::Parent),
            "." => Some(LinkComponent::Current),
            "" => None,
            _ => Some(LinkComponent::Text(comp)),
        };

        (n, parsed)
    }
}

impl<'a> Iterator for LinkComponents<'a> {
    type Item = LinkComponent<'a>;

    fn next(&mut self) -> Option<LinkComponent<'a>> {
        loop {
            if self.s.is_empty() {
                return None;
            }

            if self.uprooted {
                let (n, comp) = self.next_component();
                self.s = &self.s[n..];
                if comp.is_some() {
                    return comp;
                }
            } else {
                self.uprooted = true;
                match self.s.as_bytes() {
                    [b'/', ..] => {
                        self.s = &self.s[1..];
                        return Some(LinkComponent::Root);
                    }
                    [b'.', b'/'] => {
                        self.s = &self.s[2..];
                        return Some(LinkComponent::Current);
                    }
                    _ => (),
                }
            }
        }
    }
}

pub fn components(s: &str) -> LinkComponents<'_> {
    LinkComponents { s, uprooted: false }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizeError;

pub fn lexical_normalize_relative_to(
    mut s: &str,
    mut root: &str,
) -> Result<String, NormalizeError> {
    fn unempty(s: String) -> String {
        if s.is_empty() { ".".to_owned() } else { s }
    }

    if root.ends_with("/") && root.len() > 1 {
        root = &root[..root.len() - 1];
    }

    // if s.ends_with("/index.md") {
    //     s = &s[..s.len() - 9];
    // } else if s.ends_with("index.md") {
    //     s = &s[..s.len() - 8];
    // } else
    if s.ends_with(".md") {
        s = &s[..s.len() - 3];
    }

    let mut comps = components(s).peekable();
    let mut lexical = match comps.peek() {
        Some(LinkComponent::Root) => {
            let lexical = String::with_capacity(1 + s.len());
            comps.next();
            lexical
        }
        Some(LinkComponent::Current) => {
            let mut lexical = String::with_capacity(root.len() + 1 + s.len());
            comps.next();
            lexical.push_str(root);
            lexical
        }
        None => return Ok(unempty(root.to_owned())),
        Some(LinkComponent::Parent | LinkComponent::Text(_)) => {
            let mut lexical = String::with_capacity(root.len() + 1 + s.len());
            lexical.push_str(root);
            lexical
        }
    };

    for comp in comps {
        match comp {
            LinkComponent::Root => unreachable!(),
            LinkComponent::Current => continue,
            LinkComponent::Parent => {
                if lexical.is_empty() {
                    return Err(NormalizeError);
                }
                let last = lexical.rfind("/").unwrap_or(0);
                lexical.drain(last..);
            }
            LinkComponent::Text(s) => {
                if !lexical.is_empty() && !lexical.ends_with("/") {
                    lexical.push_str("/");
                }
                lexical.push_str(s);
            }
        }
    }

    Ok(unempty(lexical))
}

pub fn path_to_link(path: &Path) -> Option<String> {
    let mut t = String::with_capacity(path.as_os_str().len());
    for c in path.components() {
        match c {
            Component::RootDir | Component::Prefix(_) => unreachable!("relative paths only"),
            Component::CurDir => t.push_str("./"),
            Component::ParentDir => t.push_str("../"),
            Component::Normal(s) => {
                t.push_str(str::from_utf8(s.as_encoded_bytes()).ok()?);
                t.push_str("/");
            }
        }
    }

    Some(t)
}

pub fn extend_path_with_link(root: &Path, path: &Path, link: &str) -> PathBuf {
    let mut comps = components(link).peekable();
    let mut path = match comps.peek() {
        Some(LinkComponent::Root) => {
            comps.next();
            root.to_owned()
        }
        _ => path.to_owned(),
    };

    for comp in comps {
        match comp {
            LinkComponent::Root => unreachable!(),
            LinkComponent::Current => continue,
            LinkComponent::Parent => drop(path.pop()),
            LinkComponent::Text(s) => path.push(s),
        }
    }

    path
}

/// Calls `f` for `"{path}"`, `"{path}.md"`.
///
/// `owned` must not have a trailing slash.
pub fn permute_markdown_aliases<B>(
    mut owned: PathBuf,
    mut f: impl FnMut(&Path) -> ControlFlow<B>,
) -> ControlFlow<B> {
    assert!(!owned.as_os_str().as_encoded_bytes().ends_with(b"/"));

    f(&owned)?;

    owned.add_extension("md");
    f(&owned)?;

    // owned.set_extension("");
    // owned.push("index.md");
    // f(&owned)?;

    ControlFlow::Continue(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_relative_to() {
        let a = "foo/bar/baz";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_empty_str() {
        let a = "foo/bar/baz";
        let b = "";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("foo/bar/baz")
        );
    }

    #[test]
    fn normalizes_relative_to_empty_str2() {
        let a = "";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and/that")
        );
    }

    #[test]
    fn normalizes_relative_to_empty_str3() {
        let a = "";
        let b = "";
        assert_eq!(lexical_normalize_relative_to(a, b).as_deref(), Ok("."));
    }

    #[test]
    fn normalizes_relative_to_trailing_slashes() {
        let a = "foo/bar/baz//";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_trailing_slash_base() {
        let a = "foo/bar/baz";
        let b = "this/and/that/";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_curdir() {
        let a = "./foo/./bar/./././baz/.";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_pardir() {
        let a = "../foo/../bar/../baz/..";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("this/and"),
        );
    }

    #[test]
    fn normalizes_relative_to_absolute() {
        let a = "/foo/bar/../baz";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Ok("foo/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_back2root() {
        let a = "../../..";
        let b = "this/and/that";
        assert_eq!(lexical_normalize_relative_to(a, b).as_deref(), Ok("."));
    }

    #[test]
    fn normalizes_relative_to_noescape() {
        let a = "../../../..";
        let b = "this/and/that";
        assert_eq!(
            lexical_normalize_relative_to(a, b).as_deref(),
            Err(&NormalizeError),
        );
    }

    #[test]
    fn normalizes_relative_to_root_curdur() {
        let a = ".";
        let b = "/";
        assert_eq!(lexical_normalize_relative_to(a, b).as_deref(), Ok("/"));
    }
}

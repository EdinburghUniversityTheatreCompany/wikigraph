use std::path::{Component, Path};

pub fn normalize_relative_to(s: &str, mut b: &str) -> Option<String> {
    fn initialize_split_stack(s: &str) -> Vec<usize> {
        // skip last segment since not necessarily over.
        match s.rfind("/") {
            Some(i) => s[..i + 1]
                .split_inclusive("/")
                .scan(0usize, |a, s| {
                    *a += s.len();
                    Some(*a - 1)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    if b.ends_with("/") {
        b = &b[..b.len() - 1];
    }

    let mut t = String::with_capacity(b.len() + 1 + s.len());
    t.push_str(b);
    let mut split_stack: Option<Vec<usize>> = None;
    for c in s.split("/") {
        match c {
            ".." => {
                let split_stack = split_stack.get_or_insert_with(|| initialize_split_stack(&t));
                let i = split_stack.pop()?;
                t.drain(i..);
            }
            "." | "" => continue,
            _ => {
                if let Some(split_stack) = split_stack.as_mut() {
                    split_stack.push(t.len());
                }
                if !t.is_empty() {
                    t.push_str("/");
                }
                t.push_str(c);
            }
        }
    }
    Some(t)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_relative_to() {
        let a = "foo/bar/baz";
        let b = "this/and/that";
        assert_eq!(
            normalize_relative_to(a, b).as_deref(),
            Some("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_empty_str() {
        let a = "foo/bar/baz";
        let b = "";
        assert_eq!(normalize_relative_to(a, b).as_deref(), Some("foo/bar/baz"));
    }

    #[test]
    fn normalizes_relative_to_trailing_slashes() {
        let a = "foo/bar/baz//";
        let b = "this/and/that";
        assert_eq!(
            normalize_relative_to(a, b).as_deref(),
            Some("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_trailing_slash_base() {
        let a = "foo/bar/baz";
        let b = "this/and/that/";
        assert_eq!(
            normalize_relative_to(a, b).as_deref(),
            Some("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_curdir() {
        let a = "./foo/./bar/./././baz/.";
        let b = "this/and/that";
        assert_eq!(
            normalize_relative_to(a, b).as_deref(),
            Some("this/and/that/foo/bar/baz"),
        );
    }

    #[test]
    fn normalizes_relative_to_pardir() {
        let a = "../foo/../bar/../baz/..";
        let b = "this/and/that";
        assert_eq!(normalize_relative_to(a, b).as_deref(), Some("this/and"),);
    }
}

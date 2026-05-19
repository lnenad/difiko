use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct DirNode {
    pub dirs: BTreeMap<String, DirNode>,
    pub files: Vec<String>,
}

impl DirNode {
    pub fn from_paths<'a, I: IntoIterator<Item = &'a str>>(paths: I) -> Self {
        let mut root = DirNode::default();
        for path in paths {
            root.insert(path);
        }
        root
    }

    fn insert(&mut self, path: &str) {
        let mut node = self;
        let mut iter = path.split('/').peekable();
        while let Some(part) = iter.next() {
            if iter.peek().is_none() {
                node.files.push(part.to_string());
            } else {
                node = node.dirs.entry(part.to_string()).or_default();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TreeRow {
    Dir {
        path: String,
        label: String,
        depth: usize,
        collapsed: bool,
    },
    File {
        path: String,
        label: String,
        depth: usize,
    },
}

pub fn flatten(root: &DirNode, collapsed: &std::collections::HashSet<String>) -> Vec<TreeRow> {
    let mut out = Vec::new();
    walk(root, "", 0, &mut out, collapsed);
    out
}

fn walk(
    node: &DirNode,
    prefix: &str,
    depth: usize,
    out: &mut Vec<TreeRow>,
    collapsed: &std::collections::HashSet<String>,
) {
    for (name, child) in &node.dirs {
        let dir_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let is_collapsed = collapsed.contains(&dir_path);
        out.push(TreeRow::Dir {
            path: dir_path.clone(),
            label: name.clone(),
            depth,
            collapsed: is_collapsed,
        });
        if !is_collapsed {
            walk(child, &dir_path, depth + 1, out, collapsed);
        }
    }
    for file in &node.files {
        let path = if prefix.is_empty() {
            file.clone()
        } else {
            format!("{prefix}/{file}")
        };
        out.push(TreeRow::File {
            path,
            label: file.clone(),
            depth,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn builds_tree() {
        let paths = vec!["src/foo.rs", "src/bar/baz.rs", "README.md"];
        let root = DirNode::from_paths(paths);
        let rows = flatten(&root, &HashSet::new());
        // Expect: src dir, src/bar dir, baz.rs file, foo.rs file, README.md
        assert!(rows
            .iter()
            .any(|r| matches!(r, TreeRow::Dir { path, .. } if path == "src")));
        assert!(rows
            .iter()
            .any(|r| matches!(r, TreeRow::File { label, .. } if label == "README.md")));
    }
}

//! Recursive directory walk with a small, predictable ignore list.
//!
//! Dotfiles and build output are skipped so that a bare `peek` run in a
//! project root stays useful instead of drowning in `target/` noise.

use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "vendor",
];

const SKIP_FILES: &[&str] = &[".DS_Store", "Thumbs.db"];

/// Collects regular files below `root`, at most `max_depth` levels deep.
pub fn files(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut found = Vec::new();

    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || SKIP_FILES.contains(&name.as_ref()) {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if depth < max_depth && !SKIP_DIRS.contains(&name.as_ref()) {
                    pending.push((entry.path(), depth + 1));
                }
            } else if kind.is_file() {
                found.push(entry.path());
            }
        }
    }

    found.sort();
    found
}

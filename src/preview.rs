//! Cheap file preview: the first few KiB, decoded lossily, never more.
//!
//! Binary files are detected by a NUL byte and reported instead of dumped.

use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_BYTES: u64 = 8 * 1024;
const SNIFF: usize = 1024;

pub enum Preview {
    Lines(Vec<String>),
    Binary,
    Unreadable,
}

pub fn load(path: &Path, height: usize, width: usize) -> Preview {
    let Ok(file) = File::open(path) else {
        return Preview::Unreadable;
    };

    let mut buffer = Vec::with_capacity(MAX_BYTES as usize);
    if file.take(MAX_BYTES).read_to_end(&mut buffer).is_err() {
        return Preview::Unreadable;
    }
    if buffer.iter().take(SNIFF).any(|byte| *byte == 0) {
        return Preview::Binary;
    }

    let text = String::from_utf8_lossy(&buffer);
    Preview::Lines(
        text.lines()
            .take(height)
            .map(|line| shrink(&sanitize(line), width))
            .collect(),
    )
}

/// Tabs become four spaces and stray control bytes become spaces, so the
/// preview cannot move the cursor out from under the layout.
fn sanitize(line: &str) -> String {
    let mut clean = String::with_capacity(line.len());
    for ch in line.chars() {
        match ch {
            '\t' => clean.push_str("    "),
            c if c.is_control() => clean.push(' '),
            c => clean.push(c),
        }
    }
    clean
}

/// Keeps the left edge and marks the cut with `~`.
fn shrink(line: &str, width: usize) -> String {
    let width = width.max(8);
    if line.chars().count() <= width {
        return line.to_string();
    }
    let mut kept: String = line.chars().take(width - 1).collect();
    kept.push('~');
    kept
}

#[cfg(test)]
mod tests {
    use super::shrink;

    #[test]
    fn long_lines_are_cut_and_marked() {
        let cut = shrink(&"x".repeat(40), 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.ends_with('~'));
    }

    #[test]
    fn short_lines_pass_through() {
        assert_eq!(shrink("hello", 20), "hello");
    }
}

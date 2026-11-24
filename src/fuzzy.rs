//! Subsequence scoring: query chars must appear in order, case insensitively.
//!
//! Tight runs and matches at path boundaries score higher than scattered ones,
//! and every skipped byte costs a point. Shallow paths win close ties.

const BASE: i64 = 100;
const PER_CHAR: i64 = 10;
const ADJACENT: i64 = 15;
const BOUNDARY: i64 = 20;
const UPPERCASE: i64 = 2;

/// Scores `hay` against `needle`. `None` means the needle does not match.
pub fn score(needle: &str, hay: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(1);
    }

    let hay: Vec<char> = hay.chars().collect();
    let mut total = BASE;
    let mut previous: Option<usize> = None;
    let mut cursor = 0;

    for wanted in needle.chars().flat_map(char::to_lowercase) {
        let hit = loop {
            let index = cursor;
            if index >= hay.len() {
                return None;
            }
            cursor += 1;
            if hay[index].to_ascii_lowercase() == wanted {
                break index;
            }
        };

        total += PER_CHAR;
        if let Some(prev) = previous {
            let gap = hit - prev - 1;
            total -= gap as i64;
            if gap == 0 {
                total += ADJACENT;
            }
        }
        if is_boundary(&hay, hit) {
            total += BOUNDARY;
        }
        if hay[hit].is_ascii_uppercase() && wanted.is_ascii_lowercase() {
            total += UPPERCASE;
        }
        previous = Some(hit);
    }

    total -= hay.len() as i64 / 8;
    Some(total.max(1) as u32)
}

fn is_boundary(hay: &[char], index: usize) -> bool {
    match index.checked_sub(1) {
        None => true,
        Some(prev) => {
            let ch = hay[prev];
            matches!(ch, '/' | '\\' | '-' | '_' | '.' | ' ')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn matches_in_order_only() {
        assert!(score("src", "src/main.rs").is_some());
        assert!(score("main", "src/main.rs").is_some());
        assert!(score("mrz", "src/main.rs").is_none());
    }

    #[test]
    fn filename_beats_scattered_path() {
        let tight = score("main", "src/main.rs").unwrap();
        let loose = score("main", "migrations/asset/index.md").unwrap();
        assert!(tight > loose);
    }

    #[test]
    fn empty_needle_matches_everything() {
        assert!(score("", "anything").is_some());
    }
}

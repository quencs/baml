//! String utility functions used throughout the BAML compiler.

/// Compute Levenshtein edit distance between two strings.
///
/// This measures the minimum number of single-character edits (insertions,
/// deletions, or substitutions) required to transform one string into another.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows for space efficiency
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a_chars[i - 1] != b_chars[j - 1]);
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Find similar names from a list of candidates using edit distance.
///
/// Returns up to `max_suggestions` suggestions that are within a reasonable
/// edit distance of the target name, sorted by similarity (most similar first).
///
/// The threshold is computed as: `max(target.len(), 3) * 2 / 5 + 1`
/// This allows roughly 40% of the target length in edits, with a minimum of 2 edits.
pub fn find_similar_names<'a>(
    target: &str,
    candidates: impl Iterator<Item = &'a str>,
    max_suggestions: usize,
) -> Vec<String> {
    let target_lower = target.to_lowercase();
    let threshold = target.len().max(3) * 2 / 5 + 1;

    let mut scored: Vec<(String, usize)> = candidates
        .filter(|c| *c != target) // Exclude exact match
        .map(|c| {
            let c_lower = c.to_lowercase();
            let dist = edit_distance(&target_lower, &c_lower);
            (c.to_string(), dist)
        })
        .filter(|(_, dist)| *dist <= threshold)
        .collect();

    // Sort by edit distance (ascending), then alphabetically for ties
    scored.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    // Return up to max_suggestions
    scored
        .into_iter()
        .take(max_suggestions)
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance_identical() {
        assert_eq!(edit_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_edit_distance_empty() {
        assert_eq!(edit_distance("", "hello"), 5);
        assert_eq!(edit_distance("hello", ""), 5);
        assert_eq!(edit_distance("", ""), 0);
    }

    #[test]
    fn test_edit_distance_one_char() {
        assert_eq!(edit_distance("hello", "hallo"), 1); // substitution
        assert_eq!(edit_distance("hello", "hell"), 1); // deletion
        assert_eq!(edit_distance("hello", "helloo"), 1); // insertion
    }

    #[test]
    fn test_edit_distance_multiple() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_find_similar_names_basic() {
        let candidates = vec!["User", "UserProfile", "Account", "Admin"];
        let suggestions = find_similar_names("Usar", candidates.iter().map(|s| *s), 3);
        assert_eq!(suggestions, vec!["User"]);
    }

    #[test]
    fn test_find_similar_names_case_insensitive() {
        let candidates = vec!["User", "USER", "user"];
        let suggestions = find_similar_names("usar", candidates.iter().map(|s| *s), 3);
        // All should match with distance 1, sorted alphabetically
        assert!(suggestions.contains(&"User".to_string()));
    }

    #[test]
    fn test_find_similar_names_no_matches() {
        let candidates = vec!["Apple", "Banana", "Cherry"];
        let suggestions = find_similar_names("Xyz", candidates.iter().map(|s| *s), 3);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_find_similar_names_excludes_exact() {
        let candidates = vec!["User", "User", "Admin"];
        let suggestions = find_similar_names("User", candidates.iter().map(|s| *s), 3);
        assert!(!suggestions.contains(&"User".to_string()));
    }
}

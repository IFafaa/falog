//! Text helpers for forgiving comparisons of names typed or dictated by people.

/// Trims, lowercases and strips common Latin diacritics (`" Café "` → `"cafe"`).
pub fn fold(s: &str) -> String {
    s.trim()
        .chars()
        .flat_map(char::to_lowercase)
        .map(strip_diacritic)
        .collect()
}

/// Whether `haystack` contains `needle`, ignoring case and diacritics.
/// An empty needle matches everything.
pub fn fuzzy_contains(haystack: &str, needle: &str) -> bool {
    let needle = fold(needle);
    needle.is_empty() || fold(haystack).contains(&needle)
}

fn strip_diacritic(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_case_and_accents() {
        assert_eq!(fold("  Acme CAFÉ "), "acme cafe");
        assert_eq!(fold("Résumé"), "resume");
    }

    #[test]
    fn fuzzy_contains_ignores_accents() {
        assert!(fuzzy_contains("Crème brûlée recipe", "creme brulee"));
        assert!(fuzzy_contains("anything", ""));
        assert!(!fuzzy_contains("login", "logout"));
    }
}

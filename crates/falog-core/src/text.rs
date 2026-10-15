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
        assert_eq!(fold("  Work CAFÉ "), "work cafe");
        assert_eq!(fold("Résumé"), "resume");
    }

    #[test]
    fn fuzzy_contains_ignores_accents() {
        assert!(fuzzy_contains("Crème brûlée recipe", "creme brulee"));
        assert!(fuzzy_contains("anything", ""));
        assert!(!fuzzy_contains("login", "logout"));
    }

    #[test]
    fn strips_every_handled_diacritic_in_both_cases() {
        let folded = "aaaaaa eeee iiii ooooo uuuu c n";
        assert_eq!(fold("áàâãäå éèêë íìîï óòôõö úùûü ç ñ"), folded);
        assert_eq!(fold("ÁÀÂÃÄÅ ÉÈÊË ÍÌÎÏ ÓÒÔÕÖ ÚÙÛÜ Ç Ñ"), folded);
    }

    #[test]
    fn keeps_other_characters_and_inner_spaces() {
        assert_eq!(fold("ø ß #12 2026-10-09"), "ø ß #12 2026-10-09");
        assert_eq!(fold("\t Ana  Lúcia \n"), "ana  lucia");
        assert_eq!(fold("   "), "");
    }

    #[test]
    fn fuzzy_contains_folds_both_sides() {
        assert!(fuzzy_contains("SAÚDE", "saude"));
        assert!(fuzzy_contains("saude", " SAÚDE "));
        assert!(fuzzy_contains("anything", "   "));
        assert!(!fuzzy_contains("", "a"));
    }
}

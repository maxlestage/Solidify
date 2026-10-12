//! What the documentation's search compares, the same on the server (the index written
//! in every page) and in the browser (what the visitor types): words in lower case,
//! without accents, punctuation as spaces.

/// `text` as the search compares it.
pub fn norm(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => out.push('a'),
            'ç' => out.push('c'),
            'è' | 'é' | 'ê' | 'ë' => out.push('e'),
            'ì' | 'í' | 'î' | 'ï' => out.push('i'),
            'ñ' => out.push('n'),
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => out.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => out.push('u'),
            'ý' | 'ÿ' => out.push('y'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            c if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How well an entry matches `query` (both normalized): 0 when one of the query's words
/// is in none of the entry's, more when they are in its title, and more again when its
/// title starts with them.
pub fn score(query: &str, title: &str, words: &str) -> u32 {
    let mut total = 0;
    for word in query.split_whitespace() {
        let in_title = title.split(' ').any(|w| w.starts_with(word));
        total += if title.starts_with(word) {
            12
        } else if in_title {
            8
        } else if title.contains(word) {
            5
        } else if words.split(' ').any(|w| w.starts_with(word)) {
            3
        } else if words.contains(word) {
            1
        } else {
            return 0;
        };
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_words_without_accents_or_case() {
        assert_eq!(
            norm("Démarrage  rapide : `--check`"),
            "demarrage rapide check"
        );
        assert_eq!(norm("Option<T> & Œuvre"), "option t oeuvre");
        let title = norm("Ligne de commande");
        let words = norm("Ligne de commande npx solidify-rs options codes de sortie");
        assert!(score("ligne", &title, &words) > score("commande", &title, &words));
        assert!(score("comm", &title, &words) > score("sortie", &title, &words));
        assert!(score("sortie", &title, &words) > 0);
        assert_eq!(score("sortie async", &title, &words), 0);
    }
}

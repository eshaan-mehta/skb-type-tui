//! The n-grams mode course: common English letter sequences, practiced three at a time.
//!
//! Each list is ranked by how often the sequence occurs inside words in Google Books (Peter
//! Norvig's "English Letter Frequency Counts: Mayzner Revisited"). Left out are sequences that
//! are mostly one short word ("of", "was", "that") or mostly part of a longer one ("tio",
//! nearly always in "tion"), except "the" and "and", which are too common to skip.

/// How many sets the course has; after the last it starts over.
pub const SETS: usize = 30;

pub const PAIRS: [&str; SETS] = [
    "th", "he", "in", "er", "an", "re", "on", "at", "en", "nd", "ti", "es", "or", "te", "ed",
    "is", "it", "al", "ar", "st", "to", "nt", "ng", "se", "ha", "as", "ou", "io", "le", "ve",
];

pub const TRIPLES: [&str; SETS] = [
    "the", "and", "ing", "ent", "for", "ter", "ere", "ate", "con", "res", "ver", "all", "nce", "ted", "ers",
    "pro", "are", "ess", "ive", "ect", "rea", "com", "eve", "per", "int", "est", "sta", "ist", "ear", "ain",
];

/// Four and five letters.
pub const LONGER: [&str; SETS] = [
    "tion", "ation", "ther", "with", "ment", "ould", "ting", "ction", "ence", "ight", "sion", "ever", "ical",
    "ough", "ance", "tive", "over", "ding", "pres", "comp", "able", "ally", "ated", "ring", "ture", "cont",
    "cons", "part", "form", "ning",
];

/// The sequences set `i` practices: the next most common pair, triple and longer one.
pub fn set(i: usize) -> [&'static str; 3] {
    let i = i % SETS;
    [PAIRS[i], TRIPLES[i], LONGER[i]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_go_round() {
        assert_eq!(set(0), ["th", "the", "tion"]);
        assert_eq!(set(2), ["in", "ing", "ther"]);
        assert_eq!(set(SETS), set(0));
    }

    #[test]
    fn every_sequence_is_in_real_words() {
        let corpus = crate::words::corpus();
        for g in PAIRS.iter().chain(&TRIPLES).chain(&LONGER) {
            let n = corpus.iter().filter(|w| w.contains(g) && *w != g).count();
            assert!(n >= 2, "{g}: {n} words");
        }
    }
}

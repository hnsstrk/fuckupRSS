//! Stopwords for German and English
//!
//! Common words that should be filtered out during keyword extraction.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Combined German and English stopwords
pub static STOPWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut words = HashSet::new();

    // German stopwords
    for word in GERMAN_STOPWORDS {
        words.insert(*word);
    }

    // English stopwords
    for word in ENGLISH_STOPWORDS {
        words.insert(*word);
    }

    words
});

static GERMAN_STOPWORDS: &[&str] = &[
    // Articles
    "der", "die", "das", "den", "dem", "des", "ein", "eine", "einer", "einem", "einen", "eines",
    // Pronouns
    "ich", "du", "er", "sie", "es", "wir", "ihr", "sie", "mich", "dich", "sich", "uns", "euch",
    "mir", "dir", "ihm", "ihr", "ihnen", "mein", "dein", "sein", "unser", "euer",
    "meiner", "deiner", "seiner", "unserer", "eurer", "meinen", "deinen", "seinen",
    // Prepositions
    "in", "an", "auf", "aus", "bei", "mit", "nach", "über", "unter", "vor", "zwischen",
    "durch", "für", "gegen", "ohne", "um", "von", "zu", "bis", "seit", "während",
    // Conjunctions
    "und", "oder", "aber", "denn", "weil", "wenn", "als", "ob", "dass", "damit", "obwohl",
    "sondern", "sowohl", "weder", "noch", "entweder", "bevor", "nachdem", "sobald",
    // Auxiliary verbs
    "sein", "haben", "werden", "ist", "sind", "war", "waren", "hat", "hatte", "hatten",
    "wird", "wurde", "wurden", "kann", "können", "konnte", "konnten", "muss", "müssen",
    "musste", "mussten", "soll", "sollen", "sollte", "sollten", "will", "wollen",
    "wollte", "wollten", "darf", "dürfen", "durfte", "durften", "mag", "mögen",
    // Adverbs
    "auch", "noch", "schon", "nur", "sehr", "so", "wie", "hier", "dort", "da", "dann",
    "wann", "wo", "warum", "weshalb", "wieso", "jetzt", "nun", "immer", "nie", "oft",
    "manchmal", "vielleicht", "bestimmt", "sicher", "etwa", "ungefähr", "fast", "ganz",
    "gar", "ziemlich", "recht", "eher", "mehr", "weniger", "meist", "mindestens",
    // Adjectives (common)
    "andere", "anderer", "anderes", "anderen", "anderem", "alle", "aller", "alles", "allem",
    "viel", "viele", "vieler", "vielen", "vielem", "wenig", "wenige", "weniger",
    "einige", "einiger", "einiges", "einigen", "einigem", "manche", "mancher", "manches",
    "jede", "jeder", "jedes", "jeden", "jedem", "keine", "keiner", "keines", "keinen", "keinem",
    // Numbers
    "eins", "zwei", "drei", "vier", "fünf", "erste", "zweite", "dritte",
    // Question words
    "was", "wer", "wen", "wem", "wessen", "welche", "welcher", "welches", "welchen", "welchem",
    // Demonstratives
    "diese", "dieser", "dieses", "diesen", "diesem", "jene", "jener", "jenes", "jenen", "jenem",
    // Other common words
    "nicht", "kein", "nein", "ja", "doch", "mal", "halt", "eben", "wohl", "zwar",
    "jedoch", "allerdings", "freilich", "jedenfalls", "übrigens", "nämlich",
    "eigentlich", "überhaupt", "sozusagen", "gleichsam", "gewissermaßen",
    // Web/article common words
    "mehr", "lesen", "artikel", "seite", "weitere", "weiteren", "neuen", "neue", "neuer",
    "aktuell", "aktuelle", "aktuellen", "heute", "gestern", "morgen",
];

static ENGLISH_STOPWORDS: &[&str] = &[
    // Articles
    "a", "an", "the",
    // Pronouns
    "i", "me", "my", "myself", "we", "our", "ours", "ourselves", "you", "your", "yours",
    "yourself", "yourselves", "he", "him", "his", "himself", "she", "her", "hers",
    "herself", "it", "its", "itself", "they", "them", "their", "theirs", "themselves",
    "what", "which", "who", "whom", "this", "that", "these", "those",
    // Prepositions
    "in", "on", "at", "by", "for", "with", "about", "against", "between", "into",
    "through", "during", "before", "after", "above", "below", "to", "from", "up",
    "down", "out", "off", "over", "under", "again", "further", "then", "once",
    // Conjunctions
    "and", "but", "if", "or", "because", "as", "until", "while", "of", "although",
    "though", "whereas", "whether", "unless", "since", "so", "than",
    // Auxiliary verbs
    "am", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "having", "do", "does", "did", "doing", "would", "should", "could", "ought",
    "can", "cannot", "will", "shall", "may", "might", "must",
    // Adverbs
    "here", "there", "when", "where", "why", "how", "all", "each", "every", "both",
    "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only",
    "own", "same", "too", "very", "just", "also", "now", "even", "still", "already",
    // Common adjectives
    "any", "many", "much", "another", "one", "two", "three", "first", "second", "third",
    // Contractions (without apostrophe)
    "dont", "doesnt", "didnt", "wont", "wouldnt", "shouldnt", "couldnt", "cant",
    "cannot", "isnt", "arent", "wasnt", "werent", "hasnt", "havent", "hadnt",
    // Other common words
    "get", "got", "getting", "make", "made", "making", "let", "say", "said", "saying",
    "go", "going", "went", "gone", "come", "coming", "came", "take", "taken", "taking",
    "took", "see", "seen", "seeing", "saw", "know", "known", "knowing", "knew",
    "think", "thought", "thinking", "want", "wanted", "wanting", "use", "used", "using",
    // Web/article common words
    "read", "more", "article", "page", "new", "click", "share", "comment", "comments",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwords_contains_common_words() {
        assert!(STOPWORDS.contains("der"));
        assert!(STOPWORDS.contains("die"));
        assert!(STOPWORDS.contains("the"));
        assert!(STOPWORDS.contains("and"));
    }

    #[test]
    fn test_stopwords_does_not_contain_content_words() {
        assert!(!STOPWORDS.contains("politik"));
        assert!(!STOPWORDS.contains("technology"));
        assert!(!STOPWORDS.contains("wirtschaft"));
    }
}

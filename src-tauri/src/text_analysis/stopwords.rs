//! Stopwords for German, English, and HTML/technical terms
//!
//! Common words that should be filtered out during keyword extraction.
//! Includes built-in stopwords and user-defined stopwords from database.

use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Combined German, English, HTML/technical, and news stopwords
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

    // HTML/technical stopwords
    for word in HTML_STOPWORDS {
        words.insert(*word);
    }

    // News-specific stopwords
    for word in NEWS_STOPWORDS {
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

/// HTML tags, attributes, and web-related technical terms
static HTML_STOPWORDS: &[&str] = &[
    // HTML tags
    "html", "head", "body", "div", "span", "p", "br", "hr", "img", "a", "ul", "ol", "li",
    "table", "tr", "td", "th", "thead", "tbody", "tfoot", "form", "input", "button",
    "label", "select", "option", "textarea", "header", "footer", "nav", "aside",
    "section", "article", "main", "figure", "figcaption", "video", "audio", "iframe",
    "canvas", "svg", "script", "style", "link", "meta", "title", "noscript", "embed",
    "object", "param", "source", "track", "picture", "map", "area", "blockquote",
    "pre", "code", "em", "strong", "small", "sub", "sup", "mark", "del", "ins",
    "abbr", "cite", "dfn", "kbd", "samp", "var", "wbr", "details", "summary",
    "dialog", "menu", "menuitem", "template", "slot", "fieldset", "legend",
    "datalist", "output", "progress", "meter", "ruby", "rt", "rp", "bdi", "bdo",
    "col", "colgroup", "caption", "optgroup", "address", "time", "data",
    // HTML attributes
    "href", "src", "alt", "class", "id", "name", "type", "value", "placeholder",
    "action", "method", "target", "rel", "title", "style", "onclick", "onload",
    "onchange", "onsubmit", "onmouseover", "onmouseout", "onfocus", "onblur",
    "disabled", "readonly", "required", "checked", "selected", "multiple",
    "autofocus", "autocomplete", "maxlength", "minlength", "pattern", "min", "max",
    "step", "cols", "rows", "wrap", "width", "height", "size", "accept", "charset",
    "content", "http", "equiv", "async", "defer", "crossorigin", "integrity",
    "loading", "decoding", "srcset", "sizes", "media", "preload", "autoplay",
    "controls", "loop", "muted", "poster", "datetime", "download", "hreflang",
    "ping", "referrerpolicy", "sandbox", "allow", "allowfullscreen", "frameborder",
    "scrolling", "marginwidth", "marginheight", "xmlns", "lang", "dir", "tabindex",
    "accesskey", "draggable", "contenteditable", "spellcheck", "translate", "hidden",
    "role", "aria", "data", "slot", "part", "exportparts", "is",
    // HTML entities (decoded)
    "nbsp", "quot", "amp", "lt", "gt", "apos", "copy", "reg", "trade", "euro",
    "pound", "yen", "cent", "sect", "deg", "plusmn", "times", "divide", "frac",
    "mdash", "ndash", "lsquo", "rsquo", "ldquo", "rdquo", "bull", "hellip",
    // CSS properties and values
    "px", "em", "rem", "vh", "vw", "vmin", "vmax", "ch", "ex", "pt", "pc", "cm", "mm",
    "auto", "none", "inherit", "initial", "unset", "revert", "block", "inline",
    "flex", "grid", "absolute", "relative", "fixed", "sticky", "static",
    "margin", "padding", "border", "outline", "background", "color", "font",
    "display", "position", "float", "clear", "overflow", "visibility", "opacity",
    "transform", "transition", "animation", "cursor", "pointer", "text", "align",
    "vertical", "horizontal", "top", "bottom", "left", "right", "center", "middle",
    "justify", "stretch", "wrap", "nowrap", "row", "column", "reverse", "space",
    "between", "around", "evenly", "start", "end", "baseline", "content", "items",
    "self", "order", "grow", "shrink", "basis", "gap", "template", "repeat",
    "minmax", "fit", "fill", "span", "dense", "rgb", "rgba", "hsl", "hsla", "hex",
    "transparent", "currentcolor", "solid", "dashed", "dotted", "double", "groove",
    "ridge", "inset", "outset", "collapse", "separate", "hidden", "visible", "scroll",
    "clip", "ellipsis", "break", "word", "normal", "pre", "bold", "italic", "underline",
    "overline", "line", "through", "capitalize", "uppercase", "lowercase", "serif",
    "sans", "monospace", "cursive", "fantasy", "system", "ui", "emoji", "math",
    // URL and protocol related
    "https", "http", "ftp", "mailto", "tel", "javascript", "data", "blob", "file",
    "www", "com", "org", "net", "de", "edu", "gov", "io", "co", "uk", "eu", "info",
    "html", "htm", "php", "asp", "aspx", "jsp", "cgi", "xml", "json", "css", "js",
    "png", "jpg", "jpeg", "gif", "svg", "webp", "ico", "pdf", "doc", "docx", "xls",
    "xlsx", "ppt", "pptx", "zip", "rar", "tar", "gz", "mp3", "mp4", "avi", "mov",
    "webm", "ogg", "wav", "flac", "ttf", "otf", "woff", "woff2", "eot",
    // Common web/technical abbreviations
    "url", "uri", "api", "cdn", "dns", "ssl", "tls", "dom", "ajax", "xhr", "cors",
    "jwt", "oauth", "sso", "cms", "crm", "erp", "saas", "paas", "iaas", "vpc", "sdk",
    "cli", "gui", "ide", "sql", "nosql", "crud", "rest", "soap", "graphql", "rpc",
    "tcp", "udp", "ip", "mac", "lan", "wan", "vpn", "ssh", "sftp", "smtp", "imap",
    "pop", "rss", "atom", "opml", "ical", "vcf", "csv", "tsv", "yaml", "toml", "ini",
    // JavaScript/programming related
    "var", "let", "const", "function", "return", "if", "else", "for", "while", "do",
    "switch", "case", "break", "continue", "try", "catch", "finally", "throw", "new",
    "this", "class", "extends", "super", "static", "get", "set", "async", "await",
    "import", "export", "default", "from", "module", "require", "define", "typeof",
    "instanceof", "null", "undefined", "true", "false", "nan", "infinity",
    "console", "log", "error", "warn", "info", "debug", "alert", "confirm", "prompt",
    "window", "document", "element", "node", "event", "listener", "handler",
    "callback", "promise", "then", "resolve", "reject", "fetch", "response", "request",
    "object", "array", "string", "number", "boolean", "symbol", "map", "set", "date",
    "regexp", "math", "json", "parse", "stringify", "encode", "decode", "buffer",
    // Common tracking/analytics terms
    "tracking", "analytics", "pixel", "beacon", "tag", "gtm", "ga", "utm", "source",
    "medium", "campaign", "term", "cookie", "session", "storage", "local", "cache",
];

/// News-specific stopwords (journalistic terms, news agencies, common filler words)
static NEWS_STOPWORDS: &[&str] = &[
    // German news terms
    "bericht", "sagt", "laut", "unterdessen", "heute", "gestern", "video", "update",
    "interview", "kommentar", "mehr", "neue", "ersten", "lesen", "artikel", "news",
    "uhr", "foto", "quelle", "dpa", "afp", "reuters", "aktualisiert", "redaktion",
    "meldung", "nachricht", "pressemitteilung", "mitteilung", "stellungnahme",
    // English news terms
    "report", "says", "according", "today", "yesterday", "comment", "read", "article",
    "source", "photo", "breaking", "exclusive", "update", "live", "developing",
    "correspondent", "reporter", "editor", "editorial", "opinion", "analysis",
    "wire", "ap", "upi", "featured", "trending", "viral", "latest",
];

// ============================================================
// USER STOPWORDS (Database-backed)
// ============================================================

/// Load user-defined stopwords from database
pub fn load_user_stopwords(conn: &Connection) -> Result<HashSet<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT word FROM user_stopwords")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut words = HashSet::new();
    for row in rows {
        words.insert(row?.to_lowercase());
    }
    Ok(words)
}

/// Add a user stopword to the database
pub fn add_user_stopword(conn: &Connection, word: &str) -> Result<(), rusqlite::Error> {
    let word_lower = word.trim().to_lowercase();
    if word_lower.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT OR IGNORE INTO user_stopwords (word) VALUES (?)",
        [&word_lower],
    )?;
    Ok(())
}

/// Remove a user stopword from the database
pub fn remove_user_stopword(conn: &Connection, word: &str) -> Result<bool, rusqlite::Error> {
    let word_lower = word.trim().to_lowercase();
    let deleted = conn.execute(
        "DELETE FROM user_stopwords WHERE LOWER(word) = ?",
        [&word_lower],
    )?;
    Ok(deleted > 0)
}

/// Get count of user stopwords
pub fn count_user_stopwords(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM user_stopwords", [], |row| row.get(0))
}

/// Check if a word is a stopword (static + user-defined)
pub fn is_stopword(word: &str, user_stopwords: Option<&HashSet<String>>) -> bool {
    let lower = word.to_lowercase();

    // Check static stopwords first
    if STOPWORDS.contains(lower.as_str()) {
        return true;
    }

    // Check user stopwords if provided
    if let Some(user) = user_stopwords {
        if user.contains(&lower) {
            return true;
        }
    }

    false
}

/// Get all stopwords (static + user) as a combined set
pub fn get_all_stopwords(conn: &Connection) -> HashSet<String> {
    let mut all: HashSet<String> = STOPWORDS.iter().map(|s| s.to_string()).collect();

    if let Ok(user) = load_user_stopwords(conn) {
        all.extend(user);
    }

    all
}

/// Get statistics about stopwords
pub struct StopwordStats {
    pub builtin_count: usize,
    pub user_count: i64,
    pub total_count: usize,
}

pub fn get_stopword_stats(conn: &Connection) -> Result<StopwordStats, rusqlite::Error> {
    let builtin_count = STOPWORDS.len();
    let user_count = count_user_stopwords(conn)?;
    let total_count = builtin_count + user_count as usize;

    Ok(StopwordStats {
        builtin_count,
        user_count,
        total_count,
    })
}

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
    fn test_stopwords_contains_html_terms() {
        assert!(STOPWORDS.contains("https"));
        assert!(STOPWORDS.contains("href"));
        assert!(STOPWORDS.contains("figcaption"));
        assert!(STOPWORDS.contains("span"));
        assert!(STOPWORDS.contains("quot"));
        assert!(STOPWORDS.contains("div"));
        assert!(STOPWORDS.contains("onclick"));
    }

    #[test]
    fn test_stopwords_does_not_contain_content_words() {
        assert!(!STOPWORDS.contains("politik"));
        assert!(!STOPWORDS.contains("technology"));
        assert!(!STOPWORDS.contains("wirtschaft"));
    }
}

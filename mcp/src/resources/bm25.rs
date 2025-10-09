use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
struct Posting<'a> {
    doc: &'a str,
    tf: u32,
}

#[derive(Debug, Default)]
struct TermStats<'a> {
    prose: Vec<Posting<'a>>,
    code: Vec<Posting<'a>>,
    idf: f64,
}

pub struct Index<'a> {
    terms: HashMap<String, TermStats<'a>>,
    len_prose: HashMap<&'a str, u32>,
    len_code: HashMap<&'a str, u32>,
    avg_prose: f64,
    avg_code: f64,
    k1: f64,
    // field params
    w_prose: f64,
    b_prose: f64,
    w_code: f64,
    b_code: f64,
}

/// Mostly AI-generated implementation of Okapi BM25 ranking function for text search
/// https://en.wikipedia.org/wiki/Okapi_BM25
impl<'a> Index<'a> {
    pub fn new(k1: f64) -> Self {
        Self {
            terms: HashMap::new(),
            len_prose: HashMap::new(),
            len_code: HashMap::new(),
            avg_prose: 0.0,
            avg_code: 0.0,
            k1,
            // Mixed corpus default: neutral weights
            w_prose: 1.0,
            b_prose: 0.75,
            w_code: 1.0,
            b_code: 0.50,
        }
    }

    // Keep '_' and '::' and '!' so identifiers, paths and macros survive.
    fn tokenize_raw(s: &str) -> impl Iterator<Item = &str> {
        s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '!'))
            .filter(|t| !t.is_empty())
    }

    // Classify as code if it looks like a Rust identifier/path/macro or contains digits.
    fn looks_code(tok: &str) -> bool {
        tok.chars()
            .any(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            || tok.contains('_')
            || tok.contains("::")
            || tok.ends_with('!')
    }

    // Split order: path '::' -> snake '_' -> camel (acronym-aware), then lowercase.
    fn split_identifier(tok: &str) -> impl Iterator<Item = String> + '_ {
        tok.split("::")
            .flat_map(|path_seg| path_seg.split('_'))
            .flat_map(|seg| Self::split_camel_acronyms(seg))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
    }

    fn split_camel_acronyms(seg: &str) -> Vec<String> {
        let mut out = Vec::new();
        let chars: Vec<char> = seg.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let start = i;
            let c = chars[i];

            if c.is_ascii_uppercase() {
                i += 1;

                // Determine if this is a capitalized word or an acronym
                let is_capitalized_word = i < chars.len() && chars[i].is_ascii_lowercase();

                if is_capitalized_word {
                    // "Request" - consume all lowercase following the capital
                    while i < chars.len() && chars[i].is_ascii_lowercase() {
                        i += 1;
                    }
                } else {
                    // "HTTP" or "URL" - consume consecutive uppercase
                    while i < chars.len() && chars[i].is_ascii_uppercase() {
                        i += 1;
                    }

                    // Handle acronym-word boundary: "HTTPRequest" should split as "HTTP" + "Request"
                    // When we see an acronym followed by lowercase, the last uppercase starts the next word
                    let followed_by_lowercase = i < chars.len() && chars[i].is_ascii_lowercase();
                    let is_multi_char_acronym = i - start > 1;

                    if followed_by_lowercase && is_multi_char_acronym {
                        i -= 1; // Back up one char to start next word
                    }
                }

                out.push(chars[start..i].iter().collect());
                continue;
            }

            if c.is_ascii_lowercase() {
                while i < chars.len() && chars[i].is_ascii_lowercase() {
                    i += 1;
                }
                out.push(chars[start..i].iter().collect());
                continue;
            }

            if c.is_ascii_digit() {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                out.push(chars[start..i].iter().collect());
                continue;
            }

            // Punctuation - skip it
            i += 1;
        }

        out
    }

    // Light lemmatization for prose: lowercase + plural/tense folding.
    fn normalize_prose(tok: &str) -> String {
        let mut t = tok.to_ascii_lowercase();
        if t.ends_with("ies") && t.len() > 3 {
            t.truncate(t.len() - 3);
            t.push('y');
        } else if t.ends_with('s') && !t.ends_with("ss") && t.len() > 3 {
            t.truncate(t.len() - 1);
        }
        if t.ends_with("ing") && t.len() > 5 {
            t.truncate(t.len() - 3);
        } else if t.ends_with("ed") && t.len() > 4 {
            t.truncate(t.len() - 2);
        }
        t
    }

    // De-noise Rust code tokens: keywords, lifetimes, 1-char generics, common attrs/macros.
    fn is_noise_code_token(tok: &str) -> bool {
        const KW: &[&str] = &[
            "fn", "let", "mut", "pub", "use", "mod", "impl", "struct", "enum", "trait", "where",
            "as", "in", "match", "if", "else", "loop", "while", "for", "self", "super", "crate",
            "return", "type", "const", "static", "ref", "move", "async", "await", "dyn", "default",
            "extern", "unsafe", // common attrs/macros seen as tokens in raw text
            "derive", "test", "cfg", "allow", "deny", "warn",
        ];
        if KW.contains(&tok) {
            return true;
        }
        if tok.starts_with('\'') {
            return true;
        } // lifetimes like 'a
        if tok.len() == 1 {
            return true;
        } // T/U/V single-letter generics, braces-as-tokens
        false
    }

    pub fn add_doc(&mut self, doc_id: &'a str, text: &str) -> &mut Self {
        if self.len_prose.contains_key(&doc_id) {
            panic!("duplicate: {doc_id}");
        }

        let mut tf_prose: HashMap<String, u32> = HashMap::new();
        let mut tf_code: HashMap<String, u32> = HashMap::new();
        let mut lp = 0u32;
        let mut lc = 0u32;

        for raw in Self::tokenize_raw(text) {
            if Self::looks_code(raw) {
                // whole identifier/path/macro
                let lower = raw.to_ascii_lowercase();
                if !Self::is_noise_code_token(&lower) {
                    *tf_code.entry(lower).or_insert(0) += 1;
                    lc += 1;
                }
                // subwords (path -> snake -> camel acronyms)
                for sub in Self::split_identifier(raw) {
                    if !Self::is_noise_code_token(&sub) {
                        *tf_code.entry(sub).or_insert(0) += 1;
                        lc += 1;
                    }
                }
            } else {
                let n = Self::normalize_prose(raw);
                *tf_prose.entry(n).or_insert(0) += 1;
                lp += 1;
            }
        }

        self.len_prose.insert(doc_id, lp);
        self.len_code.insert(doc_id, lc);

        for (t, f) in tf_prose {
            self.terms
                .entry(t)
                .or_default()
                .prose
                .push(Posting { doc: doc_id, tf: f });
        }
        for (t, f) in tf_code {
            self.terms
                .entry(t)
                .or_default()
                .code
                .push(Posting { doc: doc_id, tf: f });
        }
        self
    }

    pub fn finalize(&mut self) {
        let n = self.len_prose.len() as f64; // total docs

        let sum_p: u64 = self.len_prose.values().map(|&x| x as u64).sum();
        let sum_c: u64 = self.len_code.values().map(|&x| x as u64).sum();
        self.avg_prose = if n > 0.0 { sum_p as f64 / n } else { 0.0 };
        self.avg_code = if n > 0.0 { sum_c as f64 / n } else { 0.0 };

        for stats in self.terms.values_mut() {
            let mut seen: HashSet<&str> = HashSet::new();
            for p in &stats.prose {
                seen.insert(p.doc);
            }
            for p in &stats.code {
                seen.insert(p.doc);
            }
            let df = seen.len() as f64;

            // Lucene-style non-negative IDF:
            // idf = ln(1 + (N - df + 0.5)/(df + 0.5))
            let num = (n - df + 0.5).max(1e-12);
            let den = (df + 0.5).max(1e-12);
            stats.idf = (1.0 + num / den).ln();
        }
    }

    pub fn score(&self, query: &str) -> Vec<(&'a str, f64)> {
        let mut seen_terms = HashSet::new();
        let mut acc: HashMap<&'a str, f64> = HashMap::new();
        let (k1, w_p, b_p, w_c, b_c, avg_p, avg_c) = (
            self.k1,
            self.w_prose,
            self.b_prose,
            self.w_code,
            self.b_code,
            self.avg_prose,
            self.avg_code,
        );

        // Classify query tokens the same way; include whole+subwords; filter noisy code tokens.
        let mut q_terms: Vec<(String, bool)> = Vec::new(); // (term, is_code)
        for raw in Self::tokenize_raw(query) {
            if Self::looks_code(raw) {
                let whole = raw.to_ascii_lowercase();
                if !Self::is_noise_code_token(&whole) {
                    q_terms.push((whole, true));
                }
                for sub in Self::split_identifier(raw) {
                    if !Self::is_noise_code_token(&sub) {
                        q_terms.push((sub, true));
                    }
                }
            } else {
                q_terms.push((Self::normalize_prose(raw), false));
            }
        }

        for (term, is_code) in q_terms {
            if !seen_terms.insert(term.clone()) {
                continue;
            }
            let Some(ts) = self.terms.get(&term) else {
                continue;
            };
            let idf = ts.idf;

            // query-time field boosts
            let (qb_prose, qb_code) = if is_code { (0.95, 1.05) } else { (1.05, 0.95) };

            for p in &ts.prose {
                let dl = *self.len_prose.get(p.doc).unwrap_or(&0) as f64;
                let tf = p.tf as f64;
                let norm = 1.0 - b_p + b_p * (dl / avg_p.max(1e-9));
                let tfp = w_p * tf / norm;
                *acc.entry(p.doc).or_insert(0.0) +=
                    qb_prose * idf * ((tfp * (k1 + 1.0)) / (tfp + k1));
            }

            for p in &ts.code {
                let dl = *self.len_code.get(p.doc).unwrap_or(&0) as f64;
                let tf = p.tf as f64;
                let norm = 1.0 - b_c + b_c * (dl / avg_c.max(1e-9));
                let tfc = w_c * tf / norm;
                *acc.entry(p.doc).or_insert(0.0) +=
                    qb_code * idf * ((tfc * (k1 + 1.0)) / (tfc + k1));
            }
        }
        let mut v: Vec<_> = acc.into_iter().collect();
        v.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ix<'a>(docs: Vec<(&'a str, &'a str)>) -> Index<'a> {
        let mut idx = Index::new(1.2);
        for (id, text) in docs {
            idx.add_doc(id, text);
        }
        idx.finalize();
        idx
    }

    #[test]
    fn plural_folding_reranks() {
        let idx = ix(vec![
            ("doc1", "the quick brown fox"),
            ("doc2", "the lazy dog"),
            ("doc3", "quick brown dogs"),
        ]);
        let res = idx.score("quick dog");
        assert!(!res.is_empty());
        assert_eq!(res[0].0, "doc3");
    }

    #[test]
    fn pascalcase_identifier_split_matches() {
        // "ParseHttpRequest" -> subwords: parse, http, request
        let idx = ix(vec![
            ("d1", "ParseHttpRequest handles headers"),
            ("d2", "ParseJson handles values"),
        ]);
        let res = idx.score("HTTP request");
        assert!(
            !res.is_empty(),
            "PascalCase should split into HTTP + request"
        );
        assert_eq!(res[0].0, "d1");
    }

    #[test]
    fn snake_case_subwords_match() {
        let idx = ix(vec![
            ("d1", "parse_json extracts fields"),
            ("d2", "parse_http_request validates input"),
            ("d3", "totally unrelated"),
        ]);
        let res = idx.score("http");
        assert!(!res.is_empty());
        assert_eq!(res[0].0, "d2");
    }

    #[test]
    fn module_paths_whole_identifier_beats_plain_word() {
        // Deterministic: query the whole path so only d1 has the exact code token.
        let idx = ix(vec![
            ("d1", "serde::de::Deserialize is implemented here"),
            ("d2", "we implement serde deserialize logic"),
        ]);
        let res = idx.score("serde::de::Deserialize");
        assert!(!res.is_empty());
        assert_eq!(
            res[0].0, "d1",
            "full path should rank highest where present"
        );
    }

    #[test]
    fn module_paths_plain_last_segment_matches_both() {
        // For plain "deserialize", both docs should match; ordering may vary with field norms.
        let idx = ix(vec![
            ("d1", "serde::de::Deserialize is implemented here"),
            ("d2", "we implement deserialize logic"),
        ]);
        let res = idx.score("deserialize");
        assert_eq!(
            res.len(),
            2,
            "both the path-based code token and prose token should match"
        );
        let ids: Vec<&str> = res.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"d1") && ids.contains(&"d2"));
    }

    #[test]
    fn mixed_prose_and_code_weighting() {
        let idx = ix(vec![
            (
                "dA",
                "implement ParseHttpRequest and handle errors errors errors",
            ),
            ("dB", "implement parser and handle errors"),
            ("dC", "no matches here"),
        ]);
        let res = idx.score("ParseHttpRequest errors");
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, "dA");
        assert_eq!(res[1].0, "dB");
    }

    #[test]
    fn empty_query_returns_empty() {
        let idx = ix(vec![("a", "some text"), ("b", "other text")]);
        let res = idx.score("");
        assert!(res.is_empty());
    }

    #[test]
    fn unknown_query_returns_empty() {
        let idx = ix(vec![("a", "foo bar baz"), ("b", "lorem ipsum")]);
        let res = idx.score("nonexistenttoken");
        assert!(res.is_empty());
    }

    #[test]
    #[should_panic]
    fn duplicate_doc_id_panics() {
        let mut idx = Index::new(1.2);
        idx.add_doc("dup", "first").add_doc("dup", "second");
    }

    /// DF must count unique docs across both fields.
    #[test]
    fn idf_uses_unique_doc_frequency_and_scores_are_finite() {
        let mut idx = Index::new(1.2);
        idx.add_doc("d1", "error Error more context");
        idx.add_doc("d2", "only error here");
        idx.finalize();

        let res = idx.score("error");
        assert!(!res.is_empty());
        for &(_, s) in &res {
            assert!(s.is_finite());
        }
        assert_eq!(res[0].0, "d1");
    }
}

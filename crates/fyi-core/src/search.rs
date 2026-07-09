//! Lightweight full-text search over FOI request title/body.
//!
//! Provides an in-memory inverted index and a hybrid score (token overlap +
//! title boost). The [`SearchIndex`] trait is SQLite FTS-friendly so a future
//! FTS5 backend can plug in without changing callers.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Document fields searchable by the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub body: String,
}

/// A single ranked hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub score: f64,
    pub title: String,
}

/// Trait for full-text search backends (in-memory inverted index or SQLite FTS).
pub trait SearchIndex {
    fn index_document(&mut self, doc: SearchDocument);
    fn remove_document(&mut self, id: &str);
    fn search(&self, query: &str, limit: usize) -> Vec<SearchHit>;
    fn document_count(&self) -> usize;
}

/// Tokenize text into lowercase alphanumeric tokens.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

/// Hybrid score: Jaccard-like token overlap on title∪body, with title boost.
///
/// `title_weight` multiplies the contribution of title-only overlap (default 2.0).
pub fn hybrid_score(query_tokens: &[String], doc: &SearchDocument, title_weight: f64) -> f64 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let query_set: HashSet<&str> = query_tokens.iter().map(String::as_str).collect();
    let title_tokens = tokenize(&doc.title);
    let body_tokens = tokenize(&doc.body);
    let title_set: HashSet<&str> = title_tokens.iter().map(String::as_str).collect();
    let body_set: HashSet<&str> = body_tokens.iter().map(String::as_str).collect();

    let title_hits = query_set.intersection(&title_set).count() as f64;
    let body_hits = query_set.intersection(&body_set).count() as f64;
    let denom = query_set.len() as f64;
    if denom == 0.0 {
        return 0.0;
    }
    (title_weight * title_hits + body_hits) / (title_weight * denom + denom)
}

/// In-memory inverted index over request documents.
#[derive(Debug, Default, Clone)]
pub struct InMemorySearchIndex {
    docs: HashMap<String, SearchDocument>,
    /// token → document ids
    inverted: HashMap<String, HashSet<String>>,
}

impl InMemorySearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    fn unindex_tokens(&mut self, id: &str, tokens: &[String]) {
        for token in tokens {
            if let Some(postings) = self.inverted.get_mut(token) {
                postings.remove(id);
                if postings.is_empty() {
                    self.inverted.remove(token);
                }
            }
        }
    }
}

impl SearchIndex for InMemorySearchIndex {
    fn index_document(&mut self, doc: SearchDocument) {
        if let Some(existing) = self.docs.get(&doc.id) {
            let old_tokens: Vec<String> = {
                let mut t = tokenize(&existing.title);
                t.extend(tokenize(&existing.body));
                t
            };
            self.unindex_tokens(&doc.id, &old_tokens);
        }

        let mut tokens = tokenize(&doc.title);
        tokens.extend(tokenize(&doc.body));
        for token in &tokens {
            self.inverted
                .entry(token.clone())
                .or_default()
                .insert(doc.id.clone());
        }
        self.docs.insert(doc.id.clone(), doc);
    }

    fn remove_document(&mut self, id: &str) {
        if let Some(existing) = self.docs.remove(id) {
            let mut tokens = tokenize(&existing.title);
            tokens.extend(tokenize(&existing.body));
            self.unindex_tokens(id, &tokens);
        }
    }

    fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || limit == 0 {
            return Vec::new();
        }

        // Candidate docs: union of postings for query tokens.
        let mut candidates: HashSet<String> = HashSet::new();
        for token in &query_tokens {
            if let Some(postings) = self.inverted.get(token) {
                candidates.extend(postings.iter().cloned());
            }
        }

        let mut hits: Vec<SearchHit> = candidates
            .into_iter()
            .filter_map(|id| {
                let doc = self.docs.get(&id)?;
                let score = hybrid_score(&query_tokens, doc, 2.0);
                if score <= 0.0 {
                    return None;
                }
                Some(SearchHit {
                    id,
                    score,
                    title: doc.title.clone(),
                })
            })
            .collect();

        hits.sort_by(|a, b| match b.score.partial_cmp(&a.score) {
            Some(ord) => ord.then_with(|| a.id.cmp(&b.id)),
            None => a.id.cmp(&b.id),
        });
        hits.truncate(limit);
        hits
    }

    fn document_count(&self) -> usize {
        self.docs.len()
    }
}

/// Helper that builds SQLite FTS5-compatible match expression from a free-text query.
/// Escapes double quotes and joins tokens with AND.
pub fn fts5_match_expression(query: &str) -> String {
    tokenize(query)
        .into_iter()
        .map(|t| format!("\"{}\"", t.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_docs() -> InMemorySearchIndex {
        let mut index = InMemorySearchIndex::new();
        index.index_document(SearchDocument {
            id: "1".into(),
            title: "Budget procurement contracts".into(),
            body: "Request for copies of all procurement contracts awarded in 2024.".into(),
        });
        index.index_document(SearchDocument {
            id: "2".into(),
            title: "Police body camera policy".into(),
            body: "Please provide the operational policy for body-worn cameras.".into(),
        });
        index.index_document(SearchDocument {
            id: "3".into(),
            title: "Hospital waiting times".into(),
            body: "Monthly waiting list statistics for elective surgery.".into(),
        });
        index
    }

    #[test]
    fn tokenize_lowercases_and_splits() {
        assert_eq!(
            tokenize("Hello, World! FOI-2024"),
            vec!["hello", "world", "foi", "2024"]
        );
    }

    #[test]
    fn search_finds_title_match_with_higher_score() {
        let index = sample_docs();
        let hits = index.search("procurement contracts", 10);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "1");
        assert!(hits[0].score > 0.0);
    }

    #[test]
    fn search_finds_body_only_match() {
        let index = sample_docs();
        let hits = index.search("elective surgery", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "3");
    }

    #[test]
    fn hybrid_score_prefers_title_hits() {
        let doc = SearchDocument {
            id: "x".into(),
            title: "cameras".into(),
            body: "unrelated text about hospitals".into(),
        };
        let q = tokenize("cameras");
        let title_score = hybrid_score(&q, &doc, 2.0);
        let body_doc = SearchDocument {
            id: "y".into(),
            title: "unrelated".into(),
            body: "something about cameras here".into(),
        };
        let body_score = hybrid_score(&q, &body_doc, 2.0);
        assert!(title_score > body_score);
    }

    #[test]
    fn remove_document_drops_from_results() {
        let mut index = sample_docs();
        assert_eq!(index.document_count(), 3);
        index.remove_document("2");
        assert_eq!(index.document_count(), 2);
        let hits = index.search("camera", 10);
        assert!(hits.is_empty());
    }

    #[test]
    fn empty_query_returns_no_hits() {
        let index = sample_docs();
        assert!(index.search("   ", 10).is_empty());
        assert!(index.search("unknownxyz", 10).is_empty());
    }

    #[test]
    fn fts5_match_expression_quotes_tokens() {
        let expr = fts5_match_expression("budget contracts");
        assert_eq!(expr, "\"budget\" AND \"contracts\"");
    }

    #[test]
    fn reindex_updates_tokens() {
        let mut index = InMemorySearchIndex::new();
        index.index_document(SearchDocument {
            id: "1".into(),
            title: "Alpha".into(),
            body: "one".into(),
        });
        index.index_document(SearchDocument {
            id: "1".into(),
            title: "Beta".into(),
            body: "two".into(),
        });
        assert!(index.search("Alpha", 5).is_empty());
        assert_eq!(index.search("Beta", 5)[0].id, "1");
    }
}

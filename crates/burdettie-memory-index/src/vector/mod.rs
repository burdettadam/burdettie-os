//! Baseline vector representation and scoring for P2 semantic ranking.
//!
//! Uses TF-IDF weighted bag-of-words vectors stored as sparse (term_index, weight)
//! pairs serialized to BLOBs.  Dense cosine similarity is computed on the fly.

use std::collections::HashMap;

/// A sparse term-weight pair.
#[derive(Debug, Clone, PartialEq)]
pub struct TermWeight {
    pub term_index: u32,
    pub weight: f32,
}

/// Compute TF-IDF weighted vectors for a set of documents.
///
/// Returns `(vocabulary, doc_vectors)` where each doc vector is a list of
/// non-zero term-weight pairs.  The vocabulary maps term → index.
pub fn fit_transform(
    docs: &[&str],
    max_terms: usize,
) -> (HashMap<String, usize>, Vec<Vec<TermWeight>>) {
    // Build vocabulary: term → document frequency
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let mut term_sets: Vec<HashMap<String, usize>> = Vec::with_capacity(docs.len());

    for doc in docs {
        let mut term_counts: HashMap<String, usize> = HashMap::new();
        for token in tokenize(doc) {
            *term_counts.entry(token.clone()).or_insert(0) += 1;
        }
        for term in term_counts.keys() {
            *doc_freq.entry(term.clone()).or_insert(0) += 1;
        }
        term_sets.push(term_counts);
    }

    let n_docs = docs.len() as f32;

    // Sort terms by document frequency (keep top max_terms)
    let mut sorted_terms: Vec<(String, usize)> = doc_freq.into_iter().collect();
    sorted_terms.sort_by(|a, b| b.1.cmp(&a.1));
    sorted_terms.truncate(max_terms);

    let vocab: HashMap<String, usize> = sorted_terms
        .into_iter()
        .enumerate()
        .map(|(idx, (term, _))| (term, idx))
        .collect();

    // Compute TF-IDF vectors
    let doc_vectors: Vec<Vec<TermWeight>> = term_sets
        .into_iter()
        .map(|term_counts| {
            let total_terms: usize = term_counts.values().sum();
            let mut weights: Vec<TermWeight> = term_counts
                .into_iter()
                .filter_map(|(term, tf)| {
                    vocab.get(&term).map(|&term_index| {
                        let tf_norm = tf as f32 / total_terms.max(1) as f32;
                        let df = 1.0f32; // smoothed — all terms in vocab have df ≥ 1
                        let idf = ((n_docs + 1.0) / (df + 1.0)).ln() + 1.0;
                        TermWeight {
                            term_index: term_index as u32,
                            weight: tf_norm * idf,
                        }
                    })
                })
                .collect();
            // L2-normalize
            let norm: f32 = weights
                .iter()
                .map(|w| w.weight * w.weight)
                .sum::<f32>()
                .sqrt();
            if norm > 0.0 {
                for w in &mut weights {
                    w.weight /= norm;
                }
            }
            weights.sort_by_key(|w| w.term_index);
            weights
        })
        .collect();

    (vocab, doc_vectors)
}

/// Compute cosine similarity between two sparse term-weight vectors.
pub fn cosine_similarity(a: &[TermWeight], b: &[TermWeight]) -> f32 {
    let mut dot = 0.0f32;
    let mut ai = 0usize;
    let mut bi = 0usize;

    while ai < a.len() && bi < b.len() {
        match a[ai].term_index.cmp(&b[bi].term_index) {
            std::cmp::Ordering::Less => ai += 1,
            std::cmp::Ordering::Greater => bi += 1,
            std::cmp::Ordering::Equal => {
                dot += a[ai].weight * b[bi].weight;
                ai += 1;
                bi += 1;
            }
        }
    }
    // Vectors are L2-normalized, so dot product is cosine similarity
    dot.clamp(-1.0, 1.0)
}

/// Serialize a sparse vector to bytes.
pub fn serialize_vector(entries: &[TermWeight]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(entries.len() * 8);
    for entry in entries {
        buf.extend_from_slice(&entry.term_index.to_le_bytes());
        buf.extend_from_slice(&entry.weight.to_le_bytes());
    }
    buf
}

/// Deserialize a sparse vector from bytes.
pub fn deserialize_vector(data: &[u8]) -> Vec<TermWeight> {
    let mut entries = Vec::with_capacity(data.len() / 8);
    for chunk in data.chunks_exact(8) {
        let term_index = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let weight = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        entries.push(TermWeight { term_index, weight });
    }
    entries
}

/// Tokenize text into lowercase alphanumeric tokens (min 2 chars).
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tfidf_yields_distinct_vectors() {
        let docs = [
            "docker containers are great for deployment",
            "linux kernel memory management is complex",
            "docker build caching speeds up docker image creation",
        ];
        let texts: Vec<&str> = docs.iter().map(|d| *d).collect();
        let (_vocab, vectors) = fit_transform(&texts, 64);

        assert_eq!(vectors.len(), 3);
        let sim_01 = cosine_similarity(&vectors[0], &vectors[1]);
        let sim_02 = cosine_similarity(&vectors[0], &vectors[2]);
        // docker docs (0 and 2) should be more similar to each other
        assert!(
            sim_02 > sim_01,
            "docker docs should be more similar: {sim_02} > {sim_01}"
        );
    }

    #[test]
    fn serialize_roundtrip() {
        let entries = vec![
            TermWeight {
                term_index: 0,
                weight: 0.5,
            },
            TermWeight {
                term_index: 7,
                weight: 0.866,
            },
        ];
        let data = serialize_vector(&entries);
        let restored = deserialize_vector(&data);
        assert_eq!(entries, restored);
    }

    #[test]
    fn empty_vectors_have_zero_similarity() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(
            cosine_similarity(
                &[TermWeight {
                    term_index: 1,
                    weight: 1.0
                }],
                &[]
            ),
            0.0
        );
    }
}

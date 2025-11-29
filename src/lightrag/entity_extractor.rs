use std::collections::HashSet;

use super::chunker::Chunk;

/// Named entity found in text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entity {
    /// Original surface form
    pub name: String,
    /// Lowercased normalized form (for matching)
    pub normalized: String,
    /// Chunk where the entity was found
    pub chunk_id: uuid::Uuid,
    /// Word position inside chunk
    pub position: usize,
}

/// Relation between entities (co-occurrence for now).
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub weight: f32,
}

/// Light-weight entity extractor with heuristics (no network calls).
#[derive(Debug, Default, Clone)]
pub struct EntityExtractor {
    stopwords: HashSet<String>,
}

impl EntityExtractor {
    pub fn new() -> Self {
        let mut stopwords = HashSet::new();
        for w in [
            "and", "or", "but", "the", "a", "an", "of", "in", "on", "for", "to", "with", "что",
            "как", "это", "или", "для", "при", "про", "без", "под", "над",
        ] {
            stopwords.insert(w.to_string());
        }
        Self { stopwords }
    }

    /// Extract entities and relations from a chunk.
    pub fn extract(&self, chunk: &Chunk) -> (Vec<Entity>, Vec<Relation>) {
        let mut entities = Vec::new();
        let mut seen = HashSet::new();
        let mut relations = Vec::new();

        for (idx, raw_token) in chunk.text.split_whitespace().enumerate() {
            let token =
                raw_token.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '#');
            if token.len() < 3 {
                continue;
            }
            let normalized = token.to_lowercase();
            if self.stopwords.contains(&normalized) {
                continue;
            }

            // Heuristic: keep capitalized words, handles, hashtags, or tokens with digits.
            let is_candidate = token
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
                || token.contains('@')
                || token.contains('#')
                || token.chars().any(|c| c.is_numeric());

            if !is_candidate {
                continue;
            }

            if seen.insert(normalized.clone()) {
                entities.push(Entity {
                    name: token.to_string(),
                    normalized: normalized.clone(),
                    chunk_id: chunk.id,
                    position: idx,
                });
            }
        }

        // Build simple co-occurrence relations between neighboring entities
        for pair in entities.windows(2) {
            if let [a, b] = pair {
                relations.push(Relation {
                    from: a.normalized.clone(),
                    to: b.normalized.clone(),
                    relation_type: "co_occurs".to_string(),
                    weight: 1.0,
                });
            }
        }

        (entities, relations)
    }

    /// Extract just entity names from free text (used for queries).
    pub fn extract_keywords(&self, text: &str) -> Vec<String> {
        let dummy = Chunk::new(
            text.to_string(),
            0,
            text.split_whitespace().count(),
            "query",
        );
        let (entities, _) = self.extract(&dummy);
        entities
            .into_iter()
            .map(|e| e.normalized)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_entities_and_relations() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new(
            "Alice met Bob in Paris with @carol".to_string(),
            0,
            6,
            "test",
        );
        let (entities, relations) = extractor.extract(&chunk);

        assert!(entities.iter().any(|e| e.name.starts_with("Alice")));
        assert!(entities.iter().any(|e| e.name.starts_with("Bob")));
        assert!(entities.iter().any(|e| e.name.contains("@carol")));
        assert!(!relations.is_empty());
    }
}

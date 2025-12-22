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

    #[test]
    fn ignores_stopwords() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("The quick fox And the lazy Dog".to_string(), 0, 7, "test");
        let (entities, _) = extractor.extract(&chunk);

        // Stopwords are filtered by normalized form (lowercase)
        // "The", "And", "the" are all stopwords when normalized
        // Only non-stopword capitalized words should appear
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(!names.contains(&"The")); // Stopword
        assert!(!names.contains(&"And")); // Stopword
        assert!(names.contains(&"Dog")); // Not a stopword, capitalized
        assert!(!names.contains(&"quick")); // Not capitalized
        assert!(!names.contains(&"fox")); // Not capitalized
    }

    #[test]
    fn extracts_hashtags() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Check out #rust and #programming".to_string(), 0, 4, "test");
        let (entities, _) = extractor.extract(&chunk);

        assert!(entities.iter().any(|e| e.name.contains("#rust")));
        assert!(entities.iter().any(|e| e.name.contains("#programming")));
    }

    #[test]
    fn extracts_mentions() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Thanks @alice and @bob_dev".to_string(), 0, 4, "test");
        let (entities, _) = extractor.extract(&chunk);

        assert!(entities.iter().any(|e| e.name.contains("@alice")));
        assert!(entities.iter().any(|e| e.name.contains("@bob_dev")));
    }

    #[test]
    fn extracts_tokens_with_numbers() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Version v2.0 released on 2024".to_string(), 0, 4, "test");
        let (entities, _) = extractor.extract(&chunk);

        assert!(entities.iter().any(|e| e.name.contains("2024")));
    }

    #[test]
    fn ignores_short_tokens() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("A B C ab AB is OK".to_string(), 0, 7, "test");
        let (entities, _) = extractor.extract(&chunk);

        // Tokens shorter than 3 chars should be ignored
        assert!(!entities.iter().any(|e| e.name == "A"));
        assert!(!entities.iter().any(|e| e.name == "AB"));
    }

    #[test]
    fn creates_co_occurrence_relations() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Alice Bob Carol".to_string(), 0, 3, "test");
        let (entities, relations) = extractor.extract(&chunk);

        assert_eq!(entities.len(), 3);
        assert_eq!(relations.len(), 2); // Alice-Bob, Bob-Carol

        assert!(relations.iter().any(|r| r.from == "alice" && r.to == "bob"));
        assert!(relations.iter().any(|r| r.from == "bob" && r.to == "carol"));
        assert!(relations.iter().all(|r| r.relation_type == "co_occurs"));
        assert!(relations
            .iter()
            .all(|r| (r.weight - 1.0).abs() < f32::EPSILON));
    }

    #[test]
    fn deduplicates_entities() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Alice met Alice again Alice".to_string(), 0, 5, "test");
        let (entities, _) = extractor.extract(&chunk);

        // Should only have one Alice
        let alice_count = entities.iter().filter(|e| e.name == "Alice").count();
        assert_eq!(alice_count, 1);
    }

    #[test]
    fn extract_keywords_returns_unique_normalized() {
        let extractor = EntityExtractor::new();
        let keywords = extractor.extract_keywords("Alice met Bob Alice and @charlie");

        assert!(keywords.contains(&"alice".to_string()));
        assert!(keywords.contains(&"bob".to_string()));
        assert!(keywords.iter().any(|k| k.contains("charlie")));
    }

    #[test]
    fn empty_text_returns_empty() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new(String::new(), 0, 0, "test");
        let (entities, relations) = extractor.extract(&chunk);

        assert!(entities.is_empty());
        assert!(relations.is_empty());
    }

    #[test]
    fn entity_stores_position() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("First Second Third".to_string(), 0, 3, "test");
        let (entities, _) = extractor.extract(&chunk);

        let first = entities.iter().find(|e| e.name == "First").unwrap();
        let second = entities.iter().find(|e| e.name == "Second").unwrap();
        let third = entities.iter().find(|e| e.name == "Third").unwrap();

        assert_eq!(first.position, 0);
        assert_eq!(second.position, 1);
        assert_eq!(third.position, 2);
    }

    #[test]
    fn entity_stores_chunk_id() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Alice".to_string(), 0, 1, "test");
        let (entities, _) = extractor.extract(&chunk);

        assert_eq!(entities[0].chunk_id, chunk.id);
    }

    #[test]
    fn russian_stopwords_filtered() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("что как это или для".to_string(), 0, 5, "test");
        let (entities, _) = extractor.extract(&chunk);

        // Russian stopwords should be filtered out (none are capitalized)
        assert!(entities.is_empty());
    }

    #[test]
    fn entity_struct_debug() {
        let entity = Entity {
            name: "Test".to_string(),
            normalized: "test".to_string(),
            chunk_id: uuid::Uuid::new_v4(),
            position: 0,
        };
        
        let debug_str = format!("{:?}", entity);
        assert!(debug_str.contains("Entity"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn entity_struct_clone() {
        let entity = Entity {
            name: "Original".to_string(),
            normalized: "original".to_string(),
            chunk_id: uuid::Uuid::new_v4(),
            position: 5,
        };
        
        let cloned = entity.clone();
        assert_eq!(entity.name, cloned.name);
        assert_eq!(entity.position, cloned.position);
        assert_eq!(entity.chunk_id, cloned.chunk_id);
    }

    #[test]
    fn entity_struct_equality() {
        let chunk_id = uuid::Uuid::new_v4();
        let entity1 = Entity {
            name: "Test".to_string(),
            normalized: "test".to_string(),
            chunk_id,
            position: 0,
        };
        let entity2 = Entity {
            name: "Test".to_string(),
            normalized: "test".to_string(),
            chunk_id,
            position: 0,
        };
        
        assert_eq!(entity1, entity2);
    }

    #[test]
    fn entity_struct_hash() {
        use std::collections::HashSet;
        
        let entity = Entity {
            name: "Test".to_string(),
            normalized: "test".to_string(),
            chunk_id: uuid::Uuid::new_v4(),
            position: 0,
        };
        
        let mut set = HashSet::new();
        set.insert(entity.clone());
        assert!(set.contains(&entity));
    }

    #[test]
    fn relation_struct_debug() {
        let relation = Relation {
            from: "alice".to_string(),
            to: "bob".to_string(),
            relation_type: "co_occurs".to_string(),
            weight: 1.0,
        };
        
        let debug_str = format!("{:?}", relation);
        assert!(debug_str.contains("Relation"));
        assert!(debug_str.contains("alice"));
    }

    #[test]
    fn relation_struct_clone() {
        let relation = Relation {
            from: "a".to_string(),
            to: "b".to_string(),
            relation_type: "rel".to_string(),
            weight: 2.5,
        };
        
        let cloned = relation.clone();
        assert_eq!(relation.from, cloned.from);
        assert_eq!(relation.weight, cloned.weight);
    }

    #[test]
    fn relation_struct_equality() {
        let rel1 = Relation {
            from: "a".to_string(),
            to: "b".to_string(),
            relation_type: "co_occurs".to_string(),
            weight: 1.0,
        };
        let rel2 = Relation {
            from: "a".to_string(),
            to: "b".to_string(),
            relation_type: "co_occurs".to_string(),
            weight: 1.0,
        };
        
        assert_eq!(rel1, rel2);
    }

    #[test]
    fn extractor_default() {
        let extractor = EntityExtractor::default();
        // Should work the same as new()
        let chunk = Chunk::new("Alice met Bob".to_string(), 0, 3, "test");
        let (entities, _) = extractor.extract(&chunk);
        
        assert!(!entities.is_empty());
    }

    #[test]
    fn extractor_clone() {
        let extractor = EntityExtractor::new();
        let cloned = extractor.clone();
        
        let chunk = Chunk::new("Alice met Bob".to_string(), 0, 3, "test");
        let (entities1, _) = extractor.extract(&chunk);
        let (entities2, _) = cloned.extract(&chunk);
        
        assert_eq!(entities1.len(), entities2.len());
    }

    #[test]
    fn extractor_debug() {
        let extractor = EntityExtractor::new();
        let debug_str = format!("{:?}", extractor);
        
        assert!(debug_str.contains("EntityExtractor"));
    }

    #[test]
    fn extract_keywords_empty_text() {
        let extractor = EntityExtractor::new();
        let keywords = extractor.extract_keywords("");
        
        assert!(keywords.is_empty());
    }

    #[test]
    fn extract_keywords_only_stopwords() {
        let extractor = EntityExtractor::new();
        let keywords = extractor.extract_keywords("the and or for");
        
        assert!(keywords.is_empty());
    }

    #[test]
    fn single_entity_no_relations() {
        let extractor = EntityExtractor::new();
        let chunk = Chunk::new("Alice".to_string(), 0, 1, "test");
        let (entities, relations) = extractor.extract(&chunk);
        
        assert_eq!(entities.len(), 1);
        assert!(relations.is_empty());
    }
}

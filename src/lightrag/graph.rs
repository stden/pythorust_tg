use std::collections::{HashMap, HashSet};

use super::entity_extractor::{Entity, Relation};
use uuid::Uuid;

/// Graph node representing an entity.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub occurrences: usize,
    pub chunks: HashSet<Uuid>,
}

/// Graph edge representing relationship between entities.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f32,
}

/// Lightweight knowledge graph (in-memory).
#[derive(Debug, Default, Clone)]
pub struct KnowledgeGraph {
    nodes: HashMap<String, Node>,
    edges: HashMap<(String, String), Edge>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entities(&mut self, entities: &[Entity]) {
        for entity in entities {
            let entry = self
                .nodes
                .entry(entity.normalized.clone())
                .or_insert_with(|| Node {
                    name: entity.name.clone(),
                    occurrences: 0,
                    chunks: HashSet::new(),
                });
            entry.occurrences += 1;
            entry.chunks.insert(entity.chunk_id);
        }
    }

    pub fn add_relations(&mut self, relations: &[Relation]) {
        for rel in relations {
            let key = ordered(rel.from.clone(), rel.to.clone());
            let edge = self.edges.entry(key).or_insert_with(|| Edge {
                from: rel.from.clone(),
                to: rel.to.clone(),
                relation: rel.relation_type.clone(),
                weight: 0.0,
            });
            edge.weight += rel.weight;
        }
    }

    /// Get related entities by weight, sorted descending.
    pub fn related_entities(&self, entity: &str, top_k: usize) -> Vec<(String, f32)> {
        let mut related: Vec<(String, f32)> = self
            .edges
            .iter()
            .filter_map(|((a, b), edge)| {
                if a == entity {
                    Some((b.clone(), edge.weight))
                } else if b == entity {
                    Some((a.clone(), edge.weight))
                } else {
                    None
                }
            })
            .collect();

        related.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        related.truncate(top_k);
        related
    }

    /// Collect neighbors for a set of entities.
    pub fn neighbors_for_entities(&self, entities: &[Entity], top_k: usize) -> Vec<String> {
        let mut scores: HashMap<String, f32> = HashMap::new();

        for entity in entities {
            for (other, weight) in self.related_entities(&entity.normalized, top_k) {
                *scores.entry(other).or_insert(0.0) += weight;
            }
        }

        let mut scored: Vec<(String, f32)> = scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        scored.into_iter().map(|(name, _)| name).collect()
    }
}

fn ordered(a: String, b: String) -> (String, String) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_entities_and_relations() {
        let mut graph = KnowledgeGraph::new();

        let chunk_id = Uuid::new_v4();
        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                normalized: "alice".to_string(),
                chunk_id,
                position: 0,
            },
            Entity {
                name: "Bob".to_string(),
                normalized: "bob".to_string(),
                chunk_id,
                position: 1,
            },
        ];

        let relations = vec![Relation {
            from: "alice".to_string(),
            to: "bob".to_string(),
            relation_type: "co_occurs".to_string(),
            weight: 1.0,
        }];

        graph.add_entities(&entities);
        graph.add_relations(&relations);

        let neighbors = graph.related_entities("alice", 5);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0, "bob");
    }
}

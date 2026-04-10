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

    #[test]
    fn empty_graph_returns_empty_relations() {
        let graph = KnowledgeGraph::new();
        let related = graph.related_entities("nonexistent", 5);
        assert!(related.is_empty());
    }

    #[test]
    fn graph_is_default_clone() {
        let graph1 = KnowledgeGraph::default();
        let graph2 = graph1.clone();

        assert_eq!(
            graph1.related_entities("any", 5).len(),
            graph2.related_entities("any", 5).len()
        );
    }

    #[test]
    fn entity_occurrences_incremented() {
        let mut graph = KnowledgeGraph::new();
        let chunk_id1 = Uuid::new_v4();
        let chunk_id2 = Uuid::new_v4();

        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                normalized: "alice".to_string(),
                chunk_id: chunk_id1,
                position: 0,
            },
            Entity {
                name: "Alice".to_string(),
                normalized: "alice".to_string(),
                chunk_id: chunk_id2,
                position: 0,
            },
        ];

        graph.add_entities(&entities);

        // Node occurrences should be 2
        assert_eq!(graph.nodes.get("alice").unwrap().occurrences, 2);
        // Should track both chunks
        assert_eq!(graph.nodes.get("alice").unwrap().chunks.len(), 2);
    }

    #[test]
    fn relation_weights_accumulate() {
        let mut graph = KnowledgeGraph::new();

        let relations = vec![
            Relation {
                from: "alice".to_string(),
                to: "bob".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 1.0,
            },
            Relation {
                from: "bob".to_string(),
                to: "alice".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 2.0,
            },
        ];

        graph.add_relations(&relations);

        // Both relations should be combined due to ordered() function
        let related = graph.related_entities("alice", 5);
        assert_eq!(related.len(), 1);
        assert!((related[0].1 - 3.0).abs() < 0.001); // 1.0 + 2.0
    }

    #[test]
    fn related_entities_sorted_by_weight() {
        let mut graph = KnowledgeGraph::new();

        let relations = vec![
            Relation {
                from: "alice".to_string(),
                to: "bob".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 1.0,
            },
            Relation {
                from: "alice".to_string(),
                to: "charlie".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 3.0,
            },
            Relation {
                from: "alice".to_string(),
                to: "dave".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 2.0,
            },
        ];

        graph.add_relations(&relations);

        let related = graph.related_entities("alice", 5);
        assert_eq!(related.len(), 3);
        assert_eq!(related[0].0, "charlie"); // highest weight
        assert_eq!(related[1].0, "dave");
        assert_eq!(related[2].0, "bob");
    }

    #[test]
    fn related_entities_respects_top_k() {
        let mut graph = KnowledgeGraph::new();

        let relations = vec![
            Relation {
                from: "alice".to_string(),
                to: "bob".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 1.0,
            },
            Relation {
                from: "alice".to_string(),
                to: "charlie".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 3.0,
            },
            Relation {
                from: "alice".to_string(),
                to: "dave".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 2.0,
            },
        ];

        graph.add_relations(&relations);

        let related = graph.related_entities("alice", 2);
        assert_eq!(related.len(), 2);
        assert_eq!(related[0].0, "charlie");
        assert_eq!(related[1].0, "dave");
    }

    #[test]
    fn neighbors_for_entities_aggregates_scores() {
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

        let relations = vec![
            Relation {
                from: "alice".to_string(),
                to: "charlie".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 2.0,
            },
            Relation {
                from: "bob".to_string(),
                to: "charlie".to_string(),
                relation_type: "co_occurs".to_string(),
                weight: 3.0,
            },
        ];

        graph.add_relations(&relations);

        let neighbors = graph.neighbors_for_entities(&entities, 5);
        assert!(!neighbors.is_empty());
        assert!(neighbors.contains(&"charlie".to_string()));
    }

    #[test]
    fn neighbors_for_empty_entities() {
        let graph = KnowledgeGraph::new();
        let entities: Vec<Entity> = vec![];

        let neighbors = graph.neighbors_for_entities(&entities, 5);
        assert!(neighbors.is_empty());
    }

    #[test]
    fn ordered_function_sorts_strings() {
        let (a, b) = ordered("bob".to_string(), "alice".to_string());
        assert_eq!(a, "alice");
        assert_eq!(b, "bob");

        let (c, d) = ordered("alice".to_string(), "bob".to_string());
        assert_eq!(c, "alice");
        assert_eq!(d, "bob");
    }

    #[test]
    fn node_struct_construction() {
        let node = Node {
            name: "Test".to_string(),
            occurrences: 5,
            chunks: HashSet::from([Uuid::new_v4()]),
        };

        assert_eq!(node.name, "Test");
        assert_eq!(node.occurrences, 5);
        assert_eq!(node.chunks.len(), 1);
    }

    #[test]
    fn edge_struct_construction() {
        let edge = Edge {
            from: "alice".to_string(),
            to: "bob".to_string(),
            relation: "co_occurs".to_string(),
            weight: 1.5,
        };

        assert_eq!(edge.from, "alice");
        assert_eq!(edge.to, "bob");
        assert_eq!(edge.relation, "co_occurs");
        assert!((edge.weight - 1.5).abs() < 0.001);
    }

    #[test]
    fn node_clone() {
        let node = Node {
            name: "Test".to_string(),
            occurrences: 3,
            chunks: HashSet::from([Uuid::new_v4()]),
        };

        let cloned = node.clone();
        assert_eq!(node.name, cloned.name);
        assert_eq!(node.occurrences, cloned.occurrences);
    }

    #[test]
    fn edge_clone() {
        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            relation: "rel".to_string(),
            weight: 2.0,
        };

        let cloned = edge.clone();
        assert_eq!(edge.from, cloned.from);
        assert_eq!(edge.weight, cloned.weight);
    }
}

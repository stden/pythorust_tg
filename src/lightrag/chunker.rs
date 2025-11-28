use uuid::Uuid;

/// Text chunk produced by the chunker.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Unique chunk id
    pub id: Uuid,
    /// Raw text of the chunk
    pub text: String,
    /// Word index of the first token (for reference)
    pub start: usize,
    /// Word index after the last token (for reference)
    pub end: usize,
    /// Optional source label (chat, document, etc.)
    pub source: String,
}

impl Chunk {
    pub fn new(text: String, start: usize, end: usize, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            text,
            start,
            end,
            source: source.into(),
        }
    }
}

/// Chunking strategy.
#[derive(Debug, Clone, Copy)]
pub enum ChunkingStrategy {
    /// Split by words with overlap (default)
    Words,
}

/// Simple chunker with word-level overlap.
#[derive(Debug, Clone)]
pub struct Chunker {
    size: usize,
    overlap: usize,
    strategy: ChunkingStrategy,
}

impl Chunker {
    /// Create a new chunker.
    pub fn new(size: usize, overlap: usize) -> Self {
        Self {
            size: size.max(1),
            overlap: overlap.min(size.saturating_sub(1)),
            strategy: ChunkingStrategy::Words,
        }
    }

    /// Create with custom strategy.
    pub fn with_strategy(size: usize, overlap: usize, strategy: ChunkingStrategy) -> Self {
        Self {
            size: size.max(1),
            overlap: overlap.min(size.saturating_sub(1)),
            strategy,
        }
    }

    /// Split text into overlapping chunks.
    pub fn chunk(&self, text: &str, source: impl Into<String>) -> Vec<Chunk> {
        match self.strategy {
            ChunkingStrategy::Words => self.chunk_words(text, source),
        }
    }

    fn chunk_words(&self, text: &str, source: impl Into<String>) -> Vec<Chunk> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        let step = self.size.saturating_sub(self.overlap).max(1);
        let mut chunks = Vec::new();
        let mut idx = 0;
        let source = source.into();

        while idx < words.len() {
            let end = (idx + self.size).min(words.len());
            let chunk_text = words[idx..end].join(" ");
            chunks.push(Chunk::new(chunk_text, idx, end, source.clone()));

            if end == words.len() {
                break;
            }
            idx += step;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunker_respects_overlap() {
        let chunker = Chunker::new(4, 1);
        let text = "one two three four five six seven";
        let chunks = chunker.chunk(text, "test");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "one two three four");
        assert_eq!(chunks[1].text, "four five six seven");
        assert_eq!(chunks[0].end - chunks[0].start, 4);
    }

    #[test]
    fn chunker_empty_text_returns_empty() {
        let chunker = Chunker::new(4, 1);
        let chunks = chunker.chunk("", "test");
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunker_whitespace_only_returns_empty() {
        let chunker = Chunker::new(4, 1);
        let chunks = chunker.chunk("   \t\n  ", "test");
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunker_single_word() {
        let chunker = Chunker::new(4, 1);
        let chunks = chunker.chunk("hello", "test");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "hello");
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 1);
    }

    #[test]
    fn chunker_exact_size_text() {
        let chunker = Chunker::new(3, 1);
        let chunks = chunker.chunk("one two three", "test");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "one two three");
    }

    #[test]
    fn chunker_no_overlap() {
        let chunker = Chunker::new(2, 0);
        let chunks = chunker.chunk("a b c d e f", "test");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "a b");
        assert_eq!(chunks[1].text, "c d");
        assert_eq!(chunks[2].text, "e f");
    }

    #[test]
    fn chunker_large_overlap() {
        // Overlap larger than size should be clamped
        let chunker = Chunker::new(3, 10);
        let chunks = chunker.chunk("a b c d e f g", "test");
        // With size=3 and overlap clamped to 2, step=1
        assert!(chunks.len() > 1);
    }

    #[test]
    fn chunker_zero_size_uses_minimum() {
        let chunker = Chunker::new(0, 0);
        let chunks = chunker.chunk("word", "test");
        // Size 0 should become 1
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_has_unique_id() {
        let c1 = Chunk::new("text1".into(), 0, 1, "src");
        let c2 = Chunk::new("text2".into(), 0, 1, "src");
        assert_ne!(c1.id, c2.id);
    }

    #[test]
    fn chunk_stores_source() {
        let chunk = Chunk::new("text".into(), 0, 1, "my_source");
        assert_eq!(chunk.source, "my_source");
    }

    #[test]
    fn chunker_with_strategy() {
        let chunker = Chunker::with_strategy(3, 1, ChunkingStrategy::Words);
        let chunks = chunker.chunk("one two three four", "test");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn chunker_preserves_word_boundaries() {
        let chunker = Chunker::new(2, 0);
        let text = "hello world test case";
        let chunks = chunker.chunk(text, "test");

        // Each chunk should be valid words joined by space
        for chunk in &chunks {
            assert!(!chunk.text.starts_with(' '));
            assert!(!chunk.text.ends_with(' '));
        }
    }
}

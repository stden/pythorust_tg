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
}

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use grammers_tl_types as tl;
use telegram_reader::lightrag::{Chunk, Chunker, EntityExtractor};
use telegram_reader::reactions::extract_reactions;

fn chunker_benchmark(c: &mut Criterion) {
    let chunker = Chunker::new(64, 8);
    let text = "Rust async Telegram data processing retrieval chunk overlap".repeat(64);

    c.bench_function("chunker_split_long_text", |b| {
        b.iter(|| {
            let chunks = chunker.chunk(black_box(text.as_str()), "bench");
            black_box(chunks.len());
        });
    });
}

fn extractor_benchmark(c: &mut Criterion) {
    let extractor = EntityExtractor::new();
    let base_text = "Alice builds Rust bots for Telegram and graph pipelines \
        with Bob and Carol in Berlin while testing entity extraction speed."
        .repeat(32);
    let token_count = base_text.split_whitespace().count();
    let chunk = Chunk::new(base_text.clone(), 0, token_count, "bench_source");

    c.bench_function("entity_extractor_dense_text", |b| {
        b.iter(|| {
            let (entities, relations) = extractor.extract(black_box(&chunk));
            black_box((entities.len(), relations.len()));
        });
    });
}

fn reactions_benchmark(c: &mut Criterion) {
    let reactions = sample_reactions();

    c.bench_function("extract_reactions_hot_path", |b| {
        b.iter(|| {
            let (count, emojis) = extract_reactions(Some(black_box(&reactions)));
            black_box((count, emojis.len()));
        });
    });
}

fn sample_reactions() -> tl::enums::MessageReactions {
    let reactions = tl::types::MessageReactions {
        min: false,
        can_see_list: false,
        reactions_as_tags: false,
        results: vec![
            tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                chosen_order: None,
                reaction: tl::enums::Reaction::Emoji(tl::types::ReactionEmoji {
                    emoticon: "🔥".into(),
                }),
                count: 4,
            }),
            tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                chosen_order: Some(1),
                reaction: tl::enums::Reaction::CustomEmoji(tl::types::ReactionCustomEmoji {
                    document_id: 7,
                }),
                count: 2,
            }),
            tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                chosen_order: None,
                reaction: tl::enums::Reaction::Paid,
                count: 1,
            }),
        ],
        recent_reactions: None,
        top_reactors: None,
    };

    tl::enums::MessageReactions::Reactions(reactions)
}

criterion_group!(
    text_processing,
    chunker_benchmark,
    extractor_benchmark,
    reactions_benchmark
);
criterion_main!(text_processing);

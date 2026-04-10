use criterion::{black_box, criterion_group, criterion_main, Criterion};
use telegram_reader::lightrag::{Chunk, Chunker, EntityExtractor};
use telegram_reader::python::{
    build_message_text, contains_phone, detect_conversion, is_meaningful_message, sanitize_filename,
};

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

fn pyo3_text_benchmarks(c: &mut Criterion) {
    // Test data
    let filename = "Test @#$ Chat Name With Special Characters!!!";
    let phone_text = "+7 999 123-45-67 call me please";
    let no_phone_text = "Just a regular message without any phone number";
    let conversion_text = "Хочу купить этот товар!";
    let message_text = "Hello, this is a normal meaningful message";
    let command_text = "/start";

    c.bench_function("pyo3_sanitize_filename", |b| {
        b.iter(|| sanitize_filename(Some(black_box(filename)), "fallback", Some(50)));
    });

    c.bench_function("pyo3_contains_phone_positive", |b| {
        b.iter(|| contains_phone(black_box(phone_text)));
    });

    c.bench_function("pyo3_contains_phone_negative", |b| {
        b.iter(|| contains_phone(black_box(no_phone_text)));
    });

    c.bench_function("pyo3_detect_conversion", |b| {
        b.iter(|| detect_conversion(black_box(conversion_text)));
    });

    c.bench_function("pyo3_is_meaningful_message", |b| {
        b.iter(|| is_meaningful_message(black_box(message_text)));
    });

    c.bench_function("pyo3_is_meaningful_command", |b| {
        b.iter(|| is_meaningful_message(black_box(command_text)));
    });

    c.bench_function("pyo3_build_message_text_with_media", |b| {
        b.iter(|| build_message_text(black_box(Some("Hello world")), black_box(true), "[Media]"));
    });

    c.bench_function("pyo3_build_message_text_without_media", |b| {
        b.iter(|| build_message_text(black_box(Some("Hello world")), black_box(false), "[Media]"));
    });
}

fn pyo3_batch_benchmark(c: &mut Criterion) {
    // Simulate processing 1000 messages
    let messages: Vec<(&str, bool)> = (0..1000)
        .map(|i| {
            let text = if i % 3 == 0 {
                "Message with phone +7 999 123-45-67"
            } else if i % 5 == 0 {
                "/command"
            } else {
                "Regular message text"
            };
            let has_media = i % 4 == 0;
            (text, has_media)
        })
        .collect();

    c.bench_function("pyo3_batch_process_1000_messages", |b| {
        b.iter(|| {
            for (text, has_media) in &messages {
                let _ =
                    build_message_text(black_box(Some(*text)), black_box(*has_media), "[Media]");
                let _ = is_meaningful_message(black_box(*text));
                let _ = contains_phone(black_box(*text));
            }
        });
    });
}

criterion_group!(
    text_processing,
    chunker_benchmark,
    extractor_benchmark,
    pyo3_text_benchmarks,
    pyo3_batch_benchmark
);
criterion_main!(text_processing);

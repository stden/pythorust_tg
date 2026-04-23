use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use telegram_reader::analysis::models::AnalyzedMessage;

fn serialization_benchmark(c: &mut Criterion) {
    let msg = create_sample_message();

    c.bench_function("serialize_analyzed_message", |b| {
        b.iter(|| {
            serde_json::to_string(black_box(&msg)).unwrap();
        });
    });

    let json = serde_json::to_string(&msg).unwrap();
    c.bench_function("deserialize_analyzed_message", |b| {
        b.iter(|| {
            let _res: AnalyzedMessage = serde_json::from_str(black_box(&json)).unwrap();
        });
    });
}

fn vec_serialization_benchmark(c: &mut Criterion) {
    let mut msgs = Vec::new();
    for _ in 0..1000 {
        msgs.push(create_sample_message());
    }

    c.bench_function("serialize_1000_messages", |b| {
        b.iter(|| {
            serde_json::to_string(black_box(&msgs)).unwrap();
        });
    });

    let json = serde_json::to_string(&msgs).unwrap();
    c.bench_function("deserialize_1000_messages", |b| {
        b.iter(|| {
            let _res: Vec<AnalyzedMessage> = serde_json::from_str(black_box(&json)).unwrap();
        });
    });
}

fn create_sample_message() -> AnalyzedMessage {
    let mut msg = AnalyzedMessage::new(
        12345,
        987654321,
        "Super Chat".to_string(),
        111222333,
        "John Doe".to_string(),
        "This is a benchmark message with some reasonable length content to simulate real world usage.".to_string(),
        Utc::now(),
    );
    msg.reaction_count = 10;
    msg.reactions = vec!["👍".to_string(), "❤️".to_string()];
    msg.topics = vec![
        "benchmark".to_string(),
        "rust".to_string(),
        "json".to_string(),
    ];
    msg.sentiment = Some(0.85);
    // Embedding is skipped by default if None, let's add one to make it heavier
    msg.embedding = Some(vec![0.1; 1536]); // OpenAI dimension
    msg
}

criterion_group!(
    benches,
    serialization_benchmark,
    vec_serialization_benchmark
);
criterion_main!(benches);

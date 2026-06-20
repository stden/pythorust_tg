//! HTTP Benchmark Tool (аналог wrk)
//!
//! Простой и быстрый HTTP бенчмарк на Rust.
//!
//! Использование:
//!   http_bench <url> -c <connections> -d <duration> -t <threads>
//!
//! Примеры:
//!   cargo run --release --bin http_bench -- https://httpbin.org/get -c 10 -d 5
//!   cargo run --release --bin http_bench -- https://example.com -c 100 -d 30 -t 4

use anyhow::Result;
use clap::Parser;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;

#[derive(Parser)]
#[command(name = "http_bench")]
#[command(about = "HTTP бенчмарк утилита (аналог wrk)", long_about = None)]
struct Cli {
    /// URL для тестирования
    url: String,

    /// Количество параллельных соединений
    #[arg(short = 'c', long, default_value = "10")]
    connections: usize,

    /// Длительность теста в секундах
    #[arg(short = 'd', long, default_value = "10")]
    duration: u64,

    /// Количество потоков (tokio tasks)
    #[arg(short = 't', long, default_value = "2")]
    threads: usize,

    /// Таймаут запроса в секундах
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// HTTP метод (GET, POST, PUT, DELETE)
    #[arg(short = 'm', long, default_value = "GET")]
    method: String,

    /// Тело запроса (для POST/PUT)
    #[arg(short = 'b', long)]
    body: Option<String>,

    /// Заголовки в формате "Key: Value" (можно указать несколько раз)
    #[arg(short = 'H', long)]
    header: Vec<String>,

    /// Показывать прогресс
    #[arg(long)]
    progress: bool,
}

/// Статистика бенчмарка
#[derive(Default)]
struct Stats {
    requests: AtomicU64,
    success: AtomicU64,
    errors: AtomicU64,
    total_latency_us: AtomicU64,
    min_latency_us: AtomicU64,
    max_latency_us: AtomicU64,
    bytes_received: AtomicU64,
    status_2xx: AtomicU64,
    status_3xx: AtomicU64,
    status_4xx: AtomicU64,
    status_5xx: AtomicU64,
}

impl Stats {
    fn new() -> Self {
        Self {
            min_latency_us: AtomicU64::new(u64::MAX),
            ..Default::default()
        }
    }

    fn record_request(&self, latency_us: u64, bytes: u64, status: u16) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.success.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);

        // Update min latency
        let mut current = self.min_latency_us.load(Ordering::Relaxed);
        while latency_us < current {
            match self.min_latency_us.compare_exchange_weak(
                current,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }

        // Update max latency
        let mut current = self.max_latency_us.load(Ordering::Relaxed);
        while latency_us > current {
            match self.max_latency_us.compare_exchange_weak(
                current,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }

        // Status code distribution
        match status {
            200..=299 => self.status_2xx.fetch_add(1, Ordering::Relaxed),
            300..=399 => self.status_3xx.fetch_add(1, Ordering::Relaxed),
            400..=499 => self.status_4xx.fetch_add(1, Ordering::Relaxed),
            500..=599 => self.status_5xx.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    fn record_error(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// Выполнение одного HTTP запроса
async fn make_request(
    client: &Client,
    url: &str,
    method: &str,
    body: Option<&str>,
    request_timeout: Duration,
) -> Result<(u64, u64, u16)> {
    let start = Instant::now();

    let request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => {
            let mut req = client.post(url);
            if let Some(b) = body {
                req = req.body(b.to_string());
            }
            req
        }
        "PUT" => {
            let mut req = client.put(url);
            if let Some(b) = body {
                req = req.body(b.to_string());
            }
            req
        }
        "DELETE" => client.delete(url),
        "HEAD" => client.head(url),
        _ => client.get(url),
    };

    let response = timeout(request_timeout, request.send()).await??;
    let status = response.status().as_u16();
    let bytes = response.bytes().await?.len() as u64;

    let latency_us = start.elapsed().as_micros() as u64;

    Ok((latency_us, bytes, status))
}

/// Воркер, выполняющий запросы
#[allow(clippy::too_many_arguments)]
async fn worker(
    client: Client,
    url: String,
    method: String,
    body: Option<String>,
    stats: Arc<Stats>,
    semaphore: Arc<Semaphore>,
    running: Arc<AtomicUsize>,
    request_timeout: Duration,
) {
    while running.load(Ordering::Relaxed) == 1 {
        let _permit = semaphore.acquire().await.unwrap();

        match make_request(&client, &url, &method, body.as_deref(), request_timeout).await {
            Ok((latency, bytes, status)) => {
                stats.record_request(latency, bytes, status);
            }
            Err(_) => {
                stats.record_error();
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_latency(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    } else if us >= 1_000 {
        format!("{:.2}ms", us as f64 / 1_000.0)
    } else {
        format!("{}us", us)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Собираем клиент с заголовками
    let mut client_builder = Client::builder()
        .pool_max_idle_per_host(cli.connections)
        .timeout(Duration::from_secs(cli.timeout))
        .user_agent("http_bench/1.0");

    // Парсим заголовки
    let mut headers = reqwest::header::HeaderMap::new();
    for h in &cli.header {
        if let Some((key, value)) = h.split_once(':') {
            if let (Ok(k), Ok(v)) = (
                reqwest::header::HeaderName::try_from(key.trim()),
                reqwest::header::HeaderValue::from_str(value.trim()),
            ) {
                headers.insert(k, v);
            }
        }
    }
    client_builder = client_builder.default_headers(headers);

    let client = client_builder.build()?;

    // Проверяем доступность URL
    println!("🔍 Проверяю доступность {}...", cli.url);
    match client.get(&cli.url).send().await {
        Ok(resp) => {
            println!("✅ URL доступен (HTTP {})\n", resp.status());
        }
        Err(e) => {
            eprintln!("❌ Ошибка подключения: {}", e);
            std::process::exit(1);
        }
    }

    let stats = Arc::new(Stats::new());
    let semaphore = Arc::new(Semaphore::new(cli.connections));
    let running = Arc::new(AtomicUsize::new(1));

    println!("🚀 Запуск бенчмарка:");
    println!("   URL: {}", cli.url);
    println!("   Метод: {}", cli.method);
    println!("   Соединений: {}", cli.connections);
    println!("   Потоков: {}", cli.threads);
    println!("   Длительность: {}s\n", cli.duration);

    let start = Instant::now();

    // Запускаем воркеры
    let mut handles = Vec::new();
    for _ in 0..cli.threads {
        let client = client.clone();
        let url = cli.url.clone();
        let method = cli.method.clone();
        let body = cli.body.clone();
        let stats = Arc::clone(&stats);
        let semaphore = Arc::clone(&semaphore);
        let running = Arc::clone(&running);
        let request_timeout = Duration::from_secs(cli.timeout);

        handles.push(tokio::spawn(async move {
            worker(
                client,
                url,
                method,
                body,
                stats,
                semaphore,
                running,
                request_timeout,
            )
            .await;
        }));
    }

    // Ждём указанное время
    if cli.progress {
        for i in 0..cli.duration {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let reqs = stats.requests.load(Ordering::Relaxed);
            let errs = stats.errors.load(Ordering::Relaxed);
            println!(
                "  [{}/{}s] Запросов: {}, Ошибок: {}",
                i + 1,
                cli.duration,
                reqs,
                errs
            );
        }
    } else {
        tokio::time::sleep(Duration::from_secs(cli.duration)).await;
    }

    // Останавливаем воркеры
    running.store(0, Ordering::Relaxed);

    // Даём воркерам завершиться
    tokio::time::sleep(Duration::from_millis(100)).await;

    let elapsed = start.elapsed();

    // Собираем статистику
    let total_requests = stats.requests.load(Ordering::Relaxed);
    let success = stats.success.load(Ordering::Relaxed);
    let errors = stats.errors.load(Ordering::Relaxed);
    let total_latency = stats.total_latency_us.load(Ordering::Relaxed);
    let min_latency = stats.min_latency_us.load(Ordering::Relaxed);
    let max_latency = stats.max_latency_us.load(Ordering::Relaxed);
    let bytes = stats.bytes_received.load(Ordering::Relaxed);

    let rps = total_requests as f64 / elapsed.as_secs_f64();
    let avg_latency = total_latency.checked_div(success).unwrap_or(0);
    let throughput = bytes as f64 / elapsed.as_secs_f64();

    // Выводим результаты
    println!("\n📊 Результаты бенчмарка:");
    println!("═══════════════════════════════════════════");
    println!("  Время теста:     {:.2}s", elapsed.as_secs_f64());
    println!("  Всего запросов:  {}", total_requests);
    println!(
        "  Успешных:        {} ({:.1}%)",
        success,
        if total_requests > 0 {
            success as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  Ошибок:          {} ({:.1}%)",
        errors,
        if total_requests > 0 {
            errors as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("───────────────────────────────────────────");
    println!("  Requests/sec:    {:.2}", rps);
    println!("  Throughput:      {}/s", format_bytes(throughput as u64));
    println!("  Transfer:        {}", format_bytes(bytes));
    println!("───────────────────────────────────────────");
    println!("  Latency:");
    println!("    Avg:           {}", format_latency(avg_latency));
    if min_latency < u64::MAX {
        println!("    Min:           {}", format_latency(min_latency));
    }
    println!("    Max:           {}", format_latency(max_latency));
    println!("───────────────────────────────────────────");
    println!("  HTTP Status Codes:");
    println!(
        "    2xx:           {}",
        stats.status_2xx.load(Ordering::Relaxed)
    );
    println!(
        "    3xx:           {}",
        stats.status_3xx.load(Ordering::Relaxed)
    );
    println!(
        "    4xx:           {}",
        stats.status_4xx.load(Ordering::Relaxed)
    );
    println!(
        "    5xx:           {}",
        stats.status_5xx.load(Ordering::Relaxed)
    );
    println!("═══════════════════════════════════════════");

    Ok(())
}

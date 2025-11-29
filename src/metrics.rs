//! Prometheus metrics for Telegram Reader CLI.
//!
//! Exposes:
//! - `telegram_reader_command_duration_seconds` (histogram)
//! - `telegram_reader_command_total` (counter with status)
//! - `telegram_reader_command_inflight` (gauge)
//! - process metrics via `process` collector

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use prometheus::process_collector::ProcessCollector;
use prometheus::{
    default_registry, register_histogram_vec, register_int_counter_vec, register_int_gauge_vec,
    Encoder, HistogramVec, IntCounterVec, IntGaugeVec, TextEncoder,
};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

static PROCESS_COLLECTOR: Lazy<()> = Lazy::new(|| {
    if let Err(err) = default_registry().register(Box::new(ProcessCollector::for_self())) {
        warn!("Failed to register process collector: {}", err);
    }
});

static COMMAND_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    // Exponential buckets from 50ms up to ~3 minutes.
    let buckets =
        prometheus::exponential_buckets(0.05, 2.0, 14).expect("failed to create histogram buckets");
    register_histogram_vec!(
        "telegram_reader_command_duration_seconds",
        "CLI command duration in seconds",
        &["command"],
        buckets
    )
    .expect("failed to register command duration histogram")
});

static COMMAND_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telegram_reader_command_total",
        "Total command executions by status",
        &["command", "status"]
    )
    .expect("failed to register command counter")
});

static COMMAND_INFLIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telegram_reader_command_inflight",
        "Number of in-flight commands",
        &["command"]
    )
    .expect("failed to register inflight gauge")
});

/// Ensure collectors are registered.
fn init_collectors() {
    Lazy::force(&PROCESS_COLLECTOR);
    Lazy::force(&COMMAND_DURATION);
    Lazy::force(&COMMAND_TOTAL);
    Lazy::force(&COMMAND_INFLIGHT);
}

/// Increment inflight gauge for a command.
pub fn record_command_start(command: &'static str) {
    init_collectors();
    COMMAND_INFLIGHT.with_label_values(&[command]).inc();
}

/// Record command completion with duration and status.
pub fn record_command_result(command: &'static str, duration: Duration, success: bool) {
    init_collectors();
    COMMAND_INFLIGHT.with_label_values(&[command]).dec();
    COMMAND_DURATION
        .with_label_values(&[command])
        .observe(duration.as_secs_f64());
    COMMAND_TOTAL
        .with_label_values(&[command, if success { "ok" } else { "error" }])
        .inc();
}

async fn metrics_response() -> Result<Response<Full<Bytes>>, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", err);
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::from("encode error"))
            .unwrap());
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, encoder.format_type())
        .body(Full::from(buffer))
        .unwrap())
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    match req.uri().path() {
        "/metrics" => metrics_response().await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::new()))
            .unwrap()),
    }
}

async fn serve(addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "Prometheus metrics endpoint started");

    loop {
        let (stream, peer) = listener.accept().await?;
        let service = service_fn(handle_request);
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                warn!(?peer, "Metrics connection error: {}", err);
            }
        });
    }
}

/// Spawn the metrics HTTP endpoint on the given address.
pub fn spawn_metrics_server(addr: SocketAddr) {
    init_collectors();
    tokio::spawn(async move {
        if let Err(err) = serve(addr).await {
            error!(%addr, "Metrics server failed: {}", err);
        }
    });
}

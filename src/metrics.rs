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

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[test]
    fn records_successful_command_metrics() {
        let cmd = "test_command_metrics_success";

        record_command_start(cmd);
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd]).get(), 1);

        record_command_result(cmd, Duration::from_millis(120), true);

        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd]).get(), 0);
        assert_eq!(COMMAND_TOTAL.with_label_values(&[cmd, "ok"]).get(), 1);
        assert_eq!(
            COMMAND_DURATION
                .with_label_values(&[cmd])
                .get_sample_count(),
            1
        );
    }

    #[test]
    fn records_failed_command_metrics() {
        let cmd = "test_command_metrics_error";

        record_command_start(cmd);
        record_command_result(cmd, Duration::from_secs(2), false);

        assert_eq!(COMMAND_TOTAL.with_label_values(&[cmd, "error"]).get(), 1);
        assert_eq!(
            COMMAND_DURATION
                .with_label_values(&[cmd])
                .get_sample_count(),
            1
        );
    }

    #[tokio::test]
    async fn metrics_response_contains_registered_metrics() {
        let cmd = "test_metrics_response";
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_millis(10), true);

        let response = metrics_response().await.expect("metrics response");
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect metrics body")
            .to_bytes();
        let text = String::from_utf8(body_bytes.to_vec()).expect("utf-8 metrics body");
        assert!(text.contains("telegram_reader_command_total"));
        assert!(text.contains(cmd));
    }

    #[test]
    fn multiple_commands_tracked_separately() {
        let cmd1 = "test_cmd_separate_1";
        let cmd2 = "test_cmd_separate_2";

        record_command_start(cmd1);
        record_command_start(cmd2);
        
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd1]).get(), 1);
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd2]).get(), 1);

        record_command_result(cmd1, Duration::from_millis(50), true);
        
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd1]).get(), 0);
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd2]).get(), 1);

        record_command_result(cmd2, Duration::from_millis(100), false);
        
        assert_eq!(COMMAND_INFLIGHT.with_label_values(&[cmd2]).get(), 0);
    }

    #[test]
    fn command_duration_recorded() {
        let cmd = "test_duration_recording";
        
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_secs_f64(0.5), true);
        
        let count = COMMAND_DURATION.with_label_values(&[cmd]).get_sample_count();
        assert!(count >= 1);
        
        let sum = COMMAND_DURATION.with_label_values(&[cmd]).get_sample_sum();
        assert!(sum >= 0.5);
    }

    #[test]
    fn init_collectors_can_be_called_multiple_times() {
        init_collectors();
        init_collectors();
        init_collectors();
        // Should not panic
    }

    #[test]
    fn records_multiple_success_and_failure() {
        let cmd = "test_multi_status";
        
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_millis(10), true);
        
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_millis(20), true);
        
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_millis(30), false);
        
        assert!(COMMAND_TOTAL.with_label_values(&[cmd, "ok"]).get() >= 2);
        assert!(COMMAND_TOTAL.with_label_values(&[cmd, "error"]).get() >= 1);
    }

    #[tokio::test]
    async fn metrics_response_has_correct_content_type() {
        let response = metrics_response().await.expect("metrics response");
        
        let content_type = response.headers().get(hyper::header::CONTENT_TYPE);
        assert!(content_type.is_some());
        
        let ct_str = content_type.unwrap().to_str().unwrap();
        assert!(ct_str.contains("text/plain") || ct_str.contains("text/"));
    }

    #[tokio::test]
    async fn metrics_response_contains_duration_histogram() {
        let cmd = "test_histogram_check";
        record_command_start(cmd);
        record_command_result(cmd, Duration::from_millis(100), true);
        
        let response = metrics_response().await.expect("metrics response");
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body_bytes.to_vec()).unwrap();
        
        assert!(text.contains("telegram_reader_command_duration_seconds"));
    }

    #[tokio::test]
    async fn metrics_response_contains_inflight_gauge() {
        let response = metrics_response().await.expect("metrics response");
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body_bytes.to_vec()).unwrap();
        
        assert!(text.contains("telegram_reader_command_inflight"));
    }
}


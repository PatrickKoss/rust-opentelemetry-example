use actix_web_opentelemetry::{PrometheusMetricsHandler};
use dotenv::dotenv;
use opentelemetry::global;
use opentelemetry_sdk::export::metrics::aggregation;
use opentelemetry_sdk::metrics::{controllers, processors, selectors};
use std::env;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;

pub fn init_tracer() {
    dotenv().ok();
    let app_name = env::var("CARGO_BIN_NAME").unwrap_or_else(|_| "api".to_string());

    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint(std::env::var("JAEGER_AGENT_ENDPOINT").unwrap_or_else(|_| "localhost:6831".to_string()))
        .with_service_name(app_name.clone())
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to install OpenTelemetry tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("INFO"));
    let formatting_layer = BunyanFormattingLayer::new(app_name, std::io::stdout);
    let subscriber = Registry::default()
        .with(telemetry)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(env_filter);
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to install `tracing` subscriber.");
}

#[derive(Debug, Clone)]
pub struct Prometheus {
    metrics_handler: PrometheusMetricsHandler,
}

impl Prometheus {
    pub fn new() -> Self {
        let controller = controllers::basic(
            processors::factory(
                selectors::simple::histogram([0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 0.7, 1.0, 2.0]),
                aggregation::cumulative_temporality_selector(),
            ).with_memory(true),
        ).build();
        let prometheus_exporter = opentelemetry_prometheus::exporter(controller).init();
        let metrics_handler = PrometheusMetricsHandler::new(prometheus_exporter);

        Self {
            metrics_handler,
        }
    }

    pub fn metrics_handler(&self) -> PrometheusMetricsHandler {
        self.metrics_handler.clone()
    }
}

impl Default for Prometheus {
    fn default() -> Self {
        Self::new()
    }
}
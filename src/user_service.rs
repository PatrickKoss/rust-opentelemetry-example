use std::time::Instant;
use opentelemetry::{Context, global, KeyValue};
use opentelemetry::metrics::{Counter, Histogram};
use tracing::instrument;
use tokio::time::{Duration, sleep};

#[derive(Debug)]
pub struct UserService {
    user_repository: UserRepository,
    user_repository_success_total: Counter<u64>,
    user_repository_duration_seconds: Histogram<f64>,
}

impl UserService {
    pub fn new(user_repository: UserRepository) -> Self {
        let meter = global::meter("api");
        let user_repository_success_total = meter.u64_counter("user_repository_success_total")
            .with_description("Number of total user repository success")
            .init();
        let user_repository_duration_seconds = meter.f64_histogram("user_repository_duration_seconds")
            .with_description("UserRepository duration in seconds")
            .init();
        Self { user_repository, user_repository_success_total, user_repository_duration_seconds }
    }

    #[instrument(name = "UserService::validate")]
    pub async fn validate(&self) {
        sleep(Duration::from_millis(2)).await;
    }

    #[instrument(name = "UserService::create")]
    pub async fn create(&self) {
        let start_time = Instant::now();

        self.validate().await;
        self.user_repository.create().await;

        let cx = Context::current();
        let elapsed = start_time.elapsed();
        self.user_repository_success_total.add(&cx, 1, &[KeyValue::new("action", "create")]);
        let duration = elapsed.as_millis() as f64 / 1000_f64;
        self.user_repository_duration_seconds.record(&cx, duration, &[KeyValue::new("action", "create")]);
    }
}

#[derive(Debug)]
pub struct UserRepository {}

impl UserRepository {
    pub fn new() -> Self {
        Self { }
    }

    #[instrument(name = "UserRepository::begin")]
    pub async fn begin(&self) {
        sleep(Duration::from_millis(3)).await;
    }

    #[instrument(name = "UserRepository::commit")]
    pub async fn commit(&self) {
        sleep(Duration::from_millis(4)).await;
    }

    #[instrument(name = "UserRepository::create")]
    pub async fn create(&self) {
        self.begin().await;
        sleep(Duration::from_millis(1)).await;
        self.commit().await;
    }
}
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use anyhow::Result;
use futures_util::future::LocalBoxFuture;
use opentelemetry::{Context, global, KeyValue};
use opentelemetry::metrics::Counter;
use opentelemetry::metrics::Histogram;
use std::future::{ready, Ready};
use std::sync::Arc;
use std::time::Instant;

pub struct RequestMetadata {}

impl<S, B> Transform<S, ServiceRequest> for RequestMetadata
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestMetadataMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let meter = global::meter("api");

        let http_requests_total = meter.u64_counter("http_requests_total")
            .with_description("Number of total HTTP requests")
            .init();

        let http_requests_4xx_total = meter.u64_counter("http_requests_4xx_total")
            .with_description("Number of total 4xx HTTP requests")
            .init();

        let http_requests_5xx_total = meter.u64_counter("http_requests_5xx_total")
            .with_description("Number of total 5xx HTTP requests")
            .init();

        let http_requests_2xx_total = meter.u64_counter("http_requests_2xx_total")
            .with_description("Number of total 2xx HTTP requests")
            .init();

        let http_requests_duration_seconds = meter.f64_histogram("http_requests_duration_seconds")
            .with_description("HTTP request duration in seconds for all requests")
            .init();

        ready(Ok(RequestMetadataMiddleware {
            service,
            http_requests_total: Arc::new(http_requests_total),
            http_requests_duration_seconds: Arc::new(http_requests_duration_seconds),
            http_requests_2xx_total: Arc::new(http_requests_2xx_total),
            http_requests_4xx_total: Arc::new(http_requests_4xx_total),
            http_requests_5xx_total: Arc::new(http_requests_5xx_total),
        }))
    }
}

pub struct RequestMetadataMiddleware<S> {
    service: S,
    http_requests_4xx_total: Arc<Counter<u64>>,
    http_requests_5xx_total: Arc<Counter<u64>>,
    http_requests_2xx_total: Arc<Counter<u64>>,
    http_requests_total: Arc<Counter<u64>>,
    http_requests_duration_seconds: Arc<Histogram<f64>>,
}

impl<S, B> Service<ServiceRequest> for RequestMetadataMiddleware<S>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let fut = self.service.call(req);

        let http_requests_4xx_total = self.http_requests_4xx_total.clone();
        let http_requests_5xx_total = self.http_requests_5xx_total.clone();
        let http_requests_2xx_total = self.http_requests_2xx_total.clone();
        let http_requests_total = self.http_requests_total.clone();
        let http_requests_duration_seconds = self.http_requests_duration_seconds.clone();

        Box::pin(async move {
            let res = fut.await?;
            let elapsed = start_time.elapsed();

            let cx = Context::current();
            let status = res.response().status();
            let path = res.request().path().to_string();
            let method = res.request().method().to_string();
            let status_key_value = status.as_str().to_string();
            let key_value = &[KeyValue::new("path", path), KeyValue::new("method", method), KeyValue::new("status", status_key_value)];

            if status.is_client_error() {
                http_requests_4xx_total.add(&cx, 1, key_value);
            } else if status.is_server_error() {
                http_requests_5xx_total.add(&cx, 1, key_value);
            } else if status.is_success() {
                http_requests_2xx_total.add(&cx, 1, key_value);
            }

            http_requests_total.add(&cx, 1, &[]);
            let duration = elapsed.as_millis() as f64 / 1000_f64;
            http_requests_duration_seconds.record(&cx, duration, key_value);

            Ok(res)
        })
    }
}
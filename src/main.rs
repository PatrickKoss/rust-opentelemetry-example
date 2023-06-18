use std::env;

use actix_web::{App, get, Handler, HttpRequest, HttpResponse, HttpServer, middleware, post, Responder, web};
use actix_web_opentelemetry::RequestTracing;
use anyhow::Result;
use serde::{Deserialize, Serialize};

mod user_service;
mod telemetry;
mod request_metadata_middleware;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    pub message: String,
}

#[get("/healthz")]
#[tracing::instrument]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(Message {
        message: "healthy".to_string(),
    })
}

#[post("/users")]
#[tracing::instrument(name = "CreateUserUseCase")]
async fn users() -> HttpResponse {
    let user_repo = user_service::UserRepository::new();
    let user_service = user_service::UserService::new(user_repo);
    user_service.create().await;

    HttpResponse::Ok().json(Message {
        message: "healthy".to_string(),
    })
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().json(Message {
        message: "not found".to_string(),
    })
}

#[get("/metrics")]
async fn metrics(prometheus_data: web::Data<telemetry::Prometheus>, request: HttpRequest) -> impl Responder {
    prometheus_data.metrics_handler().call(request).await
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    telemetry::init_tracer();

    let prometheus = telemetry::Prometheus::new();
    let prometheus_data = web::Data::new(prometheus.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(prometheus_data.clone())
            .wrap(request_metadata_middleware::RequestMetadata {})
            .wrap(middleware::Logger::default())
            .wrap(RequestTracing::new())
            .service(health)
            .service(metrics)
            .service(users)
            .default_service(web::route().to(not_found))
    })
        .bind(format!("0.0.0.0:{}", port))?
        .run()
        .await.expect("failed to run server");

    Ok(())
}

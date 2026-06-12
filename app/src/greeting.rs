//! A singleton service plus its routes — the "service + controller" slice.

use std::sync::atomic::{AtomicU64, Ordering};

use rustspring::{
    axum::{extract::Path, routing::get, Json, Router},
    serde_json::{json, Value},
    Config, Inject,
};
use serde::Deserialize;

/// Registered once via `.manage(...)`; the counter proves every request
/// hits the same instance (a singleton, like a Spring `@Service`).
pub struct GreetingService {
    served: AtomicU64,
}

impl GreetingService {
    pub fn new() -> Self {
        Self {
            served: AtomicU64::new(0),
        }
    }

    pub fn greet(&self, template: &str, name: &str) -> (String, u64) {
        let count = self.served.fetch_add(1, Ordering::Relaxed) + 1;
        (template.replace("{name}", name), count)
    }
}

/// Custom config section, like `@ConfigurationProperties(prefix = "greeting")`.
/// Defined in `config/application.toml` under `[greeting]`.
#[derive(Deserialize)]
struct GreetingConfig {
    template: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/hello", get(|| async { "hello from rustspring" }))
        .route("/api/greet/{name}", get(greet))
}

async fn greet(
    Path(name): Path<String>,
    Inject(service): Inject<GreetingService>,
    config: Config,
) -> Json<Value> {
    let template = config
        .section::<GreetingConfig>("greeting")
        .map(|g| g.template)
        .unwrap_or_else(|_| "Hello, {name}!".to_string());

    let (message, served) = service.greet(&template, &name);
    Json(json!({
        "message": message,
        "profile": config.app.profile,
        "total_greetings_served": served,
    }))
}

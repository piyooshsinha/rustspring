//! Starter slice: a singleton service plus its routes.
//! Grow your app by adding modules like this one and merging their routers
//! in main.rs. See https://github.com/piyooshsinha/rustspring for the
//! database pool, transactions, #[derive(Component)], and React serving.

use std::sync::atomic::{AtomicU64, Ordering};

use rustspring::{
    axum::{extract::Path, routing::get, Json, Router},
    serde_json::{json, Value},
    Config, Inject,
};

pub struct HelloService {
    served: AtomicU64,
}

impl HelloService {
    pub fn new() -> Self {
        Self {
            served: AtomicU64::new(0),
        }
    }

    fn hello(&self, name: &str) -> (String, u64) {
        let count = self.served.fetch_add(1, Ordering::Relaxed) + 1;
        (format!("Hello, {name}! Welcome to {{project-name}}."), count)
    }
}

pub fn routes() -> Router {
    Router::new().route("/api/hello/{name}", get(hello))
}

async fn hello(
    Path(name): Path<String>,
    Inject(service): Inject<HelloService>,
    config: Config,
) -> Json<Value> {
    let (message, served) = service.hello(&name);
    Json(json!({
        "message": message,
        "profile": config.app.profile,
        "requests_served": served,
    }))
}

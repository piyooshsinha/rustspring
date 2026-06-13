//! # rustspring
//!
//! A Spring Boot-style skeleton for Rust web applications. You bring
//! handlers and services; the framework brings:
//!
//! - **Profiles & config** — `config/application.toml` + `application-{profile}.toml`
//!   + `APP_*` env overrides, selected by `APP_PROFILE` ([`config`])
//! - **Singletons / DI** — register components with [`Application::manage`],
//!   inject them with [`Inject`] ([`context`])
//! - **Connection pool** — configured from `[database]`, registered
//!   automatically, injected as `Inject<PgPool>` ([`db`])
//! - **Transactions** — [`db::transactional`] commits on `Ok`, rolls back on `Err`
//! - **Errors** — [`AppError`] turns `?` into proper HTTP responses ([`error`])
//! - **React support** — point `[static].dir` at your frontend build and the
//!   app serves it with SPA fallback
//!
//! ## Minimal application
//!
//! ```ignore
//! use axum::{routing::get, Router};
//! use rustspring::{Application, Inject};
//!
//! struct Greeter;
//! impl Greeter {
//!     fn greet(&self) -> &'static str { "hello from rustspring" }
//! }
//!
//! async fn hello(Inject(g): Inject<Greeter>) -> &'static str { g.greet() }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     Application::new()
//!         .manage(Greeter)
//!         .routes(Router::new().route("/api/hello", get(hello)))
//!         .run()
//!         .await
//! }
//! ```

// Lets the `::rustspring::` paths emitted by #[derive(Component)] resolve
// inside this crate's own tests.
extern crate self as rustspring;

pub mod app;
pub mod component;
pub mod config;
pub mod context;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub mod db;
pub mod error;

pub use app::Application;
pub use component::{Component, ComponentError, WiringError};
pub use config::{AppConfig, ConfigSource};
pub use context::{AppContext, Config, Inject};
pub use error::AppError;
// The derive macro shares the trait's name, like serde's Serialize.
pub use rustspring_macros::Component;

#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub use db::transactional;

// Re-export the stack so applications only depend on `rustspring`.
pub use axum;
pub use figment;
pub use serde_json;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub use sqlx;
pub use tokio;
pub use tracing;

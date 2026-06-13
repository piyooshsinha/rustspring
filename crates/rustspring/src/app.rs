//! The application bootstrapper — the `SpringApplication.run()` of the
//! framework. It loads profile-aware config, initializes logging, builds the
//! connection pool, wires managed singletons, mounts your routes, and serves
//! the React build as an SPA fallback.

use std::sync::Arc;

use axum::{Extension, Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::{
    component::{Component, ComponentError},
    config::ConfigSource,
    context::AppContext,
};

type DeferredConstructor = Box<dyn FnOnce(&mut AppContext) -> Result<(), ComponentError> + Send>;

pub struct Application {
    context: AppContext,
    constructors: Vec<DeferredConstructor>,
    router: Router,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    pub fn new() -> Self {
        Self {
            context: AppContext::default(),
            constructors: Vec::new(),
            router: Router::new(),
        }
    }

    /// Register a shared singleton component, retrievable in handlers via
    /// the `Inject<T>` extractor. The Spring analogue is declaring a `@Bean`.
    pub fn manage<T: Send + Sync + 'static>(mut self, component: T) -> Self {
        self.context.register(component);
        self
    }

    /// Register a `#[derive(Component)]` type for constructor injection —
    /// the `@Component` of the framework. Construction is deferred to
    /// startup, after config and the database pool are wired, and runs in
    /// registration order: list a component after its dependencies.
    pub fn component<T: Component>(mut self) -> Self {
        self.constructors.push(Box::new(|ctx| {
            let component = T::construct(ctx)?;
            ctx.register(component);
            Ok(())
        }));
        self
    }

    /// Mount application routes. Call multiple times to merge routers —
    /// each module of your app can contribute its own `Router`.
    pub fn routes(mut self, router: Router) -> Self {
        self.router = self.router.merge(router);
        self
    }

    /// Load config, wire everything, and serve until shutdown (Ctrl-C).
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        init_tracing();

        let config = ConfigSource::load()?;
        let profile = config.app.profile.clone();
        tracing::info!(profile, "starting application");

        #[cfg(feature = "postgres")]
        if let Some(db_cfg) = &config.app.database {
            let pool = crate::db::build_pool(db_cfg)?;
            tracing::info!(
                max_connections = db_cfg.max_connections,
                "database pool configured (lazy connect)"
            );
            self.context.register(pool);
        }

        let addr = format!("{}:{}", config.app.server.host, config.app.server.port);
        let static_dir = config.app.static_files.dir.clone();
        self.context.register(config);

        // Construct components now that config and the pool are available.
        // Fail fast on a missing dependency, like a Spring context refresh.
        for construct in self.constructors.drain(..) {
            construct(&mut self.context)?;
        }

        let mut router = self
            .router
            .route("/actuator/health", axum::routing::get(health))
            .layer(Extension(self.context))
            .layer(TraceLayer::new_for_http());

        // Serve the built frontend (e.g. React's `dist/`) for any route the
        // API didn't match, falling back to index.html for client-side routing.
        if let Some(dir) = static_dir {
            let dir = std::path::PathBuf::from(dir);
            if dir.is_dir() {
                let spa = tower_http::services::ServeDir::new(&dir)
                    .fallback(tower_http::services::ServeFile::new(dir.join("index.html")));
                router = router.fallback_service(spa);
                tracing::info!(dir = %dir.display(), "serving frontend assets");
            } else {
                tracing::warn!(
                    dir = %dir.display(),
                    "static dir not found — run the frontend build, or unset static.dir"
                );
            }
        }

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        tracing::info!("listening on http://{addr}");

        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("shutdown signal received");
            })
            .await?;

        Ok(())
    }
}

async fn health(Extension(ctx): Extension<AppContext>) -> Json<serde_json::Value> {
    let profile = ctx
        .get::<ConfigSource>()
        .map(|c: Arc<ConfigSource>| c.app.profile.clone())
        .unwrap_or_default();
    Json(json!({ "status": "UP", "profile": profile }))
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

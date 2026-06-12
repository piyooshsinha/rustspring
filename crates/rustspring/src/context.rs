//! The application context: a registry of shared singletons, playing the
//! role of Spring's `ApplicationContext` / DI container.
//!
//! Register a component once at startup with `Application::manage`, then pull
//! it into any handler with the [`Inject`] extractor — the moral equivalent of
//! `@Autowired`, except a missing component is caught the first time the
//! route runs (and everything is `Arc`-shared, so all "beans" are singletons).

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
    sync::Arc,
};

use axum::{extract::FromRequestParts, http::request::Parts, http::StatusCode};

use crate::config::ConfigSource;

#[derive(Clone, Default)]
pub struct AppContext {
    components: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl AppContext {
    pub fn register<T: Send + Sync + 'static>(&mut self, component: T) {
        self.components
            .insert(TypeId::of::<T>(), Arc::new(component));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.components
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|c| c.downcast::<T>().ok())
    }
}

/// Extractor that injects a managed singleton into a handler:
///
/// ```ignore
/// async fn hello(Inject(greeter): Inject<GreetingService>) -> String {
///     greeter.greet("world")
/// }
/// ```
pub struct Inject<T>(pub Arc<T>);

impl<T> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<S, T> FromRequestParts<S> for Inject<T>
where
    S: Send + Sync,
    T: Send + Sync + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts.extensions.get::<AppContext>().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "AppContext missing — did you start via rustspring::Application?".to_string(),
        ))?;

        ctx.get::<T>().map(Inject).ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "no component of type `{}` registered — call .manage(...) at startup",
                std::any::type_name::<T>()
            ),
        ))
    }
}

/// Extractor for the loaded configuration, like injecting `Environment`:
///
/// ```ignore
/// async fn info(Config(cfg): Config) -> String {
///     format!("profile: {}", cfg.app.profile)
/// }
/// ```
pub struct Config(pub Arc<ConfigSource>);

impl Deref for Config {
    type Target = ConfigSource;

    fn deref(&self) -> &ConfigSource {
        &self.0
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Config {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Inject::<ConfigSource>::from_request_parts(parts, state)
            .await
            .map(|Inject(cfg)| Config(cfg))
    }
}

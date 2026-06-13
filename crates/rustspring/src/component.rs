//! Constructor-based dependency injection — the `@Component` of rustspring.
//!
//! A [`Component`] knows how to construct itself from the [`AppContext`] by
//! resolving its dependencies. Derive it and register the type with
//! [`Application::component`](crate::Application::component); the framework
//! constructs it at startup, after config and the database pool are wired:
//!
//! ```ignore
//! #[derive(Component)]
//! struct UserService {
//!     pool: Arc<PgPool>,             // resolved from the context
//!     greeter: Arc<GreetingService>, // another registered component
//! }
//!
//! Application::new()
//!     .manage(GreetingService::new())
//!     .component::<UserService>()
//! ```
//!
//! Components are constructed in registration order, so list a component
//! after the things it depends on. A missing dependency fails startup with
//! an error naming both types — fail fast, like a Spring context refresh.

use crate::context::AppContext;

pub trait Component: Sized + Send + Sync + 'static {
    fn construct(ctx: &AppContext) -> Result<Self, ComponentError>;
}

#[derive(Debug, thiserror::Error)]
#[error(
    "failed to construct `{component}`: missing dependency `{dependency}` — \
     register it earlier with .manage(...) or .component::<...>() (components \
     are constructed in registration order)"
)]
pub struct ComponentError {
    pub component: &'static str,
    pub dependency: &'static str,
}

impl ComponentError {
    pub fn missing(component: &'static str, dependency: &'static str) -> Self {
        Self {
            component,
            dependency,
        }
    }
}

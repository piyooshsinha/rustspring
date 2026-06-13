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
//! Registration order does not matter: components are constructed with a
//! fixpoint pass that retries until everything resolves. A genuinely missing
//! dependency (or a cycle) fails startup with an error naming every stuck
//! component and what it was waiting for — fail fast, like a Spring context
//! refresh.

use crate::context::AppContext;

pub trait Component: Sized + Send + Sync + 'static {
    fn construct(ctx: &AppContext) -> Result<Self, ComponentError>;
}

#[derive(Debug, thiserror::Error)]
#[error("`{component}` is missing dependency `{dependency}`")]
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

/// Startup failure: one or more components could never be constructed,
/// because a dependency was never registered or the components form a cycle.
#[derive(Debug)]
pub struct WiringError {
    pub unresolved: Vec<ComponentError>,
}

impl std::fmt::Display for WiringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "failed to wire {} component(s) — register the missing dependencies \
             with .manage(...) or .component::<...>(), or break the cycle:",
            self.unresolved.len()
        )?;
        for err in &self.unresolved {
            writeln!(f, "  - {err}")?;
        }
        Ok(())
    }
}

impl std::error::Error for WiringError {}

/// A deferred component constructor, as queued by
/// [`Application::component`](crate::Application::component).
pub type DeferredConstructor = Box<dyn Fn(&mut AppContext) -> Result<(), ComponentError> + Send>;

/// Make the deferred constructor for a component type.
pub fn deferred<T: Component>() -> DeferredConstructor {
    Box::new(|ctx| {
        let component = T::construct(ctx)?;
        ctx.register(component);
        Ok(())
    })
}

/// Construct all pending components, in dependency order. Order of the input
/// does not matter: each pass constructs whatever is now resolvable and
/// retries the rest; if a pass makes no progress, the remainder can never
/// resolve and startup fails with every stuck component listed.
pub fn construct_all(
    ctx: &mut AppContext,
    constructors: Vec<DeferredConstructor>,
) -> Result<(), WiringError> {
    let mut pending = constructors;
    while !pending.is_empty() {
        let before = pending.len();
        let mut unresolved = Vec::new();
        let mut retry = Vec::new();
        for construct in pending {
            match construct(ctx) {
                Ok(()) => {}
                Err(err) => {
                    unresolved.push(err);
                    retry.push(construct);
                }
            }
        }
        if retry.len() == before {
            return Err(WiringError { unresolved });
        }
        pending = retry;
    }
    Ok(())
}

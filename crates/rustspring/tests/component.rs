//! Tests for #[derive(Component)] constructor injection.

use std::sync::{atomic::AtomicU64, Arc};

use rustspring::{AppContext, Component};

struct Pool {
    url: &'static str,
}

#[derive(Component)]
struct Repository {
    pool: Arc<Pool>,
}

#[derive(Component)]
struct Service {
    repo: Arc<Repository>,
    // Not an Arc -> plain state, initialized with Default::default().
    calls: AtomicU64,
}

#[derive(Component)]
struct NoDeps;

#[test]
fn dependencies_resolve_from_the_context() {
    let mut ctx = AppContext::default();
    ctx.register(Pool {
        url: "postgres://x",
    });

    let repo = Repository::construct(&ctx).expect("construct");
    assert_eq!(repo.pool.url, "postgres://x");
}

#[test]
fn components_can_depend_on_components() {
    let mut ctx = AppContext::default();
    ctx.register(Pool {
        url: "postgres://x",
    });
    ctx.register(Repository::construct(&ctx).expect("repo"));

    let service = Service::construct(&ctx).expect("service");
    assert_eq!(service.repo.pool.url, "postgres://x");
    assert_eq!(service.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn missing_dependency_names_both_types() {
    let Err(err) = Repository::construct(&AppContext::default()) else {
        panic!("construction should fail without Pool registered");
    };
    let msg = err.to_string();
    assert!(msg.contains("Repository"), "names the component: {msg}");
    assert!(msg.contains("Pool"), "names the dependency: {msg}");
    assert!(
        msg.contains("registration order"),
        "explains the fix: {msg}"
    );
}

#[test]
fn unit_structs_construct_without_dependencies() {
    assert!(NoDeps::construct(&AppContext::default()).is_ok());
}

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
}

#[test]
fn unit_structs_construct_without_dependencies() {
    assert!(NoDeps::construct(&AppContext::default()).is_ok());
}

#[test]
fn construction_order_is_resolved_automatically() {
    // Service -> Repository -> Pool, registered in the worst possible order.
    let mut ctx = AppContext::default();
    ctx.register(Pool {
        url: "postgres://x",
    });

    rustspring::component::construct_all(
        &mut ctx,
        vec![
            rustspring::component::deferred::<Service>(),
            rustspring::component::deferred::<Repository>(),
        ],
    )
    .expect("fixpoint pass resolves dependency order");

    let service = ctx.get::<Service>().expect("constructed");
    assert_eq!(service.repo.pool.url, "postgres://x");
}

#[test]
fn unresolvable_components_fail_startup_with_a_full_report() {
    // No Pool registered: Repository can never construct, and Service is
    // stuck behind it — both must appear in the report.
    let mut ctx = AppContext::default();
    let err = rustspring::component::construct_all(
        &mut ctx,
        vec![
            rustspring::component::deferred::<Service>(),
            rustspring::component::deferred::<Repository>(),
        ],
    )
    .expect_err("must fail without Pool");

    assert_eq!(err.unresolved.len(), 2);
    let msg = err.to_string();
    assert!(msg.contains("Repository"), "lists Repository: {msg}");
    assert!(msg.contains("Service"), "lists Service: {msg}");
    assert!(msg.contains("Pool"), "names the root cause: {msg}");
}

// A dependency cycle: neither can construct first. Startup must fail with
// both listed rather than spinning forever.
#[derive(Component)]
struct Chicken {
    _egg: Arc<Egg>,
}

#[derive(Component)]
struct Egg {
    _chicken: Arc<Chicken>,
}

#[test]
fn dependency_cycles_are_reported_not_looped() {
    let mut ctx = AppContext::default();
    let err = rustspring::component::construct_all(
        &mut ctx,
        vec![
            rustspring::component::deferred::<Chicken>(),
            rustspring::component::deferred::<Egg>(),
        ],
    )
    .expect_err("a cycle can never resolve");

    let msg = err.to_string();
    assert!(msg.contains("Chicken") && msg.contains("Egg"), "{msg}");
    assert!(msg.contains("cycle"), "hints at cycles: {msg}");
}

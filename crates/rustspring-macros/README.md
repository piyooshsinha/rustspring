# rustspring-macros

Derive macros for the [rustspring](https://crates.io/crates/rustspring)
framework. You normally don't depend on this crate directly — `rustspring`
re-exports everything you need.

## `#[derive(Component)]`

Constructor-based dependency injection, the `@Component` of rustspring. Each
`Arc<T>` field is resolved from the application context at startup; other
fields are initialized with `Default::default()`.

```rust,ignore
use rustspring::Component;

#[derive(Component)]
struct UserService {
    pool: Arc<PgPool>,             // injected from the context
    greeter: Arc<GreetingService>, // injected: another component
    cache: RwLock<Vec<User>>,      // plain state: Default::default()
}

Application::new()
    .manage(GreetingService::new())
    .component::<UserService>();    // constructed at startup, order-independent
```

See the [framework documentation](https://github.com/piyooshsinha/rustspring)
for details.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/piyooshsinha/rustspring/blob/main/LICENSE).

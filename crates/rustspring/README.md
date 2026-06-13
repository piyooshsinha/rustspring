# rustspring

A Spring Boot-style skeleton framework for Rust web applications. You write
handlers and services; rustspring handles the plumbing you'd otherwise wire by
hand in every project — profiles, configuration, singletons, connection pools,
transactions, error handling, and serving a React/SPA frontend.

Built on [axum](https://crates.io/crates/axum),
[sqlx](https://crates.io/crates/sqlx),
[figment](https://crates.io/crates/figment), and
[tokio](https://crates.io/crates/tokio).

| Spring Boot | rustspring |
|---|---|
| `application-{profile}.yml` + `spring.profiles.active` | `config/application-{profile}.toml` + `APP_PROFILE` |
| `@ConfigurationProperties(prefix = "x")` | `config.section::<MyConfig>("x")` |
| `@Bean` / singleton beans | `Application::manage(component)` |
| `@Component` + constructor injection | `#[derive(Component)]` + `Application::component::<T>()` |
| `@Autowired` | `Inject<T>` extractor in handlers |
| HikariCP pool | `[database]` config → managed pool (Postgres/MySQL/SQLite) |
| `@Transactional` | `transactional(&pool, \|tx\| ...)` |
| Actuator health | `GET /actuator/health` built in |
| Serving a React build | `[static].dir` → SPA serving with `index.html` fallback |

## Quick start

Scaffold a ready-to-run app (SQLite-backed, zero external services):

```sh
cargo install cargo-generate
cargo generate --git https://github.com/piyooshsinha/rustspring template --name myapp
cd myapp && cargo run
```

Or add the dependency directly:

```toml
[dependencies]
rustspring = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust
use rustspring::{Application, Inject, axum::{Router, routing::get}};

struct Greeter;
impl Greeter { fn hi(&self) -> &'static str { "hello from rustspring" } }

async fn hello(Inject(g): Inject<Greeter>) -> &'static str { g.hi() }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Application::new()
        .manage(Greeter)
        .routes(Router::new().route("/api/hello", get(hello)))
        .run()
        .await
}
```

## Database backends

Postgres is the default; MySQL and SQLite are one feature flag away. The
framework registers the pool type matching the `database.url` scheme.

```toml
rustspring = { version = "0.1", default-features = false, features = ["sqlite"] }
```

## Documentation

Full guide, examples, and the demo application:
<https://github.com/piyooshsinha/rustspring>

## License

Licensed under the [Apache License, Version 2.0](https://github.com/piyooshsinha/rustspring/blob/main/LICENSE).

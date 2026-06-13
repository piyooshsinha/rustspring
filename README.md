# rustspring

[![CI](https://github.com/piyooshsinha/rustspring/actions/workflows/ci.yml/badge.svg)](https://github.com/piyooshsinha/rustspring/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg)](https://www.rust-lang.org)

**A Spring Boot-style skeleton framework for Rust web applications.** You write
handlers and services; rustspring handles the plumbing you'd otherwise wire by
hand in every project — profiles, configuration, singletons, connection pools,
transactions, error handling, and serving a React frontend.

Just plug in and start building your application in Rust, rather than thinking
about managing and configuring.

| Spring Boot | rustspring |
|---|---|
| `application-{profile}.yml` + `spring.profiles.active` | `config/application-{profile}.toml` + `APP_PROFILE` env var |
| `@ConfigurationProperties(prefix = "x")` | `config.section::<MyConfig>("x")` |
| `@Bean` / singleton beans | `Application::manage(component)` |
| `@Component` + constructor injection | `#[derive(Component)]` + `Application::component::<T>()` |
| `@Autowired` | `Inject<T>` extractor in handlers |
| HikariCP connection pool | `[database]` config → managed `PgPool` (lazy connect) |
| `@Transactional` | `transactional(&pool, \|tx\| ...)` — commit on `Ok`, rollback on `Err` |
| `@ControllerAdvice` error handling | `AppError` → JSON HTTP responses, works with `?` |
| Actuator health endpoint | `GET /actuator/health` built in |
| Serving a React/SPA build | `[static].dir` → SPA serving with `index.html` fallback |
| `SpringApplication.run()` | `Application::new()...run().await` |

Built on [axum](https://github.com/tokio-rs/axum), [sqlx](https://github.com/launchbadge/sqlx),
[figment](https://github.com/SergioBenitez/Figment), and [tokio](https://tokio.rs) —
all re-exported, so your app only depends on `rustspring`.

## Repository layout

```
crates/rustspring/   the framework crate — depend on this in your own apps
app/                 demo application showing every feature
config/              application.toml + per-profile overrides
frontend/            React (Vite) app: proxied in dev, served by the backend in prod
```

## Quick start (run the demo)

```sh
git clone https://github.com/piyooshsinha/rustspring
cd rustspring
cargo run
```

```sh
curl localhost:8080/actuator/health        # {"status":"UP","profile":"dev"}
curl localhost:8080/api/greet/World        # greeting from a singleton service
```

## How to use the framework

### 1. Add the dependency

```toml
[dependencies]
rustspring = { git = "https://github.com/piyooshsinha/rustspring" }
tokio = { version = "1", features = ["full"] }   # for #[tokio::main]
sqlx = "0.8"                                     # only if you use the database
serde = { version = "1", features = ["derive"] }
```

### 2. Bootstrap the application

```rust
use rustspring::{Application, Inject, axum::{Router, routing::get}};

struct GreetingService;
impl GreetingService {
    fn greet(&self) -> &'static str { "hello from rustspring" }
}

async fn hello(Inject(svc): Inject<GreetingService>) -> &'static str {
    svc.greet()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Application::new()
        .manage(GreetingService)                                // singleton "bean"
        .routes(Router::new().route("/api/hello", get(hello)))  // controller
        .run()                                                  // SpringApplication.run()
        .await
}
```

That's a complete application: config loading, logging, graceful shutdown,
and a health endpoint are already wired.

### 3. Configuration & profiles

Create `config/application.toml` (defaults) and `config/application-{profile}.toml`
(overrides). Select the profile with `APP_PROFILE` (default: `dev`):

```toml
# config/application.toml
[server]
host = "127.0.0.1"
port = 8080

[greeting]                 # your own custom sections
template = "Hello, {name}!"
```

```sh
APP_PROFILE=prod cargo run        # layers application-prod.toml on top
APP_SERVER__PORT=9090 cargo run   # env vars override files (APP_ prefix, __ separator)
```

Read your custom sections anywhere with the `Config` extractor — the
equivalent of `@ConfigurationProperties`:

```rust
use rustspring::Config;
use serde::Deserialize;

#[derive(Deserialize)]
struct GreetingConfig { template: String }

async fn handler(config: Config) -> String {
    let g: GreetingConfig = config.section("greeting").unwrap();
    g.template.replace("{name}", "World")
}
```

### 4. Singletons & dependency injection

Register any component once at startup; inject it into any handler. Everything
is `Arc`-shared, so all components are singletons:

```rust
Application::new()
    .manage(GreetingService::new())
    .manage(EmailClient::new())
    // ...

async fn handler(
    Inject(greeter): Inject<GreetingService>,
    Inject(email): Inject<EmailClient>,
) { /* ... */ }
```

A missing registration produces a clear error naming the type and the fix.

For components whose dependencies live in the context, skip the hand-wiring
entirely with `#[derive(Component)]` — the `@Component` of rustspring. Every
`Arc<T>` field is a dependency resolved at startup; other fields get
`Default::default()`:

```rust
use rustspring::Component;

#[derive(Component)]
struct UserService {
    pool: Arc<PgPool>,             // injected from [database] config
    greeter: Arc<GreetingService>, // injected: another component
    cache: RwLock<Vec<User>>,      // plain state: Default::default()
}

Application::new()
    .component::<UserService>()    // order doesn't matter —
    .manage(GreetingService::new()) // dependencies are resolved automatically
```

Components are constructed at startup after config and the pool are wired,
and dependency order is resolved automatically — register them in any order.
A dependency that was never registered (or a cycle) fails startup with a
report listing every stuck component — fail fast, like a Spring context
refresh.

### 5. Database pool & transactions

Add a `[database]` section and the framework builds and registers a `PgPool`
for you. The pool connects **lazily** — your app boots even if the database is
down, and connects on first use:

```toml
[database]
url = "postgres://postgres:postgres@localhost:5432/postgres"
max_connections = 10
```

```rust
use rustspring::{transactional, AppError, Inject};
use sqlx::PgPool;

async fn create_user(Inject(pool): Inject<PgPool>) -> Result<(), AppError> {
    // The @Transactional of rustspring: commits on Ok, rolls back on Err —
    // and on drop, so early returns and panics can't leave it half-done.
    transactional(&pool, |tx| Box::pin(async move {
        sqlx::query("INSERT INTO users (name) VALUES ($1)")
            .bind("Ada").execute(&mut **tx).await?;
        sqlx::query("INSERT INTO audit_log (action) VALUES ('user created')")
            .execute(&mut **tx).await?;
        Ok::<_, AppError>(())
    })).await
}
```

Try it against the demo (`docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16`):

```sh
curl -X POST localhost:8080/api/users -H 'content-type: application/json' -d '{"name":"Ada"}'
curl localhost:8080/api/users
```

### 6. Error handling

Return `Result<Json<T>, AppError>` from handlers and use `?` freely.
`AppError` maps to proper HTTP status codes with JSON bodies, and database /
config errors convert automatically:

```rust
async fn get_user(...) -> Result<Json<User>, AppError> {
    let user = find(id).await?                       // sqlx::Error -> 500
        .ok_or(AppError::NotFound("no such user".into()))?;  // -> 404
    Ok(Json(user))
}
```

### 7. React frontend

The `frontend/` directory is a Vite + React app, pre-wired both ways:

```sh
cd frontend && npm install

# development: Vite dev server on :5173 with hot reload,
# /api and /actuator proxied to the Rust backend on :8080
npm run dev

# production: build once, the Rust backend serves dist/ itself —
# one binary, one port, client-side routing handled via index.html fallback
npm run build && cd .. && cargo run
```

Point the backend at any SPA build via config:

```toml
[static]
dir = "frontend/dist"
```

## Demo application

The [app/](app) crate exercises every feature:

- [app/src/main.rs](app/src/main.rs) — bootstrap and wiring
- [app/src/greeting.rs](app/src/greeting.rs) — singleton service, custom config section, profile awareness
- [app/src/users.rs](app/src/users.rs) — `#[derive(Component)]` service with injected pool, multi-statement transaction with rollback

## Roadmap

- [x] `#[derive(Component)]` with constructor-based dependency resolution
- [x] Automatic dependency ordering — registration order is irrelevant
- [ ] MySQL / SQLite support behind feature flags
- [ ] `cargo generate` template for scaffolding new apps

Contributions welcome — open an issue or PR.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you shall be licensed as Apache-2.0, without any
additional terms or conditions.

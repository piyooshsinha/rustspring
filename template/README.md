# {{project-name}}

A web application built with [rustspring](https://github.com/piyooshsinha/rustspring),
the Spring Boot-style framework for Rust.

## Run

No setup required — the app uses SQLite and creates its database file on
first run.

```sh
cargo run
```

```sh
curl localhost:8080/actuator/health
curl localhost:8080/api/hello/World

# database-backed CRUD (SQLite, zero config)
curl -X POST localhost:8080/api/items \
  -H 'content-type: application/json' -d '{"title":"Buy milk"}'
curl localhost:8080/api/items
```

## What's inside

- `src/hello.rs` — a singleton service (`@Bean`) and its routes
- `src/items.rs` — a `#[derive(Component)]` service with an injected
  connection pool and a transactional insert (`@Transactional`)
- `config/application.toml` — config + profiles, with SQLite enabled

## Profiles & config

- `config/application.toml` — defaults
- `config/application-{profile}.toml` — per-profile overrides
- `APP_PROFILE=prod cargo run` — select a profile (default: `dev`)
- `APP_SERVER__PORT=9090 cargo run` — env vars override files

## Switching database

SQLite is the default. For Postgres or MySQL, change the `rustspring` and
`sqlx` features in `Cargo.toml` and update `database.url` in
`config/application.toml` — the framework picks the pool type from the URL
scheme. See the
[framework docs](https://github.com/piyooshsinha/rustspring#5-database-pool--transactions).

## Next steps

- Add services with `#[derive(Component)]` for constructor injection
- Point `[static].dir` at a React build to serve a frontend

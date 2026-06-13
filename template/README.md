# {{project-name}}

A web application built with [rustspring](https://github.com/piyooshsinha/rustspring),
the Spring Boot-style framework for Rust.

## Run

```sh
cargo run
curl localhost:8080/actuator/health
curl localhost:8080/api/hello/World
```

## Profiles & config

- `config/application.toml` — defaults
- `config/application-{profile}.toml` — per-profile overrides
- `APP_PROFILE=prod cargo run` — select a profile (default: `dev`)
- `APP_SERVER__PORT=9090 cargo run` — env vars override files

## Next steps

- Uncomment `[database]` in the config for a managed `PgPool` and
  `rustspring::transactional` — see the
  [framework docs](https://github.com/piyooshsinha/rustspring#5-database-pool--transactions)
- Add services with `#[derive(Component)]` for constructor injection
- Point `[static].dir` at a React build to serve a frontend

# Contributing to rustspring

Thanks for your interest! Issues and pull requests are welcome.

## Development setup

```sh
git clone https://github.com/piyooshsinha/rustspring
cd rustspring
cargo test                 # framework unit + integration tests
cargo run                  # demo app on :8080 (dev profile)
cd frontend && npm install && npm run dev   # React dev server on :5173
```

## Before opening a PR

CI runs these — save yourself a round trip:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Guidelines

- Keep the framework crate (`crates/rustspring`) free of application logic;
  the demo app (`app/`) is the place to showcase features.
- New framework features should come with a test and, where it helps, a short
  example in the README or demo app.
- Follow the existing Spring-analogy framing in docs — the goal is that a
  Spring Boot developer feels at home immediately.

## License

By contributing, you agree that your contributions will be licensed under the
[Apache License 2.0](LICENSE), without any additional terms or conditions.

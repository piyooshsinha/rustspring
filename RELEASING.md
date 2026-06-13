# Releasing rustspring

The workspace publishes two crates to [crates.io](https://crates.io):

1. `rustspring-macros` — the derive macros
2. `rustspring` — the framework (depends on `rustspring-macros`)

Because `rustspring` depends on `rustspring-macros` by version,
**`rustspring-macros` must be published first**. Until it is live on
crates.io, `cargo publish -p rustspring` (even `--dry-run`) fails with
"no matching package named `rustspring-macros` found" — this is expected.

The demo `app` and the `template` are never published (`publish = false` /
excluded from the workspace).

## One-time setup

```sh
cargo login   # paste a token from https://crates.io/settings/tokens
```

## Release steps

1. Bump `version` in `[workspace.package]` (root `Cargo.toml`) if needed.
   Both crates inherit it. Keep them in lockstep.

2. Dry-run the macros crate (fully verifies — builds in isolation):

   ```sh
   cargo publish -p rustspring-macros --dry-run
   ```

3. Publish the macros crate, then wait for the index to update:

   ```sh
   cargo publish -p rustspring-macros
   ```

4. Now the framework crate can be dry-run and published:

   ```sh
   cargo publish -p rustspring --dry-run
   cargo publish -p rustspring
   ```

5. Tag the release:

   ```sh
   git tag v0.1.0 && git push --tags
   ```

## Notes

- `docs.rs` builds `rustspring` with `all-features = true` (see
  `[package.metadata.docs.rs]`) so the Postgres, MySQL, and SQLite items are
  all documented.
- Each published crate carries its own `LICENSE` and `NOTICE` (copies of the
  workspace-root files) so the tarballs are license-complete.
- After publishing, update the root `README.md` quick-start to use
  `rustspring = "0.1"` instead of the git dependency, and the template's
  `Cargo.toml` likewise.

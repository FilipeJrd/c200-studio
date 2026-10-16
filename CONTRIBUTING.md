# Contributing

Create a branch from `main` and open a pull request describing the problem,
the change, and your validation. See the [README](README.md) for native build
dependencies. `rust-toolchain.toml` pins the Rust version and components used
in development and CI.

Write documentation, code comments, UI text, and pull request descriptions
in English. Keep user-facing messages concise and use the same control names
across the app and documentation. Existing profile names are user data and
must not be translated or migrated just to change the interface language.

Before submitting:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

To try the interface without accessing a camera:

```sh
cargo run -p c200-app -- --demo
```

Pull requests run these checks on Ubuntu 22.04 and 24.04, build an AppImage
on Ubuntu 22.04, and open the GUI in virtual X11. CI also checks workflow
definitions with actionlint and the syntax of each Bash script. The AppImage
workflow keeps its package, checksum, and demo screenshots in the run's
**Artifacts** section for 14 days. These are development builds of the
verified commit, not published versioned releases. See the
[release guide](docs/releases.md) for distribution details.

Tests that write to real hardware are opt-in and excluded from CI. Read the
README before running them. Demo mode and automated tests do not establish
compatibility with every firmware; report real-device checks when changing
protocol or capture code. Do not commit personal settings, serial numbers,
private diagnostics, or camera images.

Workflows use `pull_request`, read-only permissions, and Actions pinned to
commits. Preserve those properties when changing CI. Caches are only saved
by pushes to `main`. Packaging uses the project's pinned Rust version;
update the Dockerfile when changing it.

## Packaging

For the supported Ubuntu 22.04 glibc baseline, build in Docker:

```sh
bash scripts/portable-appimage.sh
```

To package directly on a prepared Linux x86_64 build host:

```sh
cargo install cargo-about --version 0.9.2 --locked --features cli
bash scripts/appimage.sh
```

The script uses linuxdeploy and includes dependency license notices. Building
on a newer distribution can require a newer glibc; use the Ubuntu 22.04 build
for portability checks. See [the release guide](docs/releases.md) before
publishing a package.

## Documentation screenshots

The README uses real screenshots of the English demo interface in light and
dark themes, stored in `docs/screenshots/`. To capture an updated screen:

```sh
cargo run --locked -p c200-app -- --demo --smoke-test --screenshot dist/demo.png
```

For a reproducible theme, pass `--config /path/to/demo-settings.json` pointing
to a separate demo settings file with `theme` set to `light` or `dark`. Inspect
the image before copying it to `docs/screenshots/`; keep screenshots free of
personal profiles, diagnostics, serial numbers, and live-camera images.

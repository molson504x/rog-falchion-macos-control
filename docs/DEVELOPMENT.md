# Development

## Prerequisites

- Rust stable via [rustup](https://rustup.rs). `rust-toolchain.toml` selects the toolchain and components automatically.
- Coverage: `cargo install cargo-llvm-cov`.
- Linux only: `libudev-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev`.

## Commands

| Task | Command |
|---|---|
| Build | `make build` |
| Test | `make test` |
| Format / lint | `make fmt` / `make lint` |
| Coverage (fails under 90% lines) | `make coverage` (`make coverage-html` opens a report) |
| Run CLI | `make run-cli ARGS="list"` or `cargo run -p falchion-cli -- list` |
| Run GUI | `make run-gui` |
| Release binaries into `dist/` | `make dist` (on Windows, add `EXE=.exe`) |

Cargo build output goes to `target/`. Packaged binaries go to `dist/`. Both are git-ignored.

## Troubleshooting

- **macOS `ld: tapi error: malformed file ... unknown architecture arm64e.x1`**: a newer (often beta) SDK is installed alongside older Command Line Tools, and `xcrun` picks the newest one. Point builds at the default SDK with `export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` (add it to `~/.zshrc`), or remove the stray SDK.

## Rules

- Every change ships with positive **and** negative unit tests. Coverage must stay at or above 90% (CI enforces this).
- The GUI must do all device I/O through `falchion-gui/src/worker.rs`. Never call a provider from `app.rs` (see [ARCHITECTURE.md](ARCHITECTURE.md#gui-threading-rule)).
- Put logic in testable modules. `main.rs`, `falchion-gui/src/app.rs` and `falchion-hid/src/backend.rs` are excluded from coverage, so keep them thin.
- Use `MockProvider` / `MockTransport` from `falchion-core::mock` for tests. Never require hardware in `cargo test`.
- Protocol changes must update [PROTOCOL.md](PROTOCOL.md).
- User-visible changes must update [USER-GUIDE.md](../USER-GUIDE.md) and, if relevant, [README.md](../README.md).
- Commit `Cargo.lock` (this workspace ships binaries).

## CI / release

- `.github/workflows/ci.yml` runs fmt, clippy (`-D warnings`), tests on macOS/Linux/Windows, and the coverage gate.
- `.github/workflows/release.yml` runs on `v*` tags. It builds for macOS (arm64 and x86_64), Linux and Windows, packages into `dist/`, and publishes a GitHub Release.

# ROG Falchion macOS Control

An early-stage project to make it possible to configure the ASUS ROG Falchion keyboard from macOS. ASUS provides Armoury Crate for configuring the keyboard, but its download requirements currently list Windows 10 (version 1903 or later) or Windows 11, not macOS.

**Project status:** scaffolded. A Rust CLI (`falchion`) and GUI (`falchion-gui`) build and are unit-tested. Most of the keyboard's HID protocol has now been documented from OpenRGB and from static analysis of Armoury Crate (see [docs/PROTOCOL.md](docs/PROTOCOL.md) and the [feature catalog](docs/FEATURES.md)), and the core commands (device info, battery, profiles, power settings, all lighting effects, per-key colour and saving to the keyboard) have been [confirmed on a real keyboard](docs/PROTOCOL.md#hardware-test-results), wired and wireless. The `lighting`, `info` and GUI code still sends placeholder packets, so those commands are not expected to work yet. `falchion raw` sends verified bytes directly.

## About

The goal is to build a user-friendly application for configuring the original ROG Falchion without requiring a Windows PC. The project is starting with macOS and aims to avoid a design that is tied to Apple-only technologies, so support for other platforms may be possible later.

The ROG Falchion is a compact 65% wireless mechanical keyboard. It supports wired USB and 2.4 GHz wireless connections, per-key RGB lighting, a touch panel, and onboard profiles. Configuration is currently provided through ASUS Armoury Crate.

## Installation

Download a release archive (macOS arm64/x86_64, Linux, Windows) from GitHub Releases, or build from source:

```sh
make dist        # binaries land in dist/
```

For ASUS's Windows software, see the [ROG Falchion support and downloads page](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/helpdesk_download/).

## Usage and planned capabilities

See [USER-GUIDE.md](USER-GUIDE.md) for usage (`falchion list`, `falchion lighting --effect wave`, or the GUI).

The project is intended to explore configuration options such as:

- Customizing keyboard lighting and RGB effects.
- Recording, assigning, and clearing macros.
- Remapping keys and managing keyboard profiles.
- Configuring the side touch panel.
- Investigating function-key behavior, including whether swapping Fn and Windows-key behavior is supported.

These are goals for development, not a promise that every feature is supported by the keyboard or will be implemented. ASUS's specifications list six onboard profiles and programmable keys except Fn and Shift; available controls may depend on the keyboard firmware and connection mode.

## Development (local debug)

Requires Rust stable ([rustup](https://rustup.rs)). On Linux, also install the packages listed in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

```sh
cargo run -p falchion-cli -- list    # CLI
cargo run -p falchion-gui            # GUI
make lint                            # rustfmt + clippy
```

## Running unit tests

```sh
make test        # cargo test --workspace --all-targets
make coverage    # cargo-llvm-cov; fails below 90% line coverage
```

Tests use `MockProvider`/`MockTransport` and never need hardware.

## Repository structure

| Path | Contents |
|---|---|
| `src/falchion-core` | Protocol, domain types, `Keyboard`, transport traits, mocks (no I/O) |
| `src/falchion-hid` | hidapi transport and device discovery |
| `src/falchion-cli` | `falchion` command-line tool |
| `src/falchion-gui` | `falchion-gui` desktop app (egui) |
| `docs/` | [Architecture](docs/ARCHITECTURE.md), [feature catalog](docs/FEATURES.md), [protocol notes](docs/PROTOCOL.md), [development guide](docs/DEVELOPMENT.md) |
| `.github/workflows` | CI (lint, test matrix, coverage gate) and tagged releases |
| `dist/` | Packaged release binaries (git-ignored) |

## Product references

- [ASUS ROG Falchion product page](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/)
- [ASUS ROG Falchion technical specifications](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/spec/)
- [ASUS ROG Falchion support and downloads](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/helpdesk_download/)

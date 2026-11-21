# Architecture

Rust Cargo workspace. All crates live under `src/`.

```
falchion-cli ─┐                 ┌─> falchion-hid (hidapi, real hardware)
              ├─> falchion-core <┤
falchion-gui ─┘                 └─> mock (in falchion-core, tests)
```

| Crate | Kind | Responsibility |
|---|---|---|
| `falchion-core` | lib | Domain types (`Rgb`, `Effect`, `LightingSettings`), report encoding (`protocol`), `Keyboard` operations, `Transport`/`DeviceProvider` traits, mocks. No I/O. |
| `falchion-hid` | lib | `HidProvider`/`HidTransport` over [hidapi](https://crates.io/crates/hidapi). `discovery.rs` holds the testable filtering logic; `backend.rs` is the thin hardware wrapper. |
| `falchion-cli` | bin `falchion` | clap command definitions and `run()`; `main.rs` only wires in `HidProvider`. |
| `falchion-gui` | bin `falchion-gui` | egui/eframe desktop app. `worker.rs` runs all device I/O on a background thread, `state.rs` holds UI behaviour, and `app.rs` only renders. |

## Key decisions

- **Rust**: single static binary per OS, no runtime, good HID support, cross-platform (macOS first, Linux/Windows come along).
- **hidapi**: uses IOKit on macOS, hidraw on Linux, and the Windows HID API. No kernel driver needed.
- **egui/eframe**: pure-Rust immediate-mode GUI. No webview and no JS toolchain. If a web UI is added later, `falchion-core` can compile to WASM and talk to the device over WebHID.
- **Testability**: front ends are generic over `DeviceProvider`, so all command and state logic is tested against `MockProvider`. Files that need hardware or a display (`main.rs`, `app.rs`, `backend.rs`) stay as thin as possible and are excluded from coverage.

## GUI threading rule

**Never call a `DeviceProvider` on the GUI thread.** On macOS, hidapi's `hid_enumerate` pumps the calling thread's CFRunLoop. On the main thread that re-enters winit's event handler, which panics inside a C callback ("tried to handle event while another event is currently being handled") and aborts the app. The GUI sends `worker::Request`s to `Worker` and applies the `Response`s in `update()`. The CLI has no event loop, so it calls hidapi directly.

## Adding a feature (e.g. macros, key remap)

1. Find the packets (USB capture, or probe the keyboard with `falchion raw`: see [DEVELOPMENT.md](DEVELOPMENT.md#probing-the-keyboard)) and document them in [PROTOCOL.md](PROTOCOL.md). Only build on commands marked ✓ (tested on hardware).
2. Add types plus a `Report::...` constructor in `falchion-core`, and a `Keyboard` method. Test both success and failure paths.
3. Expose it in the CLI (`falchion-cli/src/lib.rs`) and the GUI (add a `worker::Request` variant, handle the response in `state.rs`, then add the controls in `app.rs`).
4. Update [USER-GUIDE.md](../USER-GUIDE.md).

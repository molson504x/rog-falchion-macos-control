# Architecture

Rust Cargo workspace. All crates live under `src/`.

```
falchion-cli ─┐                 ┌─> falchion-hid (hidapi, real hardware)
              ├─> falchion-core <┤
falchion-gui ─┘                 └─> mock (in falchion-core, tests)
```

| Crate | Kind | Responsibility |
|---|---|---|
| `falchion-core` | lib | Domain types (`Rgb`, `Effect`, `LightingSettings`), report encoding (`protocol`), `Keyboard` operations (send, wait for the echo), `Transport`/`DeviceProvider` traits, mocks. No I/O. |
| `falchion-hid` | lib | `HidProvider`/`HidTransport` over [hidapi](https://crates.io/crates/hidapi). `discovery.rs` holds the testable filtering logic; `backend.rs` is the thin hardware wrapper. |
| `falchion-cli` | bin `falchion` | clap command definitions and `run()`; `main.rs` only wires in `HidProvider`. `hwtest.rs` holds the interactive hardware-test runner used by `tests/hardware_lighting.rs`. |
| `falchion-gui` | bin `falchion-gui` | egui/eframe desktop app. `worker.rs` runs all device I/O on a background thread, `state.rs` holds UI behaviour, and `app.rs` only renders. |

## Lighting model

`LightingSettings` holds every option of the `51 2C` command. `Effect` says which options each effect uses (`color_modes()`, `directions()`, `uses_thickness()`, `uses_background()`, `gradient_len()`), and `validate()` rejects anything else, so the CLI, the GUI and `Report::set_effect` all share one set of rules. `with_effect()` switches effect while keeping compatible options; the GUI routes every edit through `AppState` so its settings are always valid. The keyboard cannot report its current colours, only the active effect (`12 00`).

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
4. Add interactive hardware cases (see `falchion-cli/src/hwtest.rs` and [DEVELOPMENT.md](DEVELOPMENT.md#hardware-tests)).
5. Update [USER-GUIDE.md](../USER-GUIDE.md).

# User Guide

> **Early development.** Parts of the keyboard protocol have been confirmed on a real
> keyboard, but `falchion lighting`, `info` and the GUI still send placeholder packets,
> so they may have no effect yet. See [docs/PROTOCOL.md](docs/PROTOCOL.md).

ROG Falchion Control ships two programs:

- `falchion`: command-line tool
- `falchion-gui`: desktop app

## Install

Download the archive for your platform from the GitHub Releases page and extract it. To build from source instead, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

**macOS:** the binaries are unsigned. The first time you run one, right-click it, choose **Open**, then confirm. You can also run `xattr -d com.apple.quarantine falchion falchion-gui`.

## Connect the keyboard

Use the USB cable or the 2.4 GHz dongle. Bluetooth is not supported. Then run:

```sh
falchion list
```

You should see something like `ROG Falchion (wired) 0b05:193c at ...`.

## Command-line usage

| Goal | Command |
|---|---|
| List keyboards | `falchion list` |
| Show firmware version | `falchion info` |
| List lighting effects | `falchion effects` |
| Solid orange | `falchion lighting --effect static --color "#ff8800"` |
| Slow breathing, dim | `falchion lighting --effect breathing --color 00ffff --speed 0 --brightness 1` |
| Leftward rainbow wave, keep after power-off | `falchion lighting --effect wave --direction left --save` |
| Lights off | `falchion lighting --effect off` |
| Help | `falchion --help`, `falchion lighting --help` |
| Send a raw report (developers) | `falchion raw 12 00` (see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#probing-the-keyboard)) |

Lighting options:

| Option | Values | Default |
|---|---|---|
| `--effect` | `static`, `breathing`, `color-cycle`, `wave`, `reactive`, `off` | `static` |
| `--color` | Hex `#rrggbb` (the `#` is optional) | `#ffffff` |
| `--brightness` | `0` (off) to `4` (max) | `4` |
| `--speed` | `0` (slow) to `2` (fast) | `1` |
| `--direction` | `left`, `right` | `right` |
| `--save` | Store the setting on the keyboard | off |

Without `--save`, settings are lost when the keyboard powers off.

## Desktop app

Launch `falchion-gui`.

1. **Keyboards** lists what was detected. Click **Refresh** after plugging in.
2. **Lighting**: pick an effect. Color, speed and direction controls appear only when the effect uses them.
3. Click **Apply** to try a setting, or **Apply & Save** to keep it.
4. Status and errors appear at the bottom.

## Troubleshooting

| Problem | Fix |
|---|---|
| "no supported ROG Falchion keyboard found" | Check the cable or dongle. Unplug and replug, then run `falchion list`. Bluetooth is not supported. |
| Permission or transport error on macOS | Go to System Settings → Privacy & Security → Input Monitoring and allow your terminal app or `falchion-gui`, then restart it. |
| Permission error on Linux | Add a udev rule for vendor `0b05` that grants your user access to `hidraw`, then replug the keyboard. |
| Settings revert after unplugging | Use `--save` or **Apply & Save**. |
| Armoury Crate settings conflict | Profiles saved by Armoury Crate live on the keyboard. The last saved setting wins. |

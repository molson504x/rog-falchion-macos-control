# User Guide

> **Early development.** Lighting effects and `falchion info` work on a real keyboard.
> Profiles, key remapping, macros and the touch bar are not in the app yet.

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
| Firmware, active profile and effect | `falchion info` |
| List effects and their options | `falchion effects` |
| Solid orange | `falchion lighting --effect static --color "#ff8800"` |
| Slow breathing, red and blue | `falchion lighting -e breathing -c red -c blue --speed 0` |
| Rainbow wave moving left, kept after power-off | `falchion lighting -e wave -d left --save` |
| Red-to-blue wave from the centre, thick bands | `falchion lighting -e wave -c red -c blue -d out -t thick` |
| White stars on a blue background | `falchion lighting -e starry-night -c white -b blue` |
| Random-colour rain drops | `falchion lighting -e rain-drop -m random` |
| Lights off | `falchion lighting --brightness 0` |
| Help | `falchion --help`, `falchion lighting --help` |
| Send a raw report (developers) | `falchion raw 12 00` (see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#probing-the-keyboard)) |

### Effects

| Effect | What it does | Colours | Other options |
|---|---|---|---|
| `static` | One solid colour | 1 | — |
| `breathing` | Fades in and out | 1, 2 or random | speed |
| `color-cycle` | Whole keyboard cycles the spectrum | automatic | speed |
| `reactive` | Pressed keys light up | 1, 2 or random | speed |
| `wave` | Gradient sweeps across | 1–7 gradient stops | speed, direction (9), thickness |
| `ripple` | Rings spread from pressed keys | 1–7 stops or random | speed, thickness |
| `starry-night` | Random keys twinkle | 1, 2 or random | speed, density (`--thickness`), background |
| `quicksand` | Rows fill up | 6 (one per row, top first) or random | speed, direction `up`/`down` |
| `current` | Light flows across the keys | 1, 2 or random | speed, background |
| `rain-drop` | Drops run down each column | 1, 2 or random | speed, background |

### Lighting options

| Option | Values | Default |
|---|---|---|
| `-e`, `--effect` | See the table above (`rainbow` and `raindrop` also work) | `static` |
| `-c`, `--color` | Hex `#rrggbb` (the `#` is optional) or a name: `red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `magenta`, `pink`, `white`, `black`. Repeat for more colours | the effect's own |
| `-m`, `--color-mode` | `single`, `double`, `random`, `gradient` | `double` when two colours are given, otherwise the effect's first |
| `-b`, `--background` | Colour for unlit keys (`starry-night`, `current`, `rain-drop`) | none (dark) |
| `--brightness` | `0` (off) to `4` (100%), in 25% steps | `4` |
| `-s`, `--speed` | `0` (slowest) to `4` (fastest) | `2` |
| `-d`, `--direction` | Where the light moves: `right`, `left`, `up`, `down`, `up-right`, `up-left`, `down-right`, `down-left`, `out` (from the centre). Quicksand: `up` (fill from the bottom) or `down` | `right` (quicksand: `up`) |
| `-t`, `--thickness` | `slim`, `normal`, `thick` (band width; star density for `starry-night`) | `normal` |
| `--save` | Store the setting in the keyboard's active profile | off |

Options an effect doesn't support are rejected with a message saying what it accepts. Without `--save`, the setting may be lost when the keyboard powers off.

## Desktop app

Launch `falchion-gui`.

1. **Header**: the connected keyboard, its firmware and active profile. Click **⟳ Refresh** after plugging it in. On start, the app selects the effect the keyboard is showing (it can't read the colours).
2. **Effect**: click one of the ten effects. Hover for a description. Only the controls that effect uses are shown below it.
3. **Colors**: choose **One color**, **Two colors**, **Random** or **Custom colors** where the effect offers a choice, then click a swatch to pick a colour. For gradients use **+**/**−** to add or remove stops (up to 7). Quicksand shows one swatch per row. Tick **Background color** for unlit keys.
4. **Motion**: speed slider; for the rainbow wave, a 3 × 3 arrow pad sets the direction (the middle **✳** is outward from the centre); thickness or star density.
5. **Brightness**: Off, 25%, 50%, 75% or 100%.
6. Bottom bar: **Apply** shows the setting on the keyboard; **Apply & Save** also stores it in the active profile. Tick **Live preview** to send each change as you make it (not saved). **Factory default** resets the controls to a 50% rainbow wave. Status and errors appear underneath.

## Troubleshooting

| Problem | Fix |
|---|---|
| "no supported ROG Falchion keyboard found" | Check the cable or dongle. Unplug and replug, then run `falchion list`. Bluetooth is not supported. |
| Permission or transport error on macOS | Go to System Settings → Privacy & Security → Input Monitoring and allow your terminal app or `falchion-gui`, then restart it. |
| Permission error on Linux | Add a udev rule for vendor `0b05` that grants your user access to `hidraw`, then replug the keyboard. |
| Settings revert after unplugging | Use `--save` or **Apply & Save**. |
| "the keyboard rejected command 51 2C" | The keyboard refused the setting. Replug it and try again; if it keeps happening, open an issue with the command and the output of `falchion info`. |
| "invalid lighting settings: …" | The effect doesn't support that option. Run `falchion effects` to see what each one accepts. |
| Armoury Crate settings conflict | Profiles saved by Armoury Crate live on the keyboard. The last saved setting wins. |

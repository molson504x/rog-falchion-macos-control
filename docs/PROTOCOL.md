# Falchion HID protocol notes

> **Status:** documented from OpenRGB's working Falchion driver and from static analysis of
> Armoury Crate's keyboard HAL (see [Sources](#sources)). Not yet tested by this project on real hardware. The code in `falchion-core`
> still uses the **old placeholder values** until it is updated to match this page.

OpenRGB uses one controller for many ASUS Aura keyboards (Strix Scope, Azoth, TUF K-series).
The Falchion follows the generic "per-LED keyboard" path. It has no Falchion-only quirks
apart from its key layout.

## Device identification (confirmed in OpenRGB)

| Item | Value |
|---|---|
| Vendor ID | `0x0B05` (ASUSTek) |
| PID, wired (USB-C) | `0x193C` |
| PID, 2.4 GHz dongle | `0x193E` (same protocol as wired) |
| Control interface | USB interface **1**, usage page **`0xFF00`** |

OpenRGB opens the device by matching both the interface number and the usage page. Matching only the usage page may also hit another interface.

## Framing

- The host writes **65 bytes** with `hid_write`: report ID `0x00`, then 64 data bytes. Bytes are listed below as 64-byte data offsets, so data offset 0 is the first byte after the report ID.
- `data[0]` = command and `data[1]` = sub-command. There is **no marker byte**.
- Before each write, OpenRGB clears pending input reports by reading with a 0 ms timeout until nothing is left. After each write it waits about 20 ms for an acknowledgement (60 ms after a save). It never checks what the acknowledgement contains.

## Commands

| Purpose | Data bytes | Response |
|---|---|---|
| Firmware version | `12 00` | Version = `data[4].data[5].data[6]`, printed as hex `%02X.%02X.%02X`. Confirmed by the [Armoury Crate HAL](#replies) |
| Layout | `12 12` | Two bytes, `data[4]` and `data[5]`. OpenRGB computes `data[3]*100 + data[4]`, but the [HAL](#replies) reads 4 and 5. Then `layout % 100` selects ANSI or ISO (\*) |
| Set effect | `51 2C <mode> 00 <speed> <brightness> <color_mode> <direction> 02 <colors…>` | ack |
| Save to onboard profile | `50 55` | ack (wait 60 ms) |
| Direct per-key color | `C0 81 <n≤15> 00` then n × `<led_id> R G B` | ack, sent per packet |

(\*) Offsets are counted from the first byte of the reply, which echoes the command byte (see [Replies](#replies)). hidapi strips report ID 0 on read, so `hid_read` returns these offsets directly. The layout offset still needs checking on hardware.

### Set effect fields

| Field | Values |
|---|---|
| `mode` | 0 Static, 1 Breathing, 2 Spectrum cycle, 3 Reactive, 4 Rainbow wave, 5 Ripple, 6 Starry night, 7 Quicksand, 8 Current, 9 Rain drop. (15 = Direct: no effect packet. Use the `C0 81` command instead.) |
| `speed` | Raw byte. **Lower is faster**: 255 is slowest, 0 is fastest, and the default is 30. |
| `brightness` | Level 0–4 × 25, so 0, 25, 50, 75 or 100. There is no "off" mode: use brightness 0. |
| `color_mode` | 0 = single color, 16 = two colors (Breathing, Reactive, Starry night, Current, Rain drop), 1 = random |
| `direction` | Wave and Quicksand only: Left 4, Right 0, Up 6, Down 2, Horizontal 8, Vertical 1. Otherwise 0. |

**Color layout:**
- Non-wave modes: up to 3 colors as `R G B` triples starting at `data[9]`.
- Wave and Ripple: `data[9]` = number of colors (1–7), then 4 bytes per color starting at `data[10]`: `<position %> R G B`. Position = `100 / count × (i+1)`.

Color limits per mode (from OpenRGB): Static 1, Breathing 1–2, Wave 1–7, Reactive 1–2, Ripple 1–8, Starry night 1–3, Quicksand 6, Current 1–3, Rain drop 1–3.

### Per-key LED IDs (ANSI, 5 rows × 16 columns; `--` = no key)

```
  0   5   7  12  16  20  24  29  33  37  41  46  51  56  --  63
  1  --   8  13  17  21  25  30  34  38  42  47  52  57  59  64
  2  --   9  14  18  22  26  31  35  39  43  48  53  --  60  65
  3  --  10  15  19  23  27  32  36  40  44  49  54  --  61  66
  4   6  11  --  --  --  28  --  --  --  45  50  55  58  62  67
```

ISO uses different IDs (0–68). See `ASUS_FALCHION_LAYOUT_KEYS_ISO` in OpenRGB's `AsusAuraKeyboardLayouts.h`.

## From Armoury Crate's keyboard HAL (unverified)

> **Status:** found by static analysis of Armoury Crate's keyboard hardware layer
> (`AacKbHal_x64.dll` 1.02.97, class `AacM601` = Falchion, from the Full Installation
> Package 1.4.4.3). Not tested on hardware. Field meanings marked *(?)* are inferred.

The Falchion uses the shared `AacKbFunction_Base` / `AacKbFunction_Support_RF` / `AacKbEffect_Base` code, so these commands probably also work on other ASUS keyboards from the same generation.

### Transport behavior

- Every command is one 64-byte report, zero-filled, sent as in [Framing](#framing).
- After each write, Armoury Crate polls every 10 ms for up to **1500 ms** for a reply. It does not resend.
- `FF AA` is logged when a reply signals an error/NAK *(?)*. The exact error bytes were not found.

### Commands

| Purpose | Data bytes | Notes |
|---|---|---|
| Device info | `12 00` | See [Replies](#replies) |
| Power info (wireless) | `12 01` | Reply is parsed on a second HID interface *(?)*; fields not yet decoded |
| Touch bar info | `12 02` | Reply `data[5]` = current touch bar level |
| Layout | `12 12` | See [Commands](#commands) |
| Select profile | `51 00 00 00 <n>` | `n` = profile number *(?: whether 0- or 1-based)* |
| Save profile | `50 55` | Same as [Save](#commands) |
| Profile op A | `50 40 00 00 01` | Purpose unknown *(?: reset to default)* |
| Profile op B | `50 40` | Purpose unknown |
| Profile op C | `50 61` | Purpose unknown |
| Remap key | `51 20 <src_lo> <src_hi> <dst_lo> <dst_hi>` | Source and target are 16-bit key codes, little-endian. Armoury Crate looks them up in a table indexed by key `(row, col)`. The table is not yet extracted |
| Touch bar level | `51 0C <lo> <hi> <b>` | 16-bit value plus one extra byte *(?: function, effect mode)* |
| RGB indicator | `51 0A 00 00 <v>` | *(?: status LED on/off)* |
| Power setting | `51 <37–3A> 00 00 <v>` | Four sub-commands `37`, `38`, `39`, `3A`. They probably map to sleep time, low-battery level, power saving and power mode, but the order is unknown |
| Frame / direct mode | `51 90 00 00 <a> <b>` | Mode 1 writes `a`, mode 2 writes `b` *(?)* |
| Aura Sync | `41 <0\|1>` | 1 = enable sync (host-driven lighting) |
| Key-event log | `43 <0\|1>` | 1 = keyboard reports key presses (used for profile hotkeys). Replies start `43 xx`, with a key bitmap in `data[5..59]` |
| Set effect | `51 2C <mode> …` | See [Set effect fields](#set-effect-fields) and the notes below |

### Set effect: differences from OpenRGB

The HAL uses mode numbers 0–7 just as OpenRGB does (Static 0, Breathing 1, Color cycle 2, Reactive 3, Wave 4, Ripple 5, Starry night 6, Quicksand 7). Differences:

- **Speed** (`data[4]`): Armoury Crate's Slow/Normal/Fast levels map to bytes from a per-effect table built into the driver. The values weren't found. Static sends `FF`.
- **Brightness** (`data[5]`): level × 25. Same as OpenRGB.
- **Color mode** (`data[6]`): computed as `1 + …`, not OpenRGB's 0/16/1. Static sends `00`.
- **Bytes 7–8**: Static and Breathing send `FF FF` where OpenRGB sends `<direction> 02`. Wave sends `data[7]` = direction ID (0–8).
- **Colors**: start at `data[9]` for Static and Breathing, and at `data[8]` for the Wave gradient.
- **Thickness** (Wave/Ripple): not visible in the HAL packet. It may not be sent to the Falchion.

Test both encodings on hardware before choosing one.

### Replies

The keyboard replies on the same interface. **The reply echoes the command**: `data[0..1]` = the command bytes that were sent. Armoury Crate treats this echo as the acknowledgement for `51 20`, `51 2C`, `51 18`, `50 55`, `50 40`, `50 11`, `43 xx` and `53 xx`.

| Reply | Fields |
|---|---|
| `12 00` | `data[4..6]` = firmware version. `data[8..10]` = a second version, probably the dongle *(?)* |
| `12 02` | `data[5]` = touch bar level |
| `12 12` | `data[4]`, `data[5]` = layout bytes |
| `43 00`, `43 01` | Acknowledgement of the key-event log command |
| `43 02` | Key-state report: bitmap in `data[5..59]` |

## Still unknown

[FEATURES.md](FEATURES.md) tracks every feature. Still missing:
- The key-code table for `51 20` and the macro upload format
- Touch bar action assignment (only the level command is known)
- The meaning of each power sub-command and the `12 01` reply layout
- The speed-level → byte table, and wave thickness and background color

### How to capture

1. On Windows, install Armoury Crate and [Wireshark](https://www.wireshark.org/) with USBPcap.
2. Capture while changing **one** setting at a time.
3. Filter on `usb.idVendor == 0x0b05`. Record the SET_REPORT and interrupt OUT payloads on interface 1.
4. Add a table to this file, then implement it in `falchion-core` with tests.

## Licensing note

OpenRGB is GPL-2.0-or-later, and this project is MPL-2.0. This page records protocol **facts** (byte values) so the code can be written independently. **Do not copy OpenRGB source code** into this repository.

## Sources

- OpenRGB `Controllers/AsusAuraUSBController/AsusAuraKeyboardController/`, master branch as of 2026-11-18. Relevant files: `AsusAuraKeyboardController.{h,cpp}`, `AsusAuraKeyboardControllerDetect.cpp`, `RGBController_AsusAuraKeyboard.{h,cpp}`, `AsusAuraKeyboardLayouts.h`. https://gitlab.com/CalcProgrammer1/OpenRGB
- Original device request: https://gitlab.com/CalcProgrammer1/OpenRGB/-/issues/2043
- ASUS Armoury Crate Full Installation Package 1.4.4.3 (`dlcdnets.asus.com/pub/ASUS/mb/14Utilities/Armoury_Crate_Full_Installation_Package.zip`), studied on 2026-11-18 without running it. Files: `SupersetPackage_HAL` → `AacKbSetup` 1.02.97 → `AacKbHal_x64.dll`; `SupersetPackage_Others` → `KeyboardSDK` 3.00.85 and `HTML/ROG FALCHION(6460)` UI 4.01.08. Only facts are recorded here. No ASUS code or binaries are in this repo.

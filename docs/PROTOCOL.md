# Falchion HID protocol notes

> **Status:** documented from OpenRGB's working Falchion driver (see [Sources](#sources)).
> Not yet tested by this project on real hardware. The code in `falchion-core`
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
| Firmware version | `12 00` | Version = `data[4].data[5].data[6]`, printed as hex `%02X.%02X.%02X` (\*) |
| Layout | `12 12` | `data[3]*100 + data[4]`. Then `layout % 100` selects ANSI or ISO (\*) |
| Set effect | `51 2C <mode> 00 <speed> <brightness> <color_mode> <direction> 02 <colors…>` | ack |
| Save to onboard profile | `50 55` | ack (wait 60 ms) |
| Direct per-key color | `C0 81 <n≤15> 00` then n × `<led_id> R G B` | ack, sent per packet |

(\*) OpenRGB reads offsets 5–7 and 4–5 from a 65-byte `hid_read` buffer. hidapi strips the report ID on read for unnumbered reports, so those offsets may already be data offsets. **Check the exact offset against a real response.**

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

## Not covered by OpenRGB

OpenRGB controls lighting only. These still need USB captures from Armoury Crate (see below):
- Macros and key remapping
- Touch-panel behavior
- Fn and Windows-key options
- Choosing between the six onboard profiles
- Battery level and sleep timer for wireless

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

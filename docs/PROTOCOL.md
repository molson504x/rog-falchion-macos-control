# Falchion HID protocol notes

> **Status:** documented from OpenRGB's working Falchion driver and from static analysis of
> Armoury Crate's keyboard HAL (see [Sources](#sources)). Entries marked ✓ were
> [tested on a real keyboard](#hardware-test-results) (wired, firmware 13.61.31). `falchion-core`
> implements device info (`12 00`), set effect (`51 2C`) and save (`50 55`) from this page.

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
| Firmware version ✓ | `12 00` | Version = `data[4].data[5].data[6]`, printed as hex `%x.%x.%x` (test keyboard: `13 61 31` = 13.61.31) |
| Layout ✓ | `12 12` | Two bytes, `data[4]` and `data[5]` (test keyboard, ANSI: `01 11`). OpenRGB computes `data[3]*100 + data[4]`; which bytes mean ANSI or ISO needs an ISO keyboard to confirm (\*) |
| Set effect ✓ | `51 2C <mode> 00 <speed> <brightness> <color_mode> <direction> 02 <colors…>` | ack (echo). Layout [verified](#hardware-test-results) |
| Save to onboard profile ✓ | `50 55` | ack (echo; OpenRGB waits 60 ms) |
| Direct per-key color ✓ | `C0 81 <n≤15> 00` then n × `<led_id> R G B` | Reply **`C0 82 <n>`** (not an echo). **Send `51 2C 0F …` (effect 15, direct) first**, or nothing shows. Each packet is drawn as soon as it arrives; no sync (`41 01`) or `51 90` needed. Works wired and wireless. **Per-key colours are not stored**: after `50 55` and a power cycle the keyboard is still in direct mode but every key is dark, so the host must re-send them. (Effect colours *are* stored; this is specific to direct mode) |

(\*) Offsets are counted from the first byte of the reply, which echoes the command byte (see [Replies](#replies)). hidapi strips report ID 0 on read, so `hid_read` returns these offsets directly (confirmed on macOS).

### Set effect fields

| Field | Values |
|---|---|
| `mode` | 0 Static, 1 Breathing, 2 Spectrum cycle, 3 Reactive, 4 Rainbow wave, 5 Ripple, 6 Starry night, 7 Quicksand, 8 Current, 9 Rain drop. (✓ **15 = Direct**: send this effect packet, then colour keys with `C0 81`. OpenRGB skips the packet, but the Falchion needs it.) |
| `speed` | Raw byte. **Lower is faster**: 255 is slowest, 0 is fastest, and the default is 30. The app's five levels (slowest → fastest) send `FF A0 60 30 10`; these are our own choice, not Armoury Crate's values. |
| `brightness` ✓ | Level 0–4 × 25, so 0, 25, 50, 75 or 100. There is no "off" mode: brightness 0 turns all keys off (verified). |
| `color_mode` | 0 = single color (also used for gradients), 16 = two colors (Breathing, Reactive, Starry night, Current, Rain drop), 1 = random (those five plus Ripple and Quicksand) |
| `direction` ✓ | Wave: **0** left→right, **1** diagonal top-left→bottom-right, **2** top→bottom, **3** diagonal top-right→bottom-left, **4** right→left, **5** diagonal bottom-right→top-left, **6** bottom→top, **7** diagonal bottom-left→top-right, **8** centre-out (from the J/K column). OpenRGB's "Vertical 1" is wrong: 1 is a diagonal. Quicksand uses 2 and 6 only. Otherwise 0. |
| `size` (byte 8) ✓ | OpenRGB always sends `02` (the app does too for effects that ignore it). It is the band **thickness**: 0 = slim (about 1–2 keys), 1 = normal, 2 = thick (about 4–5 keys). On Starry Night it sets **density** (0 = fewer keys lit). |

**Color layout:**
- Non-wave modes: up to 3 colors as `R G B` triples starting at `data[9]` (slots 1–3 at `data[9]`, `data[12]`, `data[15]`).
  - ✓ **Background colour = slot 3** (`data[15..17]`) for Starry night, Current and Rain drop. With colour mode 0, slot 2 is ignored; leave slot 3 at zero for no background.
  - ✓ **Quicksand** takes 6 `R G B` triples from `data[9]`, one per row: colour 1 = top row. Direction `06` = fills from the bottom, `02` = from the top.
- Wave and Ripple: `data[9]` = number of colors (1–7), then 4 bytes per color starting at `data[10]`: `<position %> R G B`. Position = `100 × (i+1) / count`, so the last stop is 100 (the 3-colour test used `21 42 64`).

Color limits per mode (from OpenRGB): Static 1, Breathing 1–2, Wave 1–7, Reactive 1–2, Ripple 1–8, Starry night 1–3, Quicksand 6, Current 1–3, Rain drop 1–3.

### Per-key LED IDs ✓

**LED ID = column × 8 + row** (rows 0–4 top to bottom, columns 0–14 left to right). Verified on hardware (ANSI): every key lit with these IDs, and no other ID (0–127) lit anything. **OpenRGB's Falchion grid is wrong**: its sequential IDs light scattered keys.

```
row 0  Esc 0   1 8   2 16  3 24  4 32  5 40  6 48  7 56  8 64  9 72  0 80  - 88  = 96  Bksp 104  Ins 112
row 1  Tab 1   Q 9   W 17  E 25  R 33  T 41  Y 49  U 57  I 65  O 73  P 81  [ 89  ] 97  \ 105    Del 113
row 2  Caps 2  A 10  S 18  D 26  F 34  G 42  H 50  J 58  K 66  L 74  ; 82  ' 90  --    Enter 106 PgUp 114
row 3  LShift 3  --  Z 19  X 27  C 35  V 43  B 51  N 59  M 67  , 75  . 83  / 91  RShift 99  Up 107  PgDn 115
row 4  LCtrl 4  Win 12  LAlt 20  Space 52  RAlt 76  Fn 84  RCtrl 92  Left 100  Down 108  Right 116
```

- `--` = an empty slot (ISO keyboards put a key there): row 2 column 12 (ID 98) and row 3 column 1 (ID 11).
- Verified by key name: WASD, the arrows, every modifier (Shifts, Ctrls, Alts, Win, Fn), the right-edge column (Ins, Del, PgUp, PgDn, Backspace, Enter, `\`), the whole number row, R/O/G and Space each lit exactly as named, and all 68 keys lit together. See [hardware tests](#hardware-test-results).
- Space is column 6; columns 3–5 and 7–8 of row 4 have no key.

## From Armoury Crate's keyboard HAL

> **Status:** found by static analysis of Armoury Crate's keyboard hardware layer
> (`AacKbHal_x64.dll` 1.02.97, class `AacM601` = Falchion, from the Full Installation
> Package 1.4.4.3). Entries marked ✓ were [tested on hardware](#hardware-test-results);
> the rest come from static analysis only. Field meanings marked *(?)* are inferred.

The Falchion uses the shared `AacKbFunction_Base` / `AacKbFunction_Support_RF` / `AacKbEffect_Base` code, so these commands probably also work on other ASUS keyboards from the same generation.

### Transport behavior

- Every command is one 64-byte report, zero-filled, sent as in [Framing](#framing).
- After each write, Armoury Crate polls every 10 ms for up to **1500 ms** for a reply. It does not resend.
- ✓ A rejected command gets the reply **`FF AA`** (`data[0..1]`) instead of the echo.

### Commands

Command families: `12` = read info, `50` = profile storage, `51` = set, `52` = get, `41`/`43` = mode switches, `53` = macro data.

✓ Unsupported commands: `51`, `52`, `41`, `43` and `50` reply **`FF AA`**, but unknown `12` sub-commands get **no reply at all** (the read times out). `52` only supports `52 90`: the current effect settings (`52 2C`) and the other `51` settings **cannot be read back**, apart from what `12 00` and `12 01` report.

| Purpose | Data bytes | Notes |
|---|---|---|
| Device info ✓ | `12 00` | See [Replies](#replies) |
| Power info ✓ | `12 01` | See [Replies](#replies). Answers over USB too |
| Touch bar info ✓ | `12 02` | Reply `data[5]` = `01` on the test keyboard. It did not change after `51 0C` |
| Layout | `12 12` | See [Commands](#commands) |
| Unknown info | `12 13` | ✓ Answers with an echo and all-zero data (wireless). Meaning unknown. `12 03`–`12 20` otherwise get no reply |
| Select profile ✓ | `51 00 00 00 <n>` | `n` = profile number **1–6** (6 = the default profile); 7 → `FF AA`. **`n` = 0 changes nothing and the reply carries the active profile** in `data[4]`. `12 00` `data[10]` also reads it back |
| Save profile ✓ | `50 55` | Same as [Save](#commands) |
| Profile op A | `50 40 00 00 01` | Purpose unknown *(?: reset to default)* |
| Profile op B | `50 40` | Purpose unknown |
| Profile op C | `50 61` | Purpose unknown |
| Remap key | `51 20 <src_lo> <src_hi> <dst_lo> <dst_hi>` | Two 16-bit key codes, little-endian, looked up by [key ID](#key-ids). **Accepted but no effect on hardware**, live or after `50 55` (see [tests](#hardware-test-results)). The high byte is range-checked: `row << 8 \| col` codes are accepted, scan-matrix codes with high byte `06` → `FF AA`. The Falchion may remap on the host (Armoury Crate acting on key events) rather than in firmware |
| Choose macro slot | `51 18 02 00 <n>` | Selects macro slot `n` before uploading. Special forms: `51 18 01` (index "1"), `51 18 03` (index "0"), `51 18 04` (index "-1") *(?: meanings)* |
| Upload macro | `53 <key> <count> <more> <events…>` | See [Macros](#macros) |
| Touch bar level | `51 0C <lo> <hi> <b>` | Acknowledged, but **no visible effect** for any value tried (`<b>` 0–5, level `32`), wired or wireless; once the strip blanked briefly. Purpose unknown |
| Touch bar display ✓ | `51 0A 00 00 <v>` | What the touch bar's LED strip shows. **1** = battery gauge (blue, filled to the charge level; the default). **0** = full strip in red *(?: the lighting colour)* |
| *(unused)* | `51 37 00 00 <v>` | XML `blinking_lights`. **Not supported by this firmware**: rejected (`FF AA`) wired and wireless, for every value and byte position tried |
| Sleep timer ✓ | `51 38 00 00 <v>` | XML `idle_mode`. Option IDs 0–4 = 1, 2, 3, 5, 10 min; 255 = never (option 0 measured: lights off after about 58 s on battery). Read back in `12 01` `data[6]` |
| Power saving ✓ | `51 39 00 00 <v>` | XML `power_saving`. **0** = off, **1** = on, lights **turn off** below the threshold, **2** = on, lights **dim**. This is Armoury Crate's "power saving mode" choice. Read back in `12 01` `data[7]` |
| Low-battery threshold ✓ | `51 3A 00 00 <v>` | XML `low_power_mode`, Armoury Crate's "Lighting alert". `v` = 0 Never, 1–5 = 10–50% (not range-checked). Below the threshold the keyboard turns its main lighting off and the touch bar flashes red at the battery level. Read back in `12 01` `data[9]` |
| Frame / direct mode | `51 90 00 00 <a> <b>` | Mode 1 writes `a`, mode 2 writes `b` *(?)*. Read back with `52 90` (✓ answers; all zero by default) |
| Aura Sync | `41 <0\|1>` | 1 = enable sync (host-driven lighting) |
| Key-event log | `43 <0\|1>` | 1 = keyboard reports key presses (used for profile hotkeys). Acknowledged wired and wireless, but **the keyboard never sent `43 02` reports** in testing. `43 02` itself is rejected |
| Set effect | `51 2C <mode> …` | See [Set effect fields](#set-effect-fields) and the notes below |

Armoury Crate's set-function IDs 1 (no-op) and 11 (close handles) send nothing to the device.

### Key IDs

Armoury Crate identifies each key by a bit in a **21-byte key bitmap**: key ID = `byte * 8 + bit`. The settings files name keys `keyfunction_<byte>_<bit>`, and the `43 02` key report uses the same bitmap. A table in the driver maps each key ID to a physical `(row, col)` on a 5 × 15 grid:

```
        c0  c1  c2  c3  c4  c5  c6  c7  c8  c9  c10 c11 c12 c13 c14
row 0   31  37  44  51  58  65  72  95 102 109 116 123 130 144 159
row 1   29  36  43  50  57  64  87  94 101 108 115 122 129 136 158
row 2   28  35  42  49  56  79  86  93 100 107 114 121 128 151  62
row 3   27  34  41  48  71  78  85  92  99 106 113 120 150 157 104
row 4   26  33  40  --  --  --  77  --  --  105 0/112 142 149 156 155
```

Row 4 is the bottom row (the gaps are the spacebar). Key ID 0 and 112 both map to row 4, column 10 *(?: Fn)*. A second table gives each key a function code, where `0x14` = profile key (high byte = profile number).

These **key IDs** are Armoury Crate's settings numbering. They are not the [LED IDs](#per-key-led-ids-) used for per-key colour (`column × 8 + row`), although both are 8 slots per byte/column. `51 20` accepts codes in the `(row << 8) | col` form, but no remap took effect in testing (see [Commands](#commands-1)).

### Macros

1. `51 18 02 00 <slot>` selects the slot.
2. Events are sent in packets of up to 15: `53 <key> <count> <more>` then `count` × 4 bytes from `data[4]`:
   - `<key>` = code of the key that triggers the macro (from the key table)
   - `<more>` = 0 for the first packet, 1 for later ones
   - each event = `<code> <action> <d0> <d1>`. For normal keys, `action` = 2 for press and 1 for release. Codes `E8`–`E9` and `F0`–`F2` (mouse buttons *(?)*) use a different action encoding. `d0 d1` *(?: delay)*.
   - The last event's code is replaced with `FF`.
3. Each packet is acknowledged with a `53` echo.

### Set effect: HAL reading vs hardware

Static analysis suggested Armoury Crate fills bytes 4–8 differently from OpenRGB. [Hardware tests](#hardware-test-results) showed **OpenRGB's layout is correct**: colour mode `01` = random and `10` = two colours, the wave gradient starts with a stop count at `data[9]`, and lower speed bytes are faster. For Static, bytes 4, 7 and 8 are ignored. Use [Set effect fields](#set-effect-fields).

Byte 8 is the wave/ripple **thickness**, and colour slot 3 is the **background** ([tested](#hardware-test-results)).

**Without `50 55`, what survives a power cycle is unpredictable.** Re-selecting the profile did not undo an effect change. But after switching to the dongle (which power-cycles the keyboard), it came back with an effect from the middle of the test session: neither the original nor the last one sent. The keyboard seems to commit lighting to flash only at some points. **Always send `50 55` after setting lighting you want to keep**: set + `50 55` was confirmed to survive a power cycle.

### Replies

The app sends one command at a time and waits up to 1500 ms for its echo, skipping up to 8 unrelated reports; `FF AA` is reported as a rejection.

**The reply echoes the command**: `data[0..1]` = the command bytes that were sent. Armoury Crate treats this echo as the acknowledgement for `51 xx`, `50 55`, `50 40`, `50 11`, `41 xx`, `43 xx`, `52 90` and `53 xx`. Armoury Crate reads two HID interfaces: the control interface and a second one *(?: interface 3, used by the dongle path)*. Both parse the same replies.

| Reply | Fields |
|---|---|
| `12 00` ✓ | `data[8]` = profile count (6). `data[9]` = current effect ID. `data[10]` = current profile (1-based). **Wired:** `data[4..6]` = keyboard firmware, `data[11..]` unused. **Through the dongle:** `data[4..6]` = **dongle** firmware (test: 7.0.2), `data[11]` = connection state (`01` = connected, `FF` = no keyboard), `data[12..14]` = **keyboard** firmware |
| `12 01` ✓ | Power: `data[4]` = `00` *(?)*. **`data[5]` = battery level in tens** (`04` at 40%, `05` at 50%). `data[6]` = sleep timer. `data[7]` = power saving. `data[8]` = 1 while charging. `data[9]` = low-battery threshold. **`data[10]` = battery %**. **`data[11..12]` = battery voltage in mV, little-endian** (test: `C4 0E` = 3780 mV on battery, `3E 0F` = 3902 mV charging). Same layout wired and wireless |
| `12 02` | `data[5]` = touch bar level |
| `12 12` | `data[4]`, `data[5]` = layout bytes |
| `52 90` | `data[4]`, `data[5]` = frame/direct-mode state |
| `43 00`, `43 01` | Acknowledgement of the key-event log command |
| `43 02` | Key-state report: the 21-byte [key bitmap](#key-ids) starts at `data[5]` |

## Still unknown

[FEATURES.md](FEATURES.md) tracks every feature. Neither static analysis nor hardware tests from macOS settled these. Most need a USB capture of Armoury Crate on Windows:
- How key remapping actually works: `51 20` is accepted but changes nothing, live or saved. Combos, media keys, shortcuts and text assignments are also unknown. Likely needs a USB capture of Armoury Crate
- What `12 01` `data[4]` and the `12 13` reply mean (both all zero so far)
- The meaning of the `50 40` / `50 61` profile operations
- What `51 0C` changes on the touch bar, and how touch bar **actions** are assigned (in the SDK's touch panel XML; the HAL only showed `51 0C`)
- The key-event log: `43 01` is acknowledged, but the keyboard never sent `43 02` reports (see [hardware tests](#hardware-test-results)). Only one vendor interface exists, so they aren't arriving elsewhere. Possibly unused by this firmware, or Armoury Crate reads key events from a standard keyboard interface instead
- Armoury Crate's exact Slow/Normal/Fast speed bytes

### How to capture

1. On Windows, install Armoury Crate and [Wireshark](https://www.wireshark.org/) with USBPcap.
2. Capture while changing **one** setting at a time.
3. Filter on `usb.idVendor == 0x0b05`. Record the SET_REPORT and interrupt OUT payloads on interface 1.
4. Add a table to this file, then implement it in `falchion-core` with tests.

## Hardware test results

Tested 2026-11-20 on a ROG Falchion (firmware 13.61.31, ANSI) from macOS, wired (`0B05:193C`) and through the 2.4 GHz dongle (`0B05:193E`, dongle firmware 7.0.2), using `falchion raw` (see [DEVELOPMENT.md](DEVELOPMENT.md#probing-the-keyboard)). Every setting changed was restored.

| Test | Sent | Result |
|---|---|---|
| Static, colour at `data[9..11]` | `51 2C 00 00 FF 64 00 FF FF 00 FF 00` | Solid green, full brightness |
| Static, OpenRGB bytes 4–8 | `51 2C 00 00 1E 64 00 00 02 00 00 FF` | Solid blue, full brightness |
| Brightness 25% | `51 2C 00 00 FF 19 00 FF FF FF FF FF` | Dim white |
| Breathing, colour mode `01` | `51 2C 01 00 1E 64 01 FF FF FF 00 00` | Random colour each pulse |
| Breathing, colour mode `10` | `51 2C 01 00 1E 64 10 00 02 FF 00 00 00 00 FF` | Fades, alternating red and blue |
| Wave, speed `00` | `51 2C 04 00 00 64 00 00 02 03 21 FF 00 00 42 00 FF 00 64 00 00 FF` | Too fast to follow |
| Wave, speed `80` | same, `data[4]` = `80` | Left-to-right red-green-blue chase |
| Wave, speed `FF` | same, `data[4]` = `FF` | Same chase, very slow |
| Select profile 2, then 1 | `51 00 00 00 02`, then `… 01` | `12 00` `data[10]` reads `02`, then `01` |
| Power settings | `51 38`, `51 39`, `51 3A` | Echo; `12 01` bytes 6, 7 and 9 change, wired and wireless. `51 37` → `FF AA` in both modes |
| Low-battery threshold | `51 3A 00 00 05` at 40% battery, then `01` | 50%: main lighting off, touch bar flashes red about halfway. 10%: lighting back on |
| Power-saving mode | `51 39 00 00 02`, then threshold 50% | Keys **dim** instead of turning off; touch bar flashes red |
| Wave direction | `51 2C 04 …` with `data[7]` = 0–8 | See the [direction](#set-effect-fields) row |
| Thickness | wave with `data[8]` = 0, 1, 2 | Bands about 1–2 keys, medium, about 4–5 keys |
| Starry Night | `data[8]` 0 vs 2; colour in slot 4; colour mode `10` | Fewer stars at 0; slot 4 ignored; two colours work |
| Touch bar display | `51 0A 00 00 00`, then `01` | Full red strip, then back to the blue battery gauge |
| Per-key / key log (dongle) | `C0 81 …`, `43 01` | No visible change; no key reports |
| LED map | Grid rows lit one at a time, then every `column × 8 + row` ID, then every other ID | OpenRGB grid lit scattered keys; the formula lit every key, other IDs lit nothing. Enter = 106, Backspace = 104, Insert = 112, ID 98 = no key |
| Remaining effects | Colour cycle (2), Reactive (3), Ripple (5), Quicksand (7), Current (8), Rain drop (9), each with single / two-colour / random as applicable | All work as named. Reactive and Ripple respond to key presses; Ripple spreads radially from the key; Quicksand fills radially, one colour per row; Rain drop runs down each column |
| Background colour | Starry night and Rain drop with blue in slot 3; Starry night and Current with blue in slot 2 | Slot 3: unlit keys show blue. Slot 2: no background |
| Brightness 0 | `51 2C 00 00 1E 00 …` | All keys off |
| Key remap | Insert → A with `51 20`, in four encodings: HAL `row << 8 \| col` (`0E 00` → `01 02`), LED ID (`70 00` → `0A 00`), key-bitmap ID (`9F 00` → `23 00`), scan-matrix table B (`08 06` → `02 06`); the HAL form also with `50 55` | First three and the save: echo, but Insert still typed `ESC [ 2 ~`. Table B: `FF AA`. Each restored with the same encoding; Insert normal at the end |
| Profiles 3–7, 0 | `51 00 00 00 <n>` | 3–6 selected (effects 8, 9, 3, 2); 7 → `FF AA`; 0 → no change, reply `51 00 00 00 06` (the active profile) |
| Saved per-key pattern | Direct mode, WASD red + arrows blue, `50 55`, keyboard switched off and on | Before: as painted. After: `12 00` effect still `0F`, all keys dark |
| Per-key over the dongle | Same pattern through `0B05:193E` | Shown correctly; every packet answered `C0 82` |
| Sleep timer | `51 38 00 00 00` on battery, no input | Lights off after about 58 s; any key wakes it. Restored to `02` |
| Battery level | `12 01` at 40% then 50% charge | `data[5]` `04` → `05`, `data[10]` `0x28` → `0x32` |
| Touch bar strip | Watched while wired vs on battery | Green while charging, blue on battery (battery gauge) |
| Read-back probes | `52 00/0A/0C/18/20/2C/37/38/39/3A/90`; `12 03`–`12 20` (wireless) | Only `52 90` answers; the rest of `52` → `FF AA`. Only `12 13` answers (zeros); other `12` sub-commands time out |
| Key names → LEDs | Named groups painted via `column × 8 + row` (WASD, arrows, modifiers, right edge, number-row rainbow, R/O/G + Space, all 68 keys) | Every group matched exactly |
| Per-key (wired) | `51 2C 0F 00 00 64 00 00 02`, then LED IDs 0–127 blue in 9 packets | Keyboard turned blue about two columns per packet; each packet answered `C0 82 0F` (last `C0 82 08`). Adding `41 01` or `51 90 00 00 01` changed nothing |
| Remote-control mode | `41 02`, `41 03`, `41 04` | `FF AA`: only 0 and 1 are valid |
| Key-event log | `43 01`, then one continuous 15 s listen while pressing Esc, Q, Space, Fn + A, Right Arrow (wired) | Only the `43 01` echo; no `43 02` reports. `43 02` sent as a query → `FF AA`. The wired keyboard exposes one vendor interface (`0xFF00`, 64-byte), plus standard keyboard (8- and 15-byte) and consumer (22-byte) interfaces |
| Battery | `12 01` on battery vs charging | `data[8]` 0 vs 1; `data[10]` = 40 both times; `data[11..12]` 3780 vs 3902 mV |
| Dongle | `12 00` through the dongle | Dongle firmware at `data[4..6]`, keyboard firmware moves to `data[12..14]` |
| Save | `51 2C …` static red, then `50 55` | Echo; `12 00` `data[9]` = 0. **Still static red after switching the keyboard off and on** |
| Save stores the colour | Static **green** + `50 55`, power cycle; then static red + `50 55`, power cycle (wireless) | Green after the first cycle, red after the second: the save keeps the effect *and* its colours |

## Licensing note

OpenRGB is GPL-2.0-or-later, and this project is MPL-2.0. This page records protocol **facts** (byte values) so the code can be written independently. **Do not copy OpenRGB source code** into this repository.

## Sources

- OpenRGB `Controllers/AsusAuraUSBController/AsusAuraKeyboardController/`, master branch as of 2026-11-18. Relevant files: `AsusAuraKeyboardController.{h,cpp}`, `AsusAuraKeyboardControllerDetect.cpp`, `RGBController_AsusAuraKeyboard.{h,cpp}`, `AsusAuraKeyboardLayouts.h`. https://gitlab.com/CalcProgrammer1/OpenRGB
- Original device request: https://gitlab.com/CalcProgrammer1/OpenRGB/-/issues/2043
- ASUS Armoury Crate Full Installation Package 1.4.4.3 (`dlcdnets.asus.com/pub/ASUS/mb/14Utilities/Armoury_Crate_Full_Installation_Package.zip`), studied on 2026-11-18 without running it. Files: `SupersetPackage_HAL` → `AacKbSetup` 1.02.97 → `AacKbHal_x64.dll`; `SupersetPackage_Others` → `KeyboardSDK` 3.00.85 (`ArmouryKbSDK.dll`, power XML → command mapping) and `HTML/ROG FALCHION(6460)` UI 4.01.08. Analyzed with radare2 and Ghidra (headless decompiler). Only facts are recorded here. No ASUS code or binaries are in this repo.

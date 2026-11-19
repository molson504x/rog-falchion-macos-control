# Falchion feature catalog

What Armoury Crate can configure on the ROG Falchion, and how much of each feature we can reproduce. This is the feature roadmap. Byte-level details belong in [PROTOCOL.md](PROTOCOL.md).

**Source:** the Falchion device-UI package in the Armoury Crate Full Installation Package 1.4.4.3 (UI version 4.01.08, model numbers `6460` wired and `6462` dongle). The feature list comes from that package's capability and default-settings files. Everything here is written in our own words. No ASUS files or code are stored in this repo.

**Protocol column:**
- A link to [PROTOCOL.md](PROTOCOL.md) means the bytes are known (from OpenRGB or from [Armoury Crate's HAL](PROTOCOL.md#from-armoury-crates-keyboard-hal-unverified)), but untested on hardware.
- **Partial** means some of it is known, with differences noted.
- **Unknown** means the bytes still need to be found (see [Open research](#open-research)).

**App column:** **Placeholder** means the CLI/GUI have the feature but send placeholder packets. **No** means it isn't built yet.

## Device and profiles

| Feature | Armoury Crate behavior | Protocol | App |
|---|---|---|---|
| Device identity | VID `0x0B05`, PID `0x193C` wired / `0x193E` dongle | [Known](PROTOCOL.md#device-identification-confirmed-in-openrgb) | Yes |
| Firmware version | Shown as "Keyboard version" | [Known](PROTOCOL.md#commands) (`12 00`) | Placeholder |
| Onboard profiles | 5 profiles plus 1 editable default profile (ID 6) | Partial: [save](PROTOCOL.md#commands) `50 55` and [select](PROTOCOL.md#commands-1) `51 00 00 00 <n>` known; reading the active profile is unknown | Placeholder (save) |
| Profile hotkeys | Fn + A/S/D/F/G switches profile 1–5; the default profile is separate | Handled on the keyboard. The host can watch key presses via the [key-event log](PROTOCOL.md#commands-1) `43` | No |
| Sync / host-driven lighting | Lighting follows ASUS Aura Sync | [Known](PROTOCOL.md#commands-1) `41 <0\|1>` | No |
| Wireless | Dongle supported; same protocol as wired. The 2.4 GHz model adds an Aura Sync lighting mode (effect 241, brightness only) and a dongle firmware version | [Known](PROTOCOL.md#device-identification-confirmed-in-openrgb) | Yes |

## Lighting

Common controls: **brightness** 0/25/50/75/100% ([known](PROTOCOL.md#set-effect-fields)), **speed** Slow/Normal/Fast.

| # | Effect | Color options | Extra controls | Protocol |
|---|---|---|---|---|
| 0 | Static | Single | — | [Known](PROTOCOL.md#set-effect-fields) |
| 1 | Breathing | Single, Double, Random | Speed | [Known](PROTOCOL.md#set-effect-fields) |
| 2 | Color cycle | — | Speed | [Known](PROTOCOL.md#set-effect-fields) |
| 3 | Reactive | Single, Double, Random | Speed | [Known](PROTOCOL.md#set-effect-fields) |
| 4 | Rainbow (wave) | Gradient, up to 7 draggable stops | Speed, 9 directions, thickness (Slim/Normal/Thick) | Partial: thickness and 3 directions unknown |
| 5 | Ripple | Gradient (7 stops) or Random | Speed, thickness | Partial: thickness unknown |
| 6 | Starry night | Single, Double, Random | Speed, background (Single color/Off) | Partial: background unknown |
| 7 | Quicksand | Fixed 6-color pattern or Random | Speed, direction (2 options) | [Known](PROTOCOL.md#set-effect-fields) |
| 8 | Current | Single, Double, Random | Speed, background | Partial: background unknown |
| 9 | Raindrop | Single, Double, Random | Speed, background | Partial: background unknown |
| — | Per-key direct color | Any RGB per key | — | [Known](PROTOCOL.md#per-key-led-ids-ansi-5-rows--16-columns-----no-key) (`C0 81`) |

Differences from [PROTOCOL.md](PROTOCOL.md#set-effect-fields). The HAL also encodes some bytes differently from OpenRGB; see [Set effect: differences from OpenRGB](PROTOCOL.md#set-effect-differences-from-openrgb).
- **Speed:** Armoury Crate offers 3 levels (`0` Fast, `1` Normal, `2` Slow), mapped to bytes through a per-effect table built into the driver. OpenRGB sends a raw byte. The table values are unknown.
- **Directions:** Armoury Crate uses IDs 0–8 (9 values). OpenRGB documents 6 of them: 0, 1, 2, 4, 6, 8.
- **Ripple colors:** 7 gradient stops in Armoury Crate; OpenRGB allows 1–8.
- **Off:** Armoury Crate has no "Off" effect. Brightness 0% turns the lights off.
- When wireless power saving is on, Armoury Crate blocks lighting changes.

Factory default: Rainbow wave, 50% brightness, Normal speed, Normal thickness, 7-stop rainbow gradient.

## Keys (remapping)

Any key except **Fn** can be assigned one of:

Single-key remaps use [`51 20 <src> <dst>`](PROTOCOL.md#commands-1) with 16-bit key codes. Keys are addressed by [key ID](PROTOCOL.md#key-ids), and the ID → (row, column) table has been extracted. Which code `51 20` actually sends still needs a hardware test. Assignment types beyond a plain key-to-key remap are unknown.

| Assignment | Options | Protocol |
|---|---|---|
| Default | Restore the original key | Partial (`51 20`, src = dst *(?)*) |
| Keyboard function | One key or a key combination | Partial (`51 20` for one key; combos unknown) |
| Mouse function | Left button, right button | Unknown |
| Macro | A recorded macro (see below) | Partial: [upload format](PROTOCOL.md#macros) known; binding a key to a macro unknown |
| Multimedia | Mute, volume up/down, play/pause, previous, next, stop, mic mute | Unknown |
| Windows shortcut | Close app, switch apps, show desktop, cut, copy, paste | Unknown |
| Preset text | Types a string of up to 250 characters | Unknown |
| Launch app / open website | Handled by Armoury Crate on the PC, not the keyboard | N/A (needs a background app) |
| Disable | Key does nothing | Unknown |

**Gaming mode:** can block Alt+Tab and Alt+F4. Protocol unknown.

**Macros:** built in Armoury Crate and stored on the keyboard. You can also record them on the keyboard with Fn + Left Alt (press it twice to search). Upload is [known](PROTOCOL.md#macros): `51 18` selects a slot, then `53` packets carry up to 15 key events each.

## Touch bar

| Feature | Options | Protocol |
|---|---|---|
| Touch bar action | Default, zoom in/out, multimedia (volume, play/pause, previous, next, stop), keyboard function, macro, Windows shortcut, preset text, open website, disable | Unknown |
| Touch bar level / effect mode | One global setting (default `1`). The meaning of each value is still unknown | Partial: [set](PROTOCOL.md#commands-1) `51 0C`, [read](PROTOCOL.md#replies) `12 02` |
| Lock | Fn + Tab locks the slider (on the keyboard itself) | N/A |

## Power (wireless)

| Feature | Options | Protocol |
|---|---|---|
| Sleep after idle | 1, 2, 3, 5 or 10 minutes, or Never (default: 3 minutes) | [Known](PROTOCOL.md#commands-1) `51 38` |
| Low-battery alert and power saving | Alert and enter power saving at 10/20/30/40/50% battery, or Never | [Known](PROTOCOL.md#commands-1) `51 37`; value encoding *(?)* |
| Power-saving mode | On/off plus a mode setting (default on, mode `1`) | [Known](PROTOCOL.md#commands-1) `51 39` (on/off), `51 3A` (mode) |
| Battery level / charging | Shown in Armoury Crate | Partial: [`12 01`](PROTOCOL.md#replies) reply has a charging flag; which byte is battery % is unconfirmed |
| Dongle firmware / connection | 2.4 GHz model shows the dongle version | [Known](PROTOCOL.md#replies) in the `12 00` reply |

Power settings apply to the whole device, not per profile.

## Firmware update

Armoury Crate shows the firmware version and can update it. We will **not** build updating (bricking risk). Reading the version is [known](PROTOCOL.md#commands).

## Onboard shortcuts (no host software needed)

Useful for the [user guide](../USER-GUIDE.md). These work without any app:

| Keys | Action |
|---|---|
| Fn + A / S / D / F / G / H | Switch profile |
| Fn + Left / Right | Change lighting effect |
| Fn + Up / Down | Brightness |
| Fn + Q / W / E / R / T / Y / U | Play/pause, stop, previous, next, mute, volume down, volume up |
| Fn + Ins | Fn lock |
| Fn + Windows | Windows key lock |
| Fn + Tab | Touch bar lock |
| Fn + Left Alt | Record a macro on the fly |
| Fn + Del / R-Ctrl / P / PgUp / PgDn | Scroll lock / Menu / Print screen / Home / End |
| Fn + Esc, Fn + Shift + Esc | Backtick, tilde |
| Fn + Caps | Hardware reset |

## Open research

In Armoury Crate, the UI sends high-level JSON to a Windows service. The keyboard SDK (`ArmouryKbSDK.dll`) passes it to the keyboard HAL (`AacKbHal_x64.dll`, class `AacM601`), which builds the HID reports. The commands found there so far are in [PROTOCOL.md](PROTOCOL.md#from-armoury-crates-keyboard-hal-unverified). [Still unknown](PROTOCOL.md#still-unknown) lists what's left. Every command needs testing on real hardware before we rely on it.

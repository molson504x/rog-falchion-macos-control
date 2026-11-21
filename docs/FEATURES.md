# Falchion feature catalog

What Armoury Crate can configure on the ROG Falchion, and how much of each feature we can reproduce. This is the feature roadmap. Byte-level details belong in [PROTOCOL.md](PROTOCOL.md).

**Source:** the Falchion device-UI package in the Armoury Crate Full Installation Package 1.4.4.3 (UI version 4.01.08, model numbers `6460` wired and `6462` dongle). The feature list comes from that package's capability and default-settings files. Everything here is written in our own words. No ASUS files or code are stored in this repo.

**Protocol column:**
- A link to [PROTOCOL.md](PROTOCOL.md) means the bytes are known (from OpenRGB or from [Armoury Crate's HAL](PROTOCOL.md#from-armoury-crates-keyboard-hal)).
- ✓ means the command was [tested on a real keyboard](PROTOCOL.md#hardware-test-results).
- **Partial** means some of it is known, with differences noted.
- **Unknown** means the bytes still need to be found (see [Open research](#open-research)).

**App column:** **Yes** means the CLI and GUI support it with verified bytes. **No** means it isn't built yet.

## Device and profiles

| Feature | Armoury Crate behavior | Protocol | App |
|---|---|---|---|
| Device identity | VID `0x0B05`, PID `0x193C` wired / `0x193E` dongle | [Known](PROTOCOL.md#device-identification-confirmed-in-openrgb) | Yes |
| Firmware version | Shown as "Keyboard version"; the 2.4 GHz model also shows the dongle version | ✓ [Known](PROTOCOL.md#replies) (`12 00`) | Yes (`falchion info`, GUI header) |
| Onboard profiles | 5 profiles plus 1 editable default profile (ID 6) | ✓ [select](PROTOCOL.md#commands-1) `51 00 00 00 <n>` (1–6; 0 = query the active profile), read the active profile (`12 00` byte 10), ✓ [save](PROTOCOL.md#commands) `50 55` | Save: Yes. Read active profile: Yes. Select: No |
| Profile hotkeys | Fn + A/S/D/F/G switches profile 1–5; the default profile is separate | Handled on the keyboard. The host can detect a switch by polling the active profile (`12 00` byte 10, or `51 00 00 00 00`). The [key-event log](PROTOCOL.md#commands-1) `43` never reports | No |
| Sync / host-driven lighting | Lighting follows ASUS Aura Sync | Partial: `41 <0\|1>` is accepted (other values → `FF AA`), but no visible effect was seen. Per-key colour works without it | No |
| Wireless | ✓ Dongle supported; same protocol as wired (tested). The 2.4 GHz model adds an Aura Sync lighting mode (effect 241, brightness only) and a dongle firmware version | [Known](PROTOCOL.md#device-identification-confirmed-in-openrgb) | Yes |

## Lighting

Common controls: **brightness** 0/25/50/75/100% (✓ [known](PROTOCOL.md#set-effect-fields)), **speed** Slow/Normal/Fast (✓ raw byte, lower = faster; Armoury Crate's exact three values unknown).

All ten effects and every control below are in the CLI (`falchion lighting`) and GUI. Per-key direct colour is not yet.

| # | Effect | Color options | Extra controls | Protocol |
|---|---|---|---|---|
| 0 | Static | Single | — | ✓ [Known](PROTOCOL.md#set-effect-fields) |
| 1 | Breathing | Single, Double, Random | Speed | ✓ [Known](PROTOCOL.md#set-effect-fields) (single, two-colour and random tested) |
| 2 | Color cycle | — | Speed | ✓ [Known](PROTOCOL.md#set-effect-fields) |
| 3 | Reactive | Single, Double, Random | Speed | ✓ [Known](PROTOCOL.md#set-effect-fields) (all colour modes tested) |
| 4 | Rainbow (wave) | Gradient, up to 7 draggable stops | Speed, 9 directions, thickness (Slim/Normal/Thick) | ✓ [Known](PROTOCOL.md#set-effect-fields): gradient, speed, all 9 directions, thickness (byte 8) |
| 5 | Ripple | Gradient (7 stops) or Random | Speed, thickness | ✓ [Known](PROTOCOL.md#set-effect-fields): gradient and random tested; thickness is byte 8 |
| 6 | Starry night | Single, Double, Random | Speed, background (Single color/Off) | ✓ [Known](PROTOCOL.md#set-effect-fields): colours, density (byte 8), background (colour slot 3) |
| 7 | Quicksand | Fixed 6-color pattern or Random | Speed, direction (2 options) | ✓ [Known](PROTOCOL.md#set-effect-fields): 6 colours (one per row), both directions, random |
| 8 | Current | Single, Double, Random | Speed, background | ✓ [Known](PROTOCOL.md#set-effect-fields): single and two-colour tested; background = slot 3 (tested on Starry night and Rain drop) |
| 9 | Raindrop | Single, Double, Random | Speed, background | ✓ [Known](PROTOCOL.md#set-effect-fields): single, random and background (slot 3) tested |
| — | Per-key direct color | Any RGB per key | — | ✓ [Known](PROTOCOL.md#per-key-led-ids-) (`C0 81`, after direct effect `0F`), wired and wireless. [LED ID](PROTOCOL.md#per-key-led-ids-) = column × 8 + row, verified for every key. **Not stored on the keyboard**: the app must re-send colours after a power cycle or reconnect |

Differences from [PROTOCOL.md](PROTOCOL.md#set-effect-fields). The HAL also encodes some bytes differently from OpenRGB; see [Set effect: HAL reading vs hardware](PROTOCOL.md#set-effect-hal-reading-vs-hardware).
- **Speed:** Armoury Crate offers 3 levels (`0` Fast, `1` Normal, `2` Slow), mapped to bytes through a per-effect table built into the driver. ✓ The keyboard takes a raw byte, lower = faster (`00` very fast, `80` moderate, `FF` very slow). Armoury Crate's exact three values are unknown.
- **Directions:** ✓ all 9 IDs (0–8) tested: four straight, four diagonal, and centre-out. OpenRGB lists 6 and mislabels 1 as "vertical" (it's a diagonal). See [direction](PROTOCOL.md#set-effect-fields).
- **Ripple colors:** 7 gradient stops in Armoury Crate; OpenRGB allows 1–8.
- **Off:** Armoury Crate has no "Off" effect. Brightness 0% turns the lights off.
- When wireless power saving is on, Armoury Crate blocks lighting changes.

Factory default: Rainbow wave, 50% brightness, Normal speed, Normal thickness, 7-stop rainbow gradient.

## Keys (remapping)

Any key except **Fn** can be assigned one of:

Armoury Crate's single-key remap command [`51 20 <src> <dst>`](PROTOCOL.md#commands-1) is accepted by the keyboard but **had no effect in hardware tests**, live or saved, with every plausible key-code format. Remapping may happen on the host (Armoury Crate acting on key events) rather than in the keyboard. Until a USB capture shows otherwise, treat remapping as not available from this app.

| Assignment | Options | Protocol |
|---|---|---|
| Default | Restore the original key | Unknown (`51 20` has no visible effect) |
| Keyboard function | One key or a key combination | Unknown (`51 20` accepted, no effect) |
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
| Touch bar display | The LED strip shows the battery gauge (blue on battery, green while charging), or a full strip; it flashes red at the low-battery threshold | ✓ [Known](PROTOCOL.md#commands-1) `51 0A` (1 = battery gauge, 0 = full strip). `51 0C` acknowledged but has no visible effect |
| Lock | Fn + Tab locks the slider (on the keyboard itself) | N/A |

## Power (wireless)

| Feature | Options | Protocol |
|---|---|---|
| Sleep after idle | 1, 2, 3, 5 or 10 minutes, or Never (default: 3 minutes) | ✓ [Known](PROTOCOL.md#commands-1) `51 38`, read back in `12 01` |
| Low-battery alert and power saving | Alert and enter power saving at 10/20/30/40/50% battery, or Never | ✓ [Known](PROTOCOL.md#commands-1) `51 3A`. Below the threshold the lighting turns off and the touch bar flashes red as a battery gauge |
| Power saving / mode | Power saving on/off, plus what happens: "Turn off" (default) or "Decrease brightness" | ✓ [Known](PROTOCOL.md#commands-1) `51 39`: 0 off, 1 turn off, 2 dim |
| Battery level / charging | Shown in Armoury Crate | ✓ [`12 01`](PROTOCOL.md#replies): battery %, level in tens, voltage (mV) and charging flag |
| Dongle firmware / connection | 2.4 GHz model shows the dongle version | ✓ [Known](PROTOCOL.md#replies): through the dongle, `12 00` reports dongle and keyboard firmware separately |

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

In Armoury Crate, the UI sends high-level JSON to a Windows service. The keyboard SDK (`ArmouryKbSDK.dll`) passes it to the keyboard HAL (`AacKbHal_x64.dll`, class `AacM601`), which builds the HID reports. The commands found there are in [PROTOCOL.md](PROTOCOL.md#from-armoury-crates-keyboard-hal). Everything testable from macOS has been [tested on a real keyboard](PROTOCOL.md#hardware-test-results), wired and wireless (✓ above). [Still unknown](PROTOCOL.md#still-unknown) lists what's left. Most of it (key assignments, macros, touch bar actions, gaming mode) likely needs a USB capture of Armoury Crate on Windows.

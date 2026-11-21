//! Wire format of HID reports exchanged with the keyboard.
//!
//! Layout: `[command, sub-command, arguments..., zero padding]`, always
//! [`REPORT_LEN`] bytes (the transport adds the HID report-ID byte). Replies
//! echo the two command bytes; a rejected command is answered with
//! [`REJECTED`]. Byte values are documented in `docs/PROTOCOL.md`.

use std::fmt;

use crate::color::Rgb;
use crate::error::{Error, Result};
use crate::lighting::{ColorMode, Effect, LightingSettings, Thickness};

/// Size of every report, excluding the HID report-ID byte.
pub const REPORT_LEN: usize = 64;
/// Bytes before the arguments: command + sub-command.
pub const HEADER_LEN: usize = 2;
/// Largest argument list a single report can carry.
pub const MAX_ARGS: usize = REPORT_LEN - HEADER_LEN;
/// Reply the keyboard sends instead of an echo when it rejects a command.
pub const REJECTED: [u8; 2] = [0xFF, 0xAA];

/// A report as raw bytes.
pub type RawReport = [u8; REPORT_LEN];

/// Commands this crate sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `12 00`: firmware, active profile and active effect.
    DeviceInfo,
    /// `51 2C`: apply a lighting effect (volatile until saved).
    SetEffect,
    /// `50 55`: save current settings to the active onboard profile.
    Save,
}

impl Command {
    /// Every command.
    pub const ALL: [Command; 3] = [Command::DeviceInfo, Command::SetEffect, Command::Save];

    /// Command and sub-command bytes.
    pub const fn bytes(self) -> [u8; 2] {
        match self {
            Command::DeviceInfo => [0x12, 0x00],
            Command::SetEffect => [0x51, 0x2C],
            Command::Save => [0x50, 0x55],
        }
    }

    /// Looks up a command by its first two bytes.
    pub fn from_bytes(bytes: [u8; 2]) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.bytes() == bytes)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b] = self.bytes();
        write!(f, "{a:02X} {b:02X}")
    }
}

/// One 64-byte report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    bytes: RawReport,
}

impl Report {
    /// Builds a report from a command and its arguments (from byte 2).
    pub fn new(command: Command, args: &[u8]) -> Result<Self> {
        if args.len() > MAX_ARGS {
            return Err(Error::PayloadTooLong {
                len: args.len(),
                max: MAX_ARGS,
            });
        }
        let mut bytes = [0u8; REPORT_LEN];
        bytes[..HEADER_LEN].copy_from_slice(&command.bytes());
        bytes[HEADER_LEN..HEADER_LEN + args.len()].copy_from_slice(args);
        Ok(Self { bytes })
    }

    /// Wraps bytes received from the device.
    pub const fn from_raw(bytes: RawReport) -> Self {
        Self { bytes }
    }

    /// Request for device information (`12 00`).
    pub fn device_info() -> Self {
        Self::bare(Command::DeviceInfo)
    }

    /// Request to save to the active profile (`50 55`).
    pub fn save() -> Self {
        Self::bare(Command::Save)
    }

    /// Request to apply `settings` (`51 2C`). Fails if they are invalid.
    pub fn set_effect(settings: &LightingSettings) -> Result<Self> {
        settings.validate()?;
        let effect = settings.effect;
        let mode = if effect.color_modes().is_empty() {
            ColorMode::Single
        } else {
            settings.color_mode
        };
        let direction = if effect.uses_direction() {
            settings.direction.code()
        } else {
            0
        };
        let thickness = if effect.uses_thickness() {
            settings.thickness.code()
        } else {
            Thickness::UNUSED_CODE
        };
        let mut args = vec![
            effect.code(),
            0x00,
            settings.speed.code(),
            settings.brightness.percent(),
            mode.code(),
            direction,
            thickness,
        ];
        let colors = settings.active_colors();
        if mode == ColorMode::Gradient && effect == Effect::Quicksand {
            // One R G B per row from byte 9, top row first.
            args.extend(colors.iter().flat_map(|c| c.to_bytes()));
        } else if mode == ColorMode::Gradient {
            args.extend(gradient_bytes(colors));
        } else {
            // R G B slots 1-3 from byte 9; slot 3 is the background.
            let mut slots = [Rgb::BLACK; 3];
            for (slot, color) in slots.iter_mut().zip(colors) {
                *slot = *color;
            }
            if let Some(background) = settings.background {
                slots[2] = background;
            }
            args.extend(slots.iter().flat_map(|c| c.to_bytes()));
        }
        Self::new(Command::SetEffect, &args)
    }

    fn bare(command: Command) -> Self {
        Self::new(command, &[]).expect("no arguments")
    }

    /// The first two bytes: command and sub-command.
    pub fn command_bytes(&self) -> [u8; 2] {
        [self.bytes[0], self.bytes[1]]
    }

    /// The command, if it is one this crate knows.
    pub fn command(&self) -> Option<Command> {
        Command::from_bytes(self.command_bytes())
    }

    /// Every byte.
    pub fn bytes(&self) -> &RawReport {
        &self.bytes
    }

    /// Bytes after the command.
    pub fn args(&self) -> &[u8] {
        &self.bytes[HEADER_LEN..]
    }

    /// Serializes to a raw report.
    pub fn encode(&self) -> RawReport {
        self.bytes
    }

    /// Whether this is the keyboard's rejection reply (`FF AA`).
    pub fn is_rejection(&self) -> bool {
        self.command_bytes() == REJECTED
    }

    /// Whether this reply acknowledges `request` by echoing its command.
    pub fn acknowledges(&self, request: &Report) -> bool {
        self.command_bytes() == request.command_bytes()
    }
}

/// Wave/Ripple colors: count, then `position% R G B` per stop.
fn gradient_bytes(colors: &[Rgb]) -> Vec<u8> {
    let count = colors.len();
    let mut out = vec![count as u8];
    for (i, color) in colors.iter().enumerate() {
        out.push((100 * (i + 1) / count) as u8);
        out.extend(color.to_bytes());
    }
    out
}

/// What the device-info reply (`12 00`) reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceStatus {
    /// Keyboard firmware, e.g. `13.61.31`. `None` when a dongle reports no
    /// keyboard connected.
    pub firmware: Option<String>,
    /// Dongle firmware, when connected through the 2.4 GHz dongle.
    pub dongle_firmware: Option<String>,
    /// Active onboard profile (1–6).
    pub profile: u8,
    /// Raw mode byte of the active effect (15 = per-key direct mode).
    pub effect_code: u8,
}

impl DeviceStatus {
    /// The active effect, if it is a built-in one.
    pub fn effect(&self) -> Option<Effect> {
        Effect::from_code(self.effect_code)
    }
}

/// Parses a device-info reply.
pub fn parse_device_info(report: &Report) -> Result<DeviceStatus> {
    if report.command() != Some(Command::DeviceInfo) {
        let [a, b] = report.command_bytes();
        let msg = format!("expected device info reply, got {a:02X} {b:02X}");
        return Err(Error::MalformedReport(msg));
    }
    let data = report.bytes();
    let version = |at: usize| format!("{:x}.{:x}.{:x}", data[at], data[at + 1], data[at + 2]);
    let (firmware, dongle_firmware) = match data[11] {
        0x00 => (Some(version(4)), None),
        0xFF => (None, Some(version(4))),
        _ => (Some(version(12)), Some(version(4))),
    };
    Ok(DeviceStatus {
        firmware,
        dongle_firmware,
        profile: data[10],
        effect_code: data[9],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::{Brightness, Direction, Speed, RAINBOW};

    fn effect_bytes(settings: &LightingSettings) -> RawReport {
        Report::set_effect(settings).unwrap().encode()
    }

    #[test]
    fn command_bytes_round_trip() {
        for command in Command::ALL {
            assert_eq!(Command::from_bytes(command.bytes()), Some(command));
        }
        assert_eq!(Command::from_bytes([0, 0]), None);
        assert_eq!(Command::SetEffect.to_string(), "51 2C");
    }

    #[test]
    fn new_accepts_max_args_and_rejects_longer() {
        assert!(Report::new(Command::Save, &[0; MAX_ARGS]).is_ok());
        let err = Report::new(Command::Save, &[0; MAX_ARGS + 1]).unwrap_err();
        let expected = Error::PayloadTooLong {
            len: MAX_ARGS + 1,
            max: MAX_ARGS,
        };
        assert_eq!(err, expected);
    }

    #[test]
    fn encode_layout() {
        let raw = Report::new(Command::Save, &[7, 8]).unwrap().encode();
        assert_eq!(&raw[..4], &[0x50, 0x55, 7, 8]);
        assert!(raw[4..].iter().all(|&b| b == 0));
        assert_eq!(&Report::device_info().encode()[..2], &[0x12, 0x00]);
    }

    #[test]
    fn raw_reports_expose_command_and_args() {
        let mut raw = [0u8; REPORT_LEN];
        raw[..3].copy_from_slice(&[0x51, 0x2C, 9]);
        let report = Report::from_raw(raw);
        assert_eq!(report.command(), Some(Command::SetEffect));
        assert_eq!(report.args()[0], 9);
        assert_eq!(report.bytes(), &raw);
        raw[0] = 0x99;
        assert_eq!(Report::from_raw(raw).command(), None);
    }

    #[test]
    fn rejection_and_acknowledgement() {
        let mut raw = [0u8; REPORT_LEN];
        raw[..2].copy_from_slice(&REJECTED);
        let rejection = Report::from_raw(raw);
        assert!(rejection.is_rejection());
        assert!(!rejection.acknowledges(&Report::save()));
        assert!(Report::save().acknowledges(&Report::save()));
        assert!(!Report::save().is_rejection());
        assert!(!Report::device_info().acknowledges(&Report::save()));
    }

    #[test]
    fn static_matches_hardware_test() {
        // PROTOCOL.md: `51 2C 00 00 <speed> 64 00 00 02 00 00 FF` = solid blue.
        let settings = LightingSettings {
            colors: vec![Rgb::BLUE],
            ..LightingSettings::for_effect(Effect::Static)
        };
        let raw = effect_bytes(&settings);
        let speed = Speed::default().code();
        let expected = [0x51, 0x2C, 0, 0, speed, 0x64, 0, 0, 2, 0, 0, 0xFF];
        assert_eq!(&raw[..12], &expected);
        assert!(raw[12..].iter().all(|&b| b == 0));
    }

    #[test]
    fn breathing_double_uses_slots_one_and_two() {
        // PROTOCOL.md: `51 2C 01 00 1E 64 10 00 02 FF 00 00 00 00 FF`.
        let settings = LightingSettings {
            color_mode: ColorMode::Double,
            colors: vec![Rgb::RED, Rgb::BLUE],
            ..LightingSettings::for_effect(Effect::Breathing)
        };
        let raw = effect_bytes(&settings);
        assert_eq!(
            &raw[2..9],
            &[1, 0, Speed::default().code(), 0x64, 0x10, 0, 2]
        );
        assert_eq!(&raw[9..18], &[0xFF, 0, 0, 0, 0, 0xFF, 0, 0, 0]);
    }

    #[test]
    fn random_mode_sends_no_colors() {
        let settings = LightingSettings {
            color_mode: ColorMode::Random,
            ..LightingSettings::for_effect(Effect::Reactive)
        };
        let raw = effect_bytes(&settings);
        assert_eq!(raw[6], 0x01);
        assert!(raw[9..].iter().all(|&b| b == 0));
    }

    #[test]
    fn wave_matches_hardware_test() {
        // PROTOCOL.md: `51 2C 04 00 80 64 00 00 02 03 21 FF 00 00 42 00 FF 00 64 00 00 FF`.
        let settings = LightingSettings {
            colors: vec![Rgb::RED, Rgb::GREEN, Rgb::BLUE],
            brightness: Brightness::new(4).unwrap(),
            direction: Direction::Right,
            thickness: Thickness::Thick,
            ..LightingSettings::for_effect(Effect::Wave)
        };
        let raw = effect_bytes(&settings);
        let speed = Speed::default().code();
        let expected = [
            0x51, 0x2C, 4, 0, speed, 0x64, 0, 0, 2, 3, 0x21, 0xFF, 0, 0, 0x42, 0, 0xFF, 0, 0x64, 0,
            0, 0xFF,
        ];
        assert_eq!(&raw[..22], &expected);
    }

    #[test]
    fn wave_encodes_direction_and_thickness() {
        let settings = LightingSettings {
            direction: Direction::Out,
            thickness: Thickness::Slim,
            ..LightingSettings::for_effect(Effect::Wave)
        };
        let raw = effect_bytes(&settings);
        assert_eq!((raw[7], raw[8]), (8, 0));
        assert_eq!(raw[9], 7);
        assert_eq!(raw[10 + 6 * 4], 100);
    }

    #[test]
    fn non_directional_effects_send_fixed_direction_and_size() {
        let settings = LightingSettings {
            direction: Direction::Left,
            thickness: Thickness::Slim,
            ..LightingSettings::for_effect(Effect::Breathing)
        };
        let raw = effect_bytes(&settings);
        assert_eq!((raw[7], raw[8]), (0, Thickness::UNUSED_CODE));
    }

    #[test]
    fn background_goes_in_slot_three() {
        let settings = LightingSettings {
            colors: vec![Rgb::WHITE],
            background: Some(Rgb::BLUE),
            ..LightingSettings::for_effect(Effect::StarryNight)
        };
        let raw = effect_bytes(&settings);
        assert_eq!(&raw[9..18], &[0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0xFF]);
    }

    #[test]
    fn quicksand_sends_six_row_colors_and_direction() {
        let settings = LightingSettings::for_effect(Effect::Quicksand);
        let raw = effect_bytes(&settings);
        assert_eq!((raw[2], raw[6], raw[7]), (7, 0, 6));
        let rows: Vec<u8> = RAINBOW[..6].iter().flat_map(|c| c.to_bytes()).collect();
        assert_eq!(&raw[9..27], rows.as_slice());
        assert!(raw[27..].iter().all(|&b| b == 0));
    }

    #[test]
    fn color_cycle_sends_single_mode_without_colors() {
        let mut settings = LightingSettings::for_effect(Effect::ColorCycle);
        settings.color_mode = ColorMode::Double;
        let raw = effect_bytes(&settings);
        assert_eq!((raw[2], raw[6]), (2, 0));
        assert!(raw[9..].iter().all(|&b| b == 0));
    }

    #[test]
    fn brightness_zero_turns_keys_off() {
        let settings = LightingSettings {
            brightness: Brightness::new(0).unwrap(),
            ..LightingSettings::for_effect(Effect::Static)
        };
        assert_eq!(effect_bytes(&settings)[5], 0);
    }

    #[test]
    fn set_effect_rejects_invalid_settings() {
        let mut settings = LightingSettings::for_effect(Effect::Wave);
        settings.colors.clear();
        let result = Report::set_effect(&settings);
        assert!(matches!(result, Err(Error::InvalidSettings(_))));
    }

    fn info_reply(fill: &[(usize, u8)]) -> Report {
        let mut raw = [0u8; REPORT_LEN];
        raw[..2].copy_from_slice(&[0x12, 0x00]);
        for &(at, value) in fill {
            raw[at] = value;
        }
        Report::from_raw(raw)
    }

    #[test]
    fn parses_wired_device_info() {
        let reply = info_reply(&[(4, 0x13), (5, 0x61), (6, 0x31), (9, 4), (10, 2)]);
        let status = parse_device_info(&reply).unwrap();
        assert_eq!(status.firmware.as_deref(), Some("13.61.31"));
        assert_eq!(status.dongle_firmware, None);
        assert_eq!(status.profile, 2);
        assert_eq!(status.effect(), Some(Effect::Wave));
    }

    #[test]
    fn parses_dongle_device_info() {
        let fill = [
            (4, 7),
            (5, 0),
            (6, 2),
            (11, 1),
            (12, 0x13),
            (13, 0x61),
            (14, 0x31),
        ];
        let status = parse_device_info(&info_reply(&fill)).unwrap();
        assert_eq!(status.firmware.as_deref(), Some("13.61.31"));
        assert_eq!(status.dongle_firmware.as_deref(), Some("7.0.2"));

        let status = parse_device_info(&info_reply(&[(4, 7), (11, 0xFF)])).unwrap();
        assert_eq!(status.firmware, None);
        assert!(status.dongle_firmware.is_some());
    }

    #[test]
    fn device_info_reports_unknown_effects() {
        let status = parse_device_info(&info_reply(&[(9, 15)])).unwrap();
        assert_eq!(status.effect_code, 15);
        assert_eq!(status.effect(), None);
    }

    #[test]
    fn device_info_rejects_other_replies() {
        let err = parse_device_info(&Report::save()).unwrap_err();
        assert!(matches!(err, Error::MalformedReport(m) if m.contains("50 55")));
    }
}

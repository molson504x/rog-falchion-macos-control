//! Wire format of HID reports exchanged with the keyboard.
//!
//! Every constant and command code here is a PLACEHOLDER until confirmed by
//! capturing Armoury Crate traffic. See `docs/PROTOCOL.md`.
//!
//! Layout: `[REPORT_MARKER, command, payload..., zero padding]`, always
//! [`REPORT_LEN`] bytes (the HID report-ID byte is added by the transport).

use crate::error::{Error, Result};
use crate::lighting::LightingSettings;

/// Size of every report, excluding the HID report-ID byte.
pub const REPORT_LEN: usize = 64;
/// Bytes before the payload: marker + command.
pub const HEADER_LEN: usize = 2;
/// Largest payload a single report can carry.
pub const MAX_PAYLOAD: usize = REPORT_LEN - HEADER_LEN;
/// First byte of every report. PLACEHOLDER.
pub const REPORT_MARKER: u8 = 0xEC;

/// A report as raw bytes.
pub type RawReport = [u8; REPORT_LEN];

/// Commands understood by the keyboard. PLACEHOLDER codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Query the firmware version.
    GetFirmwareVersion,
    /// Apply lighting settings (volatile until saved).
    SetLighting,
    /// Persist current settings to the active onboard profile.
    Save,
}

impl Command {
    /// Every command.
    pub const ALL: [Command; 3] = [
        Command::GetFirmwareVersion,
        Command::SetLighting,
        Command::Save,
    ];

    /// Wire code. PLACEHOLDER.
    pub const fn code(self) -> u8 {
        match self {
            Command::GetFirmwareVersion => 0x82,
            Command::SetLighting => 0x35,
            Command::Save => 0x50,
        }
    }

    /// Looks up a command by wire code.
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.code() == code)
    }
}

/// A decoded report: a command plus its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    command: Command,
    payload: Vec<u8>,
}

impl Report {
    /// Creates a report, rejecting payloads longer than [`MAX_PAYLOAD`].
    pub fn new(command: Command, payload: &[u8]) -> Result<Self> {
        if payload.len() > MAX_PAYLOAD {
            return Err(Error::PayloadTooLong {
                len: payload.len(),
                max: MAX_PAYLOAD,
            });
        }
        Ok(Self {
            command,
            payload: payload.to_vec(),
        })
    }

    /// Request for the firmware version.
    pub fn get_firmware_version() -> Self {
        Self::bare(Command::GetFirmwareVersion)
    }

    /// Request to persist the current settings.
    pub fn save() -> Self {
        Self::bare(Command::Save)
    }

    /// Request to apply `settings`.
    pub fn set_lighting(settings: &LightingSettings) -> Self {
        let [r, g, b] = settings.color.to_bytes();
        let payload = vec![
            settings.effect.code(),
            settings.brightness.value(),
            settings.speed.value(),
            r,
            g,
            b,
            settings.direction.code(),
        ];
        Self {
            command: Command::SetLighting,
            payload,
        }
    }

    fn bare(command: Command) -> Self {
        Self {
            command,
            payload: Vec::new(),
        }
    }

    /// The report's command.
    pub fn command(&self) -> Command {
        self.command
    }

    /// The payload. Decoded reports include trailing zero padding.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Serializes to a zero-padded raw report.
    pub fn encode(&self) -> RawReport {
        let mut raw = [0u8; REPORT_LEN];
        raw[0] = REPORT_MARKER;
        raw[1] = self.command.code();
        let end = HEADER_LEN + self.payload.len();
        raw[HEADER_LEN..end].copy_from_slice(&self.payload);
        raw
    }

    /// Parses a raw report received from the device.
    pub fn decode(raw: &[u8]) -> Result<Self> {
        if raw.len() != REPORT_LEN {
            let msg = format!("expected {REPORT_LEN} bytes, got {}", raw.len());
            return Err(Error::MalformedReport(msg));
        }
        if raw[0] != REPORT_MARKER {
            let msg = format!("unexpected marker 0x{:02x}", raw[0]);
            return Err(Error::MalformedReport(msg));
        }
        let Some(command) = Command::from_code(raw[1]) else {
            let msg = format!("unknown command 0x{:02x}", raw[1]);
            return Err(Error::MalformedReport(msg));
        };
        Ok(Self {
            command,
            payload: raw[HEADER_LEN..].to_vec(),
        })
    }
}

/// Parses a firmware-version response as `major.minor.patch`.
pub fn parse_firmware_version(report: &Report) -> Result<String> {
    let command = report.command();
    if command != Command::GetFirmwareVersion {
        let msg = format!("expected firmware response, got {command:?}");
        return Err(Error::MalformedReport(msg));
    }
    match report.payload() {
        [major, minor, patch, ..] => Ok(format!("{major}.{minor}.{patch}")),
        _ => Err(Error::MalformedReport("firmware response too short".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::lighting::{Brightness, Direction, Effect, Speed};

    const FW: Command = Command::GetFirmwareVersion;

    #[test]
    fn command_codes_round_trip() {
        for command in Command::ALL {
            assert_eq!(Command::from_code(command.code()), Some(command));
        }
        assert_eq!(Command::from_code(0x00), None);
    }

    #[test]
    fn new_accepts_max_payload_and_rejects_longer() {
        assert!(Report::new(Command::Save, &[0; MAX_PAYLOAD]).is_ok());
        let too_long = [0; MAX_PAYLOAD + 1];
        let err = Report::new(Command::Save, &too_long).unwrap_err();
        let expected = Error::PayloadTooLong {
            len: MAX_PAYLOAD + 1,
            max: MAX_PAYLOAD,
        };
        assert_eq!(err, expected);
    }

    #[test]
    fn encode_layout() {
        let raw = Report::new(Command::Save, &[7, 8]).unwrap().encode();
        assert_eq!(&raw[..4], &[REPORT_MARKER, Command::Save.code(), 7, 8]);
        assert!(raw[4..].iter().all(|&b| b == 0));
    }

    #[test]
    fn decode_round_trips_with_padding() {
        let report = Report::new(Command::SetLighting, &[1, 2, 3]).unwrap();
        let decoded = Report::decode(&report.encode()).unwrap();
        assert_eq!(decoded.command(), Command::SetLighting);
        assert_eq!(&decoded.payload()[..3], &[1, 2, 3]);
        assert_eq!(decoded.payload().len(), MAX_PAYLOAD);
    }

    #[test]
    fn decode_rejects_wrong_length() {
        let err = Report::decode(&[REPORT_MARKER; 10]).unwrap_err();
        assert!(matches!(err, Error::MalformedReport(m) if m.contains("got 10")));
    }

    #[test]
    fn decode_rejects_bad_marker() {
        let mut raw = Report::save().encode();
        raw[0] = 0x00;
        let result = Report::decode(&raw);
        assert!(matches!(result, Err(Error::MalformedReport(_))));
    }

    #[test]
    fn decode_rejects_unknown_command() {
        let mut raw = Report::save().encode();
        raw[1] = 0x00;
        let err = Report::decode(&raw).unwrap_err();
        assert!(matches!(err, Error::MalformedReport(m) if m.contains("0x00")));
    }

    #[test]
    fn set_lighting_payload() {
        let settings = LightingSettings {
            effect: Effect::Wave,
            color: Rgb::new(10, 20, 30),
            brightness: Brightness::new(2).unwrap(),
            speed: Speed::new(0).unwrap(),
            direction: Direction::Left,
        };
        let report = Report::set_lighting(&settings);
        assert_eq!(report.command(), Command::SetLighting);
        let (wave, left) = (Effect::Wave.code(), Direction::Left.code());
        let expected = [wave, 2, 0, 10, 20, 30, left];
        assert_eq!(report.payload(), &expected);
    }

    #[test]
    fn bare_requests_have_empty_payload() {
        assert!(Report::save().payload().is_empty());
        let request = Report::get_firmware_version();
        assert_eq!(request.command(), FW);
    }

    #[test]
    fn parses_firmware_version() {
        let report = Report::new(FW, &[1, 2, 3]).unwrap();
        assert_eq!(parse_firmware_version(&report), Ok("1.2.3".to_string()));
    }

    #[test]
    fn firmware_version_rejects_wrong_command_and_short_payload() {
        assert!(parse_firmware_version(&Report::save()).is_err());
        let short = Report::new(FW, &[1]).unwrap();
        assert!(parse_firmware_version(&short).is_err());
    }
}

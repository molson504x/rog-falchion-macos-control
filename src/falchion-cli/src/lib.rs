//! Command-line interface for the ROG Falchion.
//!
//! [`run`] is generic over [`DeviceProvider`] so commands are unit-tested
//! against [`falchion_core::MockProvider`].

use std::io::Write;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use falchion_core::{
    Brightness, DeviceProvider, Direction, Effect, Error, Keyboard, LightingSettings, RawReport,
    Rgb, Speed, Transport, REPORT_LEN,
};

/// Control the ASUS ROG Falchion keyboard.
#[derive(Debug, Parser)]
#[command(name = "falchion", version, about)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// List connected Falchion keyboards.
    List,
    /// Show firmware information for the first connected keyboard.
    Info,
    /// List supported lighting effects.
    Effects,
    /// Set the keyboard lighting.
    Lighting(LightingArgs),
    /// Developer tool: send one raw 64-byte report and print the replies.
    ///
    /// Bytes are hex, e.g. `falchion raw 12 00`. Unspecified bytes are zero.
    /// Use with care: set commands change the keyboard's settings.
    Raw(RawArgs),
}

/// Arguments for `falchion raw`.
#[derive(Debug, Args)]
pub struct RawArgs {
    /// Report bytes in hex ("12 00", "0x12 0x00" or "1200").
    #[arg(required = true, num_args = 1..)]
    pub bytes: Vec<String>,
    /// How long to wait for each reply, in milliseconds.
    #[arg(long, default_value_t = 300)]
    pub timeout: i32,
    /// Stop after this many replies.
    #[arg(long, default_value_t = 4)]
    pub max_replies: usize,
}

/// Parses hex tokens into report bytes.
///
/// Each token may hold several bytes ("512c", "51 2c") and may use a `0x`
/// prefix or `:`/`-`/whitespace separators. Fails on invalid hex, an odd digit count, no bytes, or
/// more than [`REPORT_LEN`] bytes.
pub fn parse_hex_bytes(tokens: &[String]) -> std::result::Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for token in tokens {
        let digits: String = token
            .split(|c: char| c == ':' || c == '-' || c.is_whitespace())
            .map(|part| part.trim_start_matches("0x").trim_start_matches("0X"))
            .collect();
        if !digits.len().is_multiple_of(2) {
            return Err(format!("odd number of hex digits in {token:?}"));
        }
        for pair in digits.as_bytes().chunks(2) {
            let pair = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            let byte = u8::from_str_radix(pair, 16);
            bytes.push(byte.map_err(|_| format!("invalid hex {pair:?} in {token:?}"))?);
        }
    }
    if bytes.is_empty() {
        return Err("no bytes given".into());
    }
    if bytes.len() > REPORT_LEN {
        let len = bytes.len();
        return Err(format!("{len} bytes given, a report holds {REPORT_LEN}"));
    }
    Ok(bytes)
}

/// Formats a report as hex, dropping trailing zeros (at least 8 bytes shown).
pub fn format_hex(report: &[u8]) -> String {
    let last = report.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let shown = &report[..last.max(8).min(report.len())];
    let mut text: Vec<String> = shown.iter().map(|b| format!("{b:02X}")).collect();
    if shown.len() < report.len() {
        text.push("…".into());
    }
    text.join(" ")
}

/// Arguments for `falchion lighting`.
#[derive(Debug, Args)]
pub struct LightingArgs {
    /// Effect name (see `falchion effects`).
    #[arg(long, default_value = "static")]
    pub effect: Effect,
    /// Color as hex, e.g. "#ff8800".
    #[arg(long, default_value = "#ffffff")]
    pub color: Rgb,
    /// Brightness from 0 (off) to 4 (max).
    #[arg(long, default_value_t = Brightness::MAX)]
    pub brightness: u8,
    /// Animation speed from 0 (slow) to 2 (fast).
    #[arg(long, default_value_t = 1)]
    pub speed: u8,
    /// Wave direction: left or right.
    #[arg(long, default_value = "right")]
    pub direction: Direction,
    /// Save to the keyboard's active profile so it survives power cycles.
    #[arg(long)]
    pub save: bool,
}

impl LightingArgs {
    /// Validates the arguments into [`LightingSettings`].
    pub fn to_settings(&self) -> falchion_core::Result<LightingSettings> {
        Ok(LightingSettings {
            effect: self.effect,
            color: self.color,
            brightness: Brightness::new(self.brightness)?,
            speed: Speed::new(self.speed)?,
            direction: self.direction,
        })
    }
}

/// Executes `cli` against devices from `provider`, writing output to `out`.
pub fn run<P: DeviceProvider>(cli: &Cli, provider: &P, out: &mut dyn Write) -> Result<()> {
    match &cli.command {
        Command::List => list(provider, out),
        Command::Info => info(provider, out),
        Command::Effects => effects(out),
        Command::Lighting(args) => lighting(provider, args, out),
        Command::Raw(args) => raw(provider, args, out),
    }
}

fn list<P: DeviceProvider>(provider: &P, out: &mut dyn Write) -> Result<()> {
    let devices = provider.list()?;
    if devices.is_empty() {
        writeln!(out, "No ROG Falchion keyboards found.")?;
    }
    for device in devices {
        writeln!(out, "{device}")?;
    }
    Ok(())
}

fn info<P: DeviceProvider>(provider: &P, out: &mut dyn Write) -> Result<()> {
    let (device, transport) = provider.open_first()?;
    let version = Keyboard::new(transport).firmware_version()?;
    writeln!(out, "{device}")?;
    writeln!(out, "Firmware: {version}")?;
    Ok(())
}

fn effects(out: &mut dyn Write) -> Result<()> {
    for effect in Effect::ALL {
        writeln!(out, "{effect}")?;
    }
    Ok(())
}

fn lighting<P: DeviceProvider>(
    provider: &P,
    args: &LightingArgs,
    out: &mut dyn Write,
) -> Result<()> {
    let settings = args.to_settings()?;
    let (_, transport) = provider.open_first()?;
    let mut keyboard = Keyboard::new(transport);
    keyboard.apply_lighting(&settings)?;
    if args.save {
        keyboard.save()?;
    }
    let effect = settings.effect;
    writeln!(out, "Applied {effect} lighting.")?;
    Ok(())
}

fn raw<P: DeviceProvider>(provider: &P, args: &RawArgs, out: &mut dyn Write) -> Result<()> {
    let bytes = parse_hex_bytes(&args.bytes).map_err(anyhow::Error::msg)?;
    let mut report: RawReport = [0; REPORT_LEN];
    report[..bytes.len()].copy_from_slice(&bytes);
    let (_, mut transport) = provider.open_first()?;
    transport.write(&report)?;
    writeln!(out, "sent:  {}", format_hex(&report))?;
    let mut replies = 0;
    while replies < args.max_replies {
        let reply = match transport.read(args.timeout) {
            Ok(reply) => reply,
            Err(Error::Timeout) => break,
            Err(err) => return Err(err.into()),
        };
        let echo = reply[..2] == report[..2];
        let tag = if echo { "  (echo)" } else { "" };
        writeln!(out, "reply: {}{tag}", format_hex(&reply))?;
        replies += 1;
    }
    if replies == 0 {
        writeln!(out, "no reply within {} ms", args.timeout)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use falchion_core::protocol::{Command as Wire, Report};
    use falchion_core::{Error, MockProvider};

    fn parse(args: &[&str]) -> Cli {
        let argv = std::iter::once("falchion").chain(args.iter().copied());
        Cli::try_parse_from(argv).expect("valid arguments")
    }

    fn exec(args: &[&str], provider: &MockProvider) -> (Result<()>, String) {
        let mut out = Vec::new();
        let result = run(&parse(args), provider, &mut out);
        (result, String::from_utf8(out).unwrap())
    }

    fn wire_commands(provider: &MockProvider) -> Vec<Wire> {
        let mut commands = Vec::new();
        for raw in provider.transport.written() {
            commands.push(Report::decode(&raw).unwrap().command());
        }
        commands
    }

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_rejects_bad_values() {
        let bad = [
            vec!["lighting", "--color", "nothex"],
            vec!["lighting", "--effect", "sparkle"],
            vec!["lighting", "--brightness", "300"],
            vec!["unknown-command"],
            vec![],
        ];
        for args in bad {
            let argv = std::iter::once("falchion").chain(args.clone());
            assert!(Cli::try_parse_from(argv).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn list_prints_devices() {
        let provider = MockProvider::with_device();
        let (result, out) = exec(&["list"], &provider);
        assert!(result.is_ok());
        assert!(out.contains("ROG Falchion"));
    }

    #[test]
    fn list_reports_no_devices() {
        let (result, out) = exec(&["list"], &MockProvider::default());
        assert!(result.is_ok());
        assert!(out.contains("No ROG Falchion"));
    }

    #[test]
    fn list_propagates_errors() {
        let provider = MockProvider {
            list_error: Some(Error::Transport("hid".into())),
            ..MockProvider::default()
        };
        assert!(exec(&["list"], &provider).0.is_err());
    }

    #[test]
    fn info_prints_firmware() {
        let provider = MockProvider::with_device();
        let response = Report::new(Wire::GetFirmwareVersion, &[2, 1, 0]).unwrap();
        provider.transport.queue_response(&response);
        let (result, out) = exec(&["info"], &provider);
        assert!(result.is_ok());
        assert!(out.contains("Firmware: 2.1.0"));
    }

    #[test]
    fn info_without_device_fails() {
        let (result, _) = exec(&["info"], &MockProvider::default());
        let err = result.unwrap_err().downcast::<Error>().unwrap();
        assert_eq!(err, Error::DeviceNotFound);
    }

    #[test]
    fn effects_lists_all() {
        let (result, out) = exec(&["effects"], &MockProvider::default());
        assert!(result.is_ok());
        assert_eq!(out.lines().count(), Effect::ALL.len());
    }

    #[test]
    fn lighting_applies_without_saving_by_default() {
        let provider = MockProvider::with_device();
        let args = ["lighting", "--effect", "wave", "--speed", "2"];
        let (result, out) = exec(&args, &provider);
        assert!(result.is_ok());
        assert!(out.contains("Applied wave lighting."));
        assert_eq!(wire_commands(&provider), [Wire::SetLighting]);
    }

    #[test]
    fn lighting_save_persists() {
        let provider = MockProvider::with_device();
        let (result, _) = exec(&["lighting", "--save"], &provider);
        assert!(result.is_ok());
        let expected = [Wire::SetLighting, Wire::Save];
        assert_eq!(wire_commands(&provider), expected);
    }

    #[test]
    fn lighting_rejects_out_of_range_before_touching_device() {
        let provider = MockProvider::with_device();
        for args in [
            ["lighting", "--brightness", "9"],
            ["lighting", "--speed", "3"],
        ] {
            assert!(exec(&args, &provider).0.is_err());
        }
        assert!(provider.transport.written().is_empty());
    }

    #[test]
    fn lighting_without_device_fails() {
        let (result, _) = exec(&["lighting"], &MockProvider::default());
        assert!(result.is_err());
    }

    fn strings(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn parse_hex_accepts_common_forms() {
        let parsed = parse_hex_bytes(&strings(&["51", "0x2C", "0a0B", "de:ad", "0x01 02"]));
        assert_eq!(
            parsed.unwrap(),
            [0x51, 0x2C, 0x0A, 0x0B, 0xDE, 0xAD, 0x01, 0x02]
        );
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        let too_long = vec!["00".to_string(); REPORT_LEN + 1];
        for tokens in [strings(&["5"]), strings(&["zz"]), strings(&[""]), too_long] {
            assert!(parse_hex_bytes(&tokens).is_err(), "accepted {tokens:?}");
        }
    }

    #[test]
    fn format_hex_trims_trailing_zeros() {
        let mut report = [0u8; REPORT_LEN];
        report[..2].copy_from_slice(&[0x12, 0x00]);
        assert_eq!(format_hex(&report), "12 00 00 00 00 00 00 00 …");
        report[9] = 0xAB;
        assert_eq!(format_hex(&report), "12 00 00 00 00 00 00 00 00 AB …");
        assert_eq!(format_hex(&[0xFF; 4]), "FF FF FF FF");
    }

    #[test]
    fn raw_sends_bytes_and_prints_echo() {
        let provider = MockProvider::with_device();
        let mut reply = [0u8; REPORT_LEN];
        reply[..7].copy_from_slice(&[0x12, 0x00, 0, 0, 1, 2, 3]);
        provider.transport.queue_raw(reply);
        let (result, out) = exec(&["raw", "12", "00"], &provider);
        assert!(result.is_ok());
        assert_eq!(provider.transport.written()[0][..2], [0x12, 0x00]);
        assert!(out.contains("reply: 12 00 00 00 01 02 03 00 …  (echo)"));
    }

    #[test]
    fn raw_marks_unrelated_reports_and_honours_max() {
        let provider = MockProvider::with_device();
        for first in [0x43, 0x43, 0x43] {
            let mut reply = [0u8; REPORT_LEN];
            reply[0] = first;
            provider.transport.queue_raw(reply);
        }
        let (result, out) = exec(&["raw", "12", "--max-replies", "2"], &provider);
        assert!(result.is_ok());
        assert_eq!(out.matches("reply:").count(), 2);
        assert!(!out.contains("(echo)"));
    }

    #[test]
    fn raw_reports_silence() {
        let provider = MockProvider::with_device();
        let (result, out) = exec(&["raw", "12", "00", "--timeout", "5"], &provider);
        assert!(result.is_ok());
        assert!(out.contains("no reply within 5 ms"));
    }

    #[test]
    fn raw_rejects_bad_hex_before_touching_device() {
        let provider = MockProvider::with_device();
        assert!(exec(&["raw", "xyz"], &provider).0.is_err());
        assert!(provider.transport.written().is_empty());
    }

    #[test]
    fn raw_propagates_transport_errors() {
        let provider = MockProvider::with_device();
        provider
            .transport
            .queue_error(Error::Transport("unplugged".into()));
        assert!(exec(&["raw", "12"], &provider).0.is_err());

        let provider = MockProvider::with_device();
        provider
            .transport
            .fail_writes_with(Error::Transport("busy".into()));
        assert!(exec(&["raw", "12"], &provider).0.is_err());
        assert!(exec(&["raw", "12"], &MockProvider::default()).0.is_err());
    }
}

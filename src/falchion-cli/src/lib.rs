//! Command-line interface for the ROG Falchion.
//!
//! [`run`] is generic over [`DeviceProvider`] so commands are unit-tested
//! against [`falchion_core::MockProvider`].

use std::io::Write;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use falchion_core::{
    Brightness, DeviceProvider, Direction, Effect, Keyboard, LightingSettings, Rgb, Speed,
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
}

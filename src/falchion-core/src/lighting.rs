//! Lighting settings and their validated value types.
//!
//! Wire codes (`code()`) and ranges are PLACEHOLDERS until verified against
//! captured Armoury Crate traffic. See `docs/PROTOCOL.md`.

use std::fmt;
use std::str::FromStr;

use crate::color::Rgb;
use crate::error::{Error, Result};

/// A lighting effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Effect {
    /// One solid color.
    #[default]
    Static,
    /// Fades the color in and out.
    Breathing,
    /// Cycles through the color spectrum.
    ColorCycle,
    /// A rainbow wave across the keyboard.
    Wave,
    /// Keys light up when pressed.
    Reactive,
    /// Lighting disabled.
    Off,
}

impl Effect {
    /// Every effect, in display order.
    pub const ALL: [Effect; 6] = [
        Effect::Static,
        Effect::Breathing,
        Effect::ColorCycle,
        Effect::Wave,
        Effect::Reactive,
        Effect::Off,
    ];

    /// The name used by the CLI and GUI, e.g. `color-cycle`.
    pub const fn name(self) -> &'static str {
        match self {
            Effect::Static => "static",
            Effect::Breathing => "breathing",
            Effect::ColorCycle => "color-cycle",
            Effect::Wave => "wave",
            Effect::Reactive => "reactive",
            Effect::Off => "off",
        }
    }

    /// Wire code. PLACEHOLDER.
    pub const fn code(self) -> u8 {
        match self {
            Effect::Static => 0x00,
            Effect::Breathing => 0x01,
            Effect::ColorCycle => 0x02,
            Effect::Wave => 0x03,
            Effect::Reactive => 0x04,
            Effect::Off => 0x05,
        }
    }

    /// Whether the effect uses the configured color.
    pub const fn uses_color(self) -> bool {
        matches!(self, Effect::Static | Effect::Breathing | Effect::Reactive)
    }

    /// Whether the effect is animated (uses speed).
    pub const fn uses_speed(self) -> bool {
        !matches!(self, Effect::Static | Effect::Off)
    }

    /// Whether the effect uses a direction.
    pub const fn uses_direction(self) -> bool {
        matches!(self, Effect::Wave)
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Effect {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_named(s, "effect", &Self::ALL, Self::name)
    }
}

/// Animation direction for directional effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Right to left.
    Left,
    /// Left to right.
    #[default]
    Right,
}

impl Direction {
    /// Every direction.
    pub const ALL: [Direction; 2] = [Direction::Left, Direction::Right];

    /// The name used by the CLI and GUI.
    pub const fn name(self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }

    /// Wire code. PLACEHOLDER.
    pub const fn code(self) -> u8 {
        match self {
            Direction::Left => 0x00,
            Direction::Right => 0x01,
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_named(s, "direction", &Self::ALL, Self::name)
    }
}

/// Brightness, from 0 (off) to [`Brightness::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    /// Highest brightness level.
    pub const MAX: u8 = 4;

    /// Creates a level, rejecting values above [`Self::MAX`].
    pub fn new(value: u8) -> Result<Self> {
        check_range("brightness", value, Self::MAX).map(Self)
    }

    /// Returns the raw level.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self(Self::MAX)
    }
}

/// Animation speed, from 0 (slow) to [`Speed::MAX`] (fast).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Speed(u8);

impl Speed {
    /// Fastest speed.
    pub const MAX: u8 = 2;

    /// Creates a speed, rejecting values above [`Self::MAX`].
    pub fn new(value: u8) -> Result<Self> {
        check_range("speed", value, Self::MAX).map(Self)
    }

    /// Returns the raw speed.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self(1)
    }
}

/// A complete lighting configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightingSettings {
    /// Effect to show.
    pub effect: Effect,
    /// Color, for effects where [`Effect::uses_color`] is true.
    pub color: Rgb,
    /// Overall brightness.
    pub brightness: Brightness,
    /// Animation speed, for animated effects.
    pub speed: Speed,
    /// Direction, for directional effects.
    pub direction: Direction,
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            effect: Effect::default(),
            color: Rgb::WHITE,
            brightness: Brightness::default(),
            speed: Speed::default(),
            direction: Direction::default(),
        }
    }
}

fn check_range(field: &'static str, value: u8, max: u8) -> Result<u8> {
    if value > max {
        Err(Error::OutOfRange { field, value, max })
    } else {
        Ok(value)
    }
}

fn parse_named<T: Copy>(
    input: &str,
    kind: &'static str,
    all: &[T],
    name: fn(T) -> &'static str,
) -> Result<T> {
    let wanted = input.trim().to_ascii_lowercase().replace('_', "-");
    for &item in all {
        if name(item) == wanted {
            return Ok(item);
        }
    }
    Err(Error::UnknownValue {
        kind,
        value: input.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_names_round_trip() {
        for effect in Effect::ALL {
            assert_eq!(effect.to_string().parse(), Ok(effect));
        }
    }

    #[test]
    fn effect_parsing_is_lenient_about_case_and_underscores() {
        assert_eq!("COLOR_CYCLE".parse(), Ok(Effect::ColorCycle));
        assert_eq!(" Wave ".parse(), Ok(Effect::Wave));
    }

    #[test]
    fn unknown_effect_is_rejected() {
        let err = "sparkle".parse::<Effect>().unwrap_err();
        let expected = Error::UnknownValue {
            kind: "effect",
            value: "sparkle".into(),
        };
        assert_eq!(err, expected);
    }

    #[test]
    fn effect_codes_are_unique() {
        let mut codes: Vec<u8> = Effect::ALL.iter().map(|e| e.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Effect::ALL.len());
    }

    #[test]
    fn effect_capabilities() {
        assert!(Effect::Static.uses_color());
        assert!(!Effect::Static.uses_speed());
        assert!(!Effect::Wave.uses_color());
        assert!(Effect::Wave.uses_speed());
        assert!(Effect::Wave.uses_direction());
        assert!(!Effect::Breathing.uses_direction());
        assert!(!Effect::Off.uses_color());
        assert!(!Effect::Off.uses_speed());
    }

    #[test]
    fn direction_round_trips_and_rejects_unknown() {
        for direction in Direction::ALL {
            assert_eq!(direction.to_string().parse(), Ok(direction));
        }
        assert_ne!(Direction::Left.code(), Direction::Right.code());
        assert!("up".parse::<Direction>().is_err());
    }

    #[test]
    fn brightness_accepts_range() {
        for value in 0..=Brightness::MAX {
            assert_eq!(Brightness::new(value).map(Brightness::value), Ok(value));
        }
        assert_eq!(Brightness::default().value(), Brightness::MAX);
    }

    #[test]
    fn brightness_rejects_above_max() {
        let expected = Error::OutOfRange {
            field: "brightness",
            value: Brightness::MAX + 1,
            max: Brightness::MAX,
        };
        assert_eq!(Brightness::new(Brightness::MAX + 1), Err(expected));
    }

    #[test]
    fn speed_accepts_range_and_rejects_above_max() {
        assert_eq!(Speed::new(Speed::MAX).map(Speed::value), Ok(Speed::MAX));
        assert_eq!(Speed::default().value(), 1);
        assert!(Speed::new(u8::MAX).is_err());
    }

    #[test]
    fn default_settings() {
        let settings = LightingSettings::default();
        assert_eq!(settings.effect, Effect::Static);
        assert_eq!(settings.color, Rgb::WHITE);
        assert_eq!(settings.direction, Direction::Right);
    }
}

//! Lighting effects and their settings.
//!
//! Wire codes match the `51 2C` set-effect command, verified on hardware (see
//! `docs/PROTOCOL.md#set-effect-fields`). The speed levels are this app's own
//! choice of raw speed bytes; Armoury Crate's exact values are unknown.

use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::color::Rgb;
use crate::error::{Error, Result};

/// The default 7-stop rainbow gradient (Armoury Crate's factory default).
pub const RAINBOW: [Rgb; 7] = [
    Rgb::RED,
    Rgb::ORANGE,
    Rgb::YELLOW,
    Rgb::GREEN,
    Rgb::CYAN,
    Rgb::BLUE,
    Rgb::PURPLE,
];

/// Number of colors Quicksand takes: one per keyboard row, top first.
pub const QUICKSAND_ROWS: usize = 6;

/// A built-in lighting effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Effect {
    /// One solid color.
    #[default]
    Static,
    /// Fades in and out.
    Breathing,
    /// The whole keyboard cycles through the spectrum.
    ColorCycle,
    /// Keys light up when pressed.
    Reactive,
    /// A gradient sweeps across the keyboard.
    Wave,
    /// Rings spread out from each pressed key.
    Ripple,
    /// Random keys twinkle.
    StarryNight,
    /// Rows fill up with color like sand.
    Quicksand,
    /// Light flows across the keys.
    Current,
    /// Drops run down each column.
    RainDrop,
}

impl Effect {
    /// Every effect, in wire-code order.
    pub const ALL: [Effect; 10] = [
        Effect::Static,
        Effect::Breathing,
        Effect::ColorCycle,
        Effect::Reactive,
        Effect::Wave,
        Effect::Ripple,
        Effect::StarryNight,
        Effect::Quicksand,
        Effect::Current,
        Effect::RainDrop,
    ];

    /// The name used by the CLI, e.g. `color-cycle`.
    pub const fn name(self) -> &'static str {
        match self {
            Effect::Static => "static",
            Effect::Breathing => "breathing",
            Effect::ColorCycle => "color-cycle",
            Effect::Reactive => "reactive",
            Effect::Wave => "wave",
            Effect::Ripple => "ripple",
            Effect::StarryNight => "starry-night",
            Effect::Quicksand => "quicksand",
            Effect::Current => "current",
            Effect::RainDrop => "rain-drop",
        }
    }

    /// Human-readable name, e.g. `Color cycle`.
    pub const fn label(self) -> &'static str {
        match self {
            Effect::Static => "Static",
            Effect::Breathing => "Breathing",
            Effect::ColorCycle => "Color cycle",
            Effect::Reactive => "Reactive",
            Effect::Wave => "Rainbow wave",
            Effect::Ripple => "Ripple",
            Effect::StarryNight => "Starry night",
            Effect::Quicksand => "Quicksand",
            Effect::Current => "Current",
            Effect::RainDrop => "Rain drop",
        }
    }

    /// One-line description for help text and tooltips.
    pub const fn description(self) -> &'static str {
        match self {
            Effect::Static => "One solid color on every key.",
            Effect::Breathing => "Keys slowly fade in and out.",
            Effect::ColorCycle => "The whole keyboard cycles through the spectrum.",
            Effect::Reactive => "Keys light up when you press them.",
            Effect::Wave => "A color gradient sweeps across the keyboard.",
            Effect::Ripple => "Rings spread out from each key you press.",
            Effect::StarryNight => "Random keys twinkle like stars.",
            Effect::Quicksand => "Rows fill up with color, one color per row.",
            Effect::Current => "Light flows across the keys.",
            Effect::RainDrop => "Drops of light run down each column.",
        }
    }

    /// Mode byte of the set-effect command.
    pub const fn code(self) -> u8 {
        match self {
            Effect::Static => 0,
            Effect::Breathing => 1,
            Effect::ColorCycle => 2,
            Effect::Reactive => 3,
            Effect::Wave => 4,
            Effect::Ripple => 5,
            Effect::StarryNight => 6,
            Effect::Quicksand => 7,
            Effect::Current => 8,
            Effect::RainDrop => 9,
        }
    }

    /// Looks up an effect by mode byte (e.g. from the device-info reply).
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.code() == code)
    }

    /// Color modes the effect supports. The first is the default. Empty means
    /// the effect picks its own colors.
    pub const fn color_modes(self) -> &'static [ColorMode] {
        use ColorMode::{Double, Gradient, Random, Single};
        match self {
            Effect::Static => &[Single],
            Effect::ColorCycle => &[],
            Effect::Wave => &[Gradient],
            Effect::Ripple | Effect::Quicksand => &[Gradient, Random],
            Effect::Breathing
            | Effect::Reactive
            | Effect::StarryNight
            | Effect::Current
            | Effect::RainDrop => &[Single, Double, Random],
        }
    }

    /// How many colors [`ColorMode::Gradient`] takes, if supported.
    pub fn gradient_len(self) -> Option<RangeInclusive<usize>> {
        match self {
            Effect::Wave | Effect::Ripple => Some(1..=7),
            Effect::Quicksand => Some(QUICKSAND_ROWS..=QUICKSAND_ROWS),
            _ => None,
        }
    }

    /// Whether the effect is animated (uses speed).
    pub const fn uses_speed(self) -> bool {
        !matches!(self, Effect::Static)
    }

    /// Directions the effect supports. The first is the default.
    pub const fn directions(self) -> &'static [Direction] {
        match self {
            Effect::Wave => &Direction::ALL,
            Effect::Quicksand => &[Direction::Up, Direction::Down],
            _ => &[],
        }
    }

    /// Whether the effect uses a direction.
    pub const fn uses_direction(self) -> bool {
        !self.directions().is_empty()
    }

    /// Whether the effect uses [`Thickness`] (shown as "density" for Starry night).
    pub const fn uses_thickness(self) -> bool {
        matches!(self, Effect::Wave | Effect::Ripple | Effect::StarryNight)
    }

    /// Whether the effect supports a background color for unlit keys.
    pub const fn uses_background(self) -> bool {
        matches!(
            self,
            Effect::StarryNight | Effect::Current | Effect::RainDrop
        )
    }

    /// Whether the effect reacts to key presses.
    pub const fn reacts_to_keys(self) -> bool {
        matches!(self, Effect::Reactive | Effect::Ripple)
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Effect {
    type Err = Error;

    /// Parses a name from [`Effect::name`] or a common alias such as
    /// `rainbow` or `raindrop`.
    fn from_str(s: &str) -> Result<Self> {
        let aliases = [
            ("rainbow", Effect::Wave),
            ("rainbow-wave", Effect::Wave),
            ("raindrop", Effect::RainDrop),
            ("starry", Effect::StarryNight),
            ("spectrum", Effect::ColorCycle),
            ("cycle", Effect::ColorCycle),
        ];
        let wanted = normalize(s);
        if let Some((_, effect)) = aliases.iter().find(|(name, _)| *name == wanted) {
            return Ok(*effect);
        }
        parse_named(s, "effect", &Self::ALL, Self::name)
    }
}

/// How an effect picks its colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// One chosen color.
    #[default]
    Single,
    /// Two chosen colors, alternating.
    Double,
    /// The keyboard picks random colors.
    Random,
    /// A list of colors: gradient stops (Wave, Ripple) or one per row (Quicksand).
    Gradient,
}

impl ColorMode {
    /// Every color mode.
    pub const ALL: [ColorMode; 4] = [
        ColorMode::Single,
        ColorMode::Double,
        ColorMode::Random,
        ColorMode::Gradient,
    ];

    /// The name used by the CLI.
    pub const fn name(self) -> &'static str {
        match self {
            ColorMode::Single => "single",
            ColorMode::Double => "double",
            ColorMode::Random => "random",
            ColorMode::Gradient => "gradient",
        }
    }

    /// Human-readable name.
    pub const fn label(self) -> &'static str {
        match self {
            ColorMode::Single => "One color",
            ColorMode::Double => "Two colors",
            ColorMode::Random => "Random",
            ColorMode::Gradient => "Custom colors",
        }
    }

    /// Color-mode byte of the set-effect command.
    pub const fn code(self) -> u8 {
        match self {
            ColorMode::Single | ColorMode::Gradient => 0x00,
            ColorMode::Random => 0x01,
            ColorMode::Double => 0x10,
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for ColorMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_named(s, "color mode", &Self::ALL, Self::name)
    }
}

/// Animation direction. Names describe where the light moves *to*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Left to right.
    #[default]
    Right,
    /// Top-left to bottom-right.
    DownRight,
    /// Top to bottom.
    Down,
    /// Top-right to bottom-left.
    DownLeft,
    /// Right to left.
    Left,
    /// Bottom-right to top-left.
    UpLeft,
    /// Bottom to top.
    Up,
    /// Bottom-left to top-right.
    UpRight,
    /// Outward from the centre (the J/K column).
    Out,
}

impl Direction {
    /// Every direction, in wire-code order.
    pub const ALL: [Direction; 9] = [
        Direction::Right,
        Direction::DownRight,
        Direction::Down,
        Direction::DownLeft,
        Direction::Left,
        Direction::UpLeft,
        Direction::Up,
        Direction::UpRight,
        Direction::Out,
    ];

    /// The name used by the CLI.
    pub const fn name(self) -> &'static str {
        match self {
            Direction::Right => "right",
            Direction::DownRight => "down-right",
            Direction::Down => "down",
            Direction::DownLeft => "down-left",
            Direction::Left => "left",
            Direction::UpLeft => "up-left",
            Direction::Up => "up",
            Direction::UpRight => "up-right",
            Direction::Out => "out",
        }
    }

    /// Human-readable description.
    pub const fn label(self) -> &'static str {
        match self {
            Direction::Right => "Left to right",
            Direction::DownRight => "Top-left to bottom-right",
            Direction::Down => "Top to bottom",
            Direction::DownLeft => "Top-right to bottom-left",
            Direction::Left => "Right to left",
            Direction::UpLeft => "Bottom-right to top-left",
            Direction::Up => "Bottom to top",
            Direction::UpRight => "Bottom-left to top-right",
            Direction::Out => "Outward from the centre",
        }
    }

    /// Direction byte of the set-effect command.
    pub const fn code(self) -> u8 {
        match self {
            Direction::Right => 0,
            Direction::DownRight => 1,
            Direction::Down => 2,
            Direction::DownLeft => 3,
            Direction::Left => 4,
            Direction::UpLeft => 5,
            Direction::Up => 6,
            Direction::UpRight => 7,
            Direction::Out => 8,
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

/// Band thickness (Wave, Ripple) or star density (Starry night).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Thickness {
    /// About 1–2 keys; fewer stars.
    Slim,
    /// Medium.
    #[default]
    Normal,
    /// About 4–5 keys; more stars.
    Thick,
}

impl Thickness {
    /// Every thickness.
    pub const ALL: [Thickness; 3] = [Thickness::Slim, Thickness::Normal, Thickness::Thick];

    /// Byte sent for effects that ignore thickness (OpenRGB's fixed value).
    pub const UNUSED_CODE: u8 = 0x02;

    /// The name used by the CLI.
    pub const fn name(self) -> &'static str {
        match self {
            Thickness::Slim => "slim",
            Thickness::Normal => "normal",
            Thickness::Thick => "thick",
        }
    }

    /// Size byte of the set-effect command.
    pub const fn code(self) -> u8 {
        match self {
            Thickness::Slim => 0,
            Thickness::Normal => 1,
            Thickness::Thick => 2,
        }
    }
}

impl fmt::Display for Thickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Thickness {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_named(s, "thickness", &Self::ALL, Self::name)
    }
}

/// Brightness level, from 0 (off) to [`Brightness::MAX`] (100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    /// Highest brightness level (100%).
    pub const MAX: u8 = 4;

    /// Creates a level, rejecting values above [`Self::MAX`].
    pub fn new(value: u8) -> Result<Self> {
        check_range("brightness", value, Self::MAX).map(Self)
    }

    /// Returns the raw level.
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Brightness as a percentage: 0, 25, 50, 75 or 100. Also the wire byte.
    pub const fn percent(self) -> u8 {
        self.0 * 25
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self(Self::MAX)
    }
}

/// Animation speed level, from 0 (slowest) to [`Speed::MAX`] (fastest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Speed(u8);

impl Speed {
    /// Fastest level.
    pub const MAX: u8 = 4;

    /// Raw speed byte per level. The keyboard treats lower bytes as faster.
    const BYTES: [u8; 5] = [0xFF, 0xA0, 0x60, 0x30, 0x10];
    const LABELS: [&'static str; 5] = ["Slowest", "Slow", "Normal", "Fast", "Fastest"];

    /// Creates a level, rejecting values above [`Self::MAX`].
    pub fn new(value: u8) -> Result<Self> {
        check_range("speed", value, Self::MAX).map(Self)
    }

    /// Returns the raw level.
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Speed byte of the set-effect command.
    pub const fn code(self) -> u8 {
        Self::BYTES[self.0 as usize]
    }

    /// Human-readable name, e.g. `Normal`.
    pub const fn label(self) -> &'static str {
        Self::LABELS[self.0 as usize]
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self(2)
    }
}

/// A complete lighting configuration for the set-effect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightingSettings {
    /// Effect to show.
    pub effect: Effect,
    /// How colors are chosen. Ignored by effects with no color modes.
    pub color_mode: ColorMode,
    /// Chosen colors. Single uses the first, Double the first two, Gradient
    /// all of them (Quicksand: the first six). Random ignores them.
    pub colors: Vec<Rgb>,
    /// Background for unlit keys, for effects where
    /// [`Effect::uses_background`] is true. `None` = dark.
    pub background: Option<Rgb>,
    /// Overall brightness.
    pub brightness: Brightness,
    /// Animation speed, for animated effects.
    pub speed: Speed,
    /// Direction, for effects where [`Effect::uses_direction`] is true.
    pub direction: Direction,
    /// Thickness or density, for effects where [`Effect::uses_thickness`] is true.
    pub thickness: Thickness,
}

impl LightingSettings {
    /// Sensible defaults for `effect`, at full brightness.
    pub fn for_effect(effect: Effect) -> Self {
        let color_mode = effect.color_modes().first().copied().unwrap_or_default();
        Self {
            effect,
            color_mode,
            colors: default_colors(effect, color_mode),
            background: None,
            brightness: Brightness::default(),
            speed: Speed::default(),
            direction: effect.directions().first().copied().unwrap_or_default(),
            thickness: Thickness::default(),
        }
    }

    /// Armoury Crate's factory default: rainbow wave at 50% brightness.
    pub fn factory_default() -> Self {
        Self {
            brightness: Brightness(2),
            ..Self::for_effect(Effect::Wave)
        }
    }

    /// Switches to `effect`, keeping every setting the new effect can use and
    /// replacing the rest with that effect's defaults. Always valid afterwards
    /// if `self` was.
    pub fn with_effect(&self, effect: Effect) -> Self {
        let modes = effect.color_modes();
        let color_mode = if modes.contains(&self.color_mode) || modes.is_empty() {
            self.color_mode
        } else {
            modes[0]
        };
        let entering_gradient =
            color_mode == ColorMode::Gradient && self.color_mode != ColorMode::Gradient;
        let mut colors = if entering_gradient {
            default_colors(effect, color_mode)
        } else {
            self.colors.clone()
        };
        match effect.gradient_len() {
            Some(range) if color_mode == ColorMode::Gradient => {
                fit_len(&mut colors, *range.start(), *range.end());
            }
            // Enough for Single or Double.
            _ => fit_len(&mut colors, 2, usize::MAX),
        }
        let directions = effect.directions();
        let direction = if directions.contains(&self.direction) || directions.is_empty() {
            self.direction
        } else {
            directions[0]
        };
        Self {
            effect,
            color_mode,
            colors,
            background: self.background.filter(|_| effect.uses_background()),
            direction,
            ..self.clone()
        }
    }

    /// The colors the current color mode sends. Call [`Self::validate`] first.
    pub fn active_colors(&self) -> &[Rgb] {
        let wanted = match self.color_mode {
            _ if self.effect.color_modes().is_empty() => 0,
            ColorMode::Random => 0,
            ColorMode::Single => 1,
            ColorMode::Double => 2,
            ColorMode::Gradient if self.effect == Effect::Quicksand => QUICKSAND_ROWS,
            ColorMode::Gradient => self.colors.len(),
        };
        &self.colors[..wanted.min(self.colors.len())]
    }

    /// Checks that the effect supports every chosen option.
    pub fn validate(&self) -> Result<()> {
        let effect = self.effect;
        let modes = effect.color_modes();
        let invalid = |msg: String| Err(Error::InvalidSettings(msg));
        if !modes.is_empty() && !modes.contains(&self.color_mode) {
            let names: Vec<&str> = modes.iter().map(|m| m.name()).collect();
            let mode = self.color_mode;
            return invalid(format!(
                "{effect} does not support {mode} colors (use: {})",
                names.join(", ")
            ));
        }
        let count = self.colors.len();
        if !modes.is_empty() {
            let needed = match self.color_mode {
                ColorMode::Single => Some(1..=usize::MAX),
                ColorMode::Double => Some(2..=usize::MAX),
                ColorMode::Random => None,
                ColorMode::Gradient => effect.gradient_len(),
            };
            if let Some(range) = needed {
                if !range.contains(&count) {
                    return invalid(color_count_message(effect, self.color_mode, &range, count));
                }
            }
        }
        if self.background.is_some() && !effect.uses_background() {
            return invalid(format!("{effect} does not support a background color"));
        }
        let directions = effect.directions();
        if !directions.is_empty() && !directions.contains(&self.direction) {
            let names: Vec<&str> = directions.iter().map(|d| d.name()).collect();
            let direction = self.direction;
            return invalid(format!(
                "{effect} does not support direction {direction} (use: {})",
                names.join(", ")
            ));
        }
        Ok(())
    }
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self::factory_default()
    }
}

fn default_colors(effect: Effect, mode: ColorMode) -> Vec<Rgb> {
    match (mode, effect) {
        (ColorMode::Gradient, Effect::Quicksand) => RAINBOW[..QUICKSAND_ROWS].to_vec(),
        (ColorMode::Gradient, _) => RAINBOW.to_vec(),
        _ => vec![Rgb::RED, Rgb::BLUE],
    }
}

/// Pads `colors` from [`RAINBOW`] up to `min` and truncates to `max`.
fn fit_len(colors: &mut Vec<Rgb>, min: usize, max: usize) {
    while colors.len() < min {
        colors.push(RAINBOW[colors.len() % RAINBOW.len()]);
    }
    colors.truncate(max);
}

fn color_count_message(
    effect: Effect,
    mode: ColorMode,
    range: &RangeInclusive<usize>,
    count: usize,
) -> String {
    let (min, max) = (*range.start(), *range.end());
    let needed = if min == max {
        format!("exactly {min}")
    } else if max == usize::MAX {
        format!("at least {min}")
    } else {
        format!("{min}-{max}")
    };
    format!("{effect} with {mode} colors needs {needed} color(s), got {count}")
}

fn check_range(field: &'static str, value: u8, max: u8) -> Result<u8> {
    if value > max {
        Err(Error::OutOfRange { field, value, max })
    } else {
        Ok(value)
    }
}

fn normalize(input: &str) -> String {
    input.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

fn parse_named<T: Copy>(
    input: &str,
    kind: &'static str,
    all: &[T],
    name: fn(T) -> &'static str,
) -> Result<T> {
    let wanted = normalize(input);
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

    fn invalid(settings: &LightingSettings) -> String {
        match settings.validate() {
            Err(Error::InvalidSettings(msg)) => msg,
            other => panic!("expected InvalidSettings, got {other:?}"),
        }
    }

    #[test]
    fn effect_names_round_trip() {
        for effect in Effect::ALL {
            assert_eq!(effect.to_string().parse(), Ok(effect));
            assert!(!effect.label().is_empty());
            assert!(effect.description().ends_with('.'));
        }
    }

    #[test]
    fn effect_parsing_accepts_case_underscores_and_aliases() {
        assert_eq!("COLOR_CYCLE".parse(), Ok(Effect::ColorCycle));
        assert_eq!(" Wave ".parse(), Ok(Effect::Wave));
        assert_eq!("rainbow".parse(), Ok(Effect::Wave));
        assert_eq!("Starry Night".parse(), Ok(Effect::StarryNight));
        assert_eq!("raindrop".parse(), Ok(Effect::RainDrop));
    }

    #[test]
    fn unknown_effect_is_rejected() {
        let err = "sparkle".parse::<Effect>().unwrap_err();
        let expected = Error::UnknownValue {
            kind: "effect",
            value: "sparkle".into(),
        };
        assert_eq!(err, expected);
        assert!("off".parse::<Effect>().is_err());
    }

    #[test]
    fn effect_codes_match_protocol_and_round_trip() {
        for (code, effect) in Effect::ALL.iter().enumerate() {
            assert_eq!(effect.code() as usize, code);
            assert_eq!(Effect::from_code(code as u8), Some(*effect));
        }
        assert_eq!(Effect::from_code(15), None);
    }

    #[test]
    fn effect_capabilities() {
        assert!(!Effect::Static.uses_speed());
        assert!(Effect::Wave.uses_speed());
        assert!(Effect::Wave.uses_direction());
        assert_eq!(Effect::Wave.directions().len(), 9);
        assert_eq!(
            Effect::Quicksand.directions(),
            [Direction::Up, Direction::Down]
        );
        assert!(!Effect::Breathing.uses_direction());
        assert!(Effect::StarryNight.uses_thickness());
        assert!(!Effect::Static.uses_thickness());
        assert!(Effect::RainDrop.uses_background());
        assert!(!Effect::Wave.uses_background());
        assert!(Effect::Ripple.reacts_to_keys());
        assert!(!Effect::Wave.reacts_to_keys());
        assert!(Effect::ColorCycle.color_modes().is_empty());
        assert_eq!(Effect::Static.color_modes(), [ColorMode::Single]);
        assert_eq!(Effect::Wave.gradient_len(), Some(1..=7));
        assert_eq!(Effect::Quicksand.gradient_len(), Some(6..=6));
        assert_eq!(Effect::Static.gradient_len(), None);
    }

    #[test]
    fn color_mode_names_and_codes() {
        for mode in ColorMode::ALL {
            assert_eq!(mode.to_string().parse(), Ok(mode));
            assert!(!mode.label().is_empty());
        }
        assert_eq!(ColorMode::Single.code(), 0x00);
        assert_eq!(ColorMode::Random.code(), 0x01);
        assert_eq!(ColorMode::Double.code(), 0x10);
        assert!("triple".parse::<ColorMode>().is_err());
    }

    #[test]
    fn direction_codes_match_protocol_and_round_trip() {
        for (code, direction) in Direction::ALL.iter().enumerate() {
            assert_eq!(direction.code() as usize, code);
            assert_eq!(direction.to_string().parse(), Ok(*direction));
            assert!(!direction.label().is_empty());
        }
        assert_eq!("UP_LEFT".parse(), Ok(Direction::UpLeft));
        assert!("sideways".parse::<Direction>().is_err());
    }

    #[test]
    fn thickness_codes_and_parsing() {
        for (code, thickness) in Thickness::ALL.iter().enumerate() {
            assert_eq!(thickness.code() as usize, code);
            assert_eq!(thickness.to_string().parse(), Ok(*thickness));
        }
        assert_eq!(Thickness::default(), Thickness::Normal);
        assert!("chunky".parse::<Thickness>().is_err());
    }

    #[test]
    fn brightness_accepts_range_and_maps_to_percent() {
        for value in 0..=Brightness::MAX {
            let level = Brightness::new(value).unwrap();
            assert_eq!(level.value(), value);
            assert_eq!(level.percent(), value * 25);
        }
        assert_eq!(Brightness::default().percent(), 100);
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
    fn speed_levels_get_faster() {
        let codes: Vec<u8> = (0..=Speed::MAX)
            .map(|v| Speed::new(v).unwrap().code())
            .collect();
        assert!(codes.windows(2).all(|w| w[0] > w[1]), "{codes:?}");
        assert_eq!(Speed::default().label(), "Normal");
        assert_eq!(Speed::new(0).unwrap().label(), "Slowest");
    }

    #[test]
    fn speed_rejects_above_max() {
        assert!(Speed::new(Speed::MAX + 1).is_err());
        assert!(Speed::new(u8::MAX).is_err());
    }

    #[test]
    fn factory_default_is_half_bright_rainbow_wave() {
        let settings = LightingSettings::default();
        assert_eq!(settings.effect, Effect::Wave);
        assert_eq!(settings.brightness.percent(), 50);
        assert_eq!(settings.colors, RAINBOW);
        assert_eq!(settings.thickness, Thickness::Normal);
        assert_eq!(settings.validate(), Ok(()));
    }

    #[test]
    fn for_effect_is_valid_for_every_effect() {
        for effect in Effect::ALL {
            let settings = LightingSettings::for_effect(effect);
            assert_eq!(settings.validate(), Ok(()), "{effect}");
        }
        let quicksand = LightingSettings::for_effect(Effect::Quicksand);
        assert_eq!(quicksand.colors.len(), QUICKSAND_ROWS);
        assert_eq!(quicksand.direction, Direction::Up);
    }

    #[test]
    fn with_effect_is_valid_between_every_pair() {
        for from in Effect::ALL {
            for mode in ColorMode::ALL {
                let mut start = LightingSettings::for_effect(from);
                if from.color_modes().contains(&mode) {
                    start.color_mode = mode;
                }
                for to in Effect::ALL {
                    let next = start.with_effect(to);
                    assert_eq!(next.effect, to);
                    assert_eq!(next.validate(), Ok(()), "{from} {mode} -> {to}");
                }
            }
        }
    }

    #[test]
    fn with_effect_keeps_shared_settings() {
        let mut breathing = LightingSettings::for_effect(Effect::Breathing);
        breathing.color_mode = ColorMode::Random;
        breathing.brightness = Brightness::new(1).unwrap();
        let reactive = breathing.with_effect(Effect::Reactive);
        assert_eq!(reactive.color_mode, ColorMode::Random);
        assert_eq!(reactive.brightness.value(), 1);

        let mut wave = LightingSettings::for_effect(Effect::Wave);
        wave.direction = Direction::Out;
        assert_eq!(wave.with_effect(Effect::Quicksand).direction, Direction::Up);
        assert_eq!(wave.with_effect(Effect::Ripple).colors, RAINBOW);
        assert_eq!(wave.with_effect(Effect::Quicksand).colors.len(), 6);
    }

    #[test]
    fn with_effect_drops_unsupported_background() {
        let mut starry = LightingSettings::for_effect(Effect::StarryNight);
        starry.background = Some(Rgb::BLUE);
        assert_eq!(
            starry.with_effect(Effect::Current).background,
            Some(Rgb::BLUE)
        );
        assert_eq!(starry.with_effect(Effect::Static).background, None);
    }

    #[test]
    fn active_colors_per_mode() {
        let mut settings = LightingSettings::for_effect(Effect::Breathing);
        settings.colors = RAINBOW.to_vec();
        assert_eq!(settings.active_colors(), &RAINBOW[..1]);
        settings.color_mode = ColorMode::Double;
        assert_eq!(settings.active_colors(), &RAINBOW[..2]);
        settings.color_mode = ColorMode::Random;
        assert!(settings.active_colors().is_empty());

        let wave = LightingSettings::for_effect(Effect::Wave);
        assert_eq!(wave.active_colors().len(), 7);
        let quicksand = LightingSettings {
            colors: RAINBOW.to_vec(),
            ..LightingSettings::for_effect(Effect::Quicksand)
        };
        assert_eq!(quicksand.active_colors().len(), 6);
        let cycle = LightingSettings::for_effect(Effect::ColorCycle);
        assert!(cycle.active_colors().is_empty());
    }

    #[test]
    fn validate_rejects_unsupported_color_mode() {
        let mut settings = LightingSettings::for_effect(Effect::Static);
        settings.color_mode = ColorMode::Random;
        assert!(invalid(&settings).contains("static does not support random"));
        settings.effect = Effect::Wave;
        assert!(invalid(&settings).contains("use: gradient"));
    }

    #[test]
    fn validate_checks_color_counts() {
        let mut settings = LightingSettings::for_effect(Effect::Static);
        settings.colors.clear();
        assert!(invalid(&settings).contains("at least 1"));

        let mut double = LightingSettings::for_effect(Effect::Breathing);
        double.color_mode = ColorMode::Double;
        double.colors = vec![Rgb::RED];
        assert!(invalid(&double).contains("at least 2"));

        let mut wave = LightingSettings::for_effect(Effect::Wave);
        wave.colors = vec![Rgb::RED; 8];
        assert!(invalid(&wave).contains("1-7"));
        wave.colors.clear();
        assert!(invalid(&wave).contains("got 0"));

        let mut quicksand = LightingSettings::for_effect(Effect::Quicksand);
        quicksand.colors.pop();
        assert!(invalid(&quicksand).contains("exactly 6"));

        let mut random = LightingSettings::for_effect(Effect::Ripple);
        random.color_mode = ColorMode::Random;
        random.colors.clear();
        assert_eq!(random.validate(), Ok(()));
    }

    #[test]
    fn validate_ignores_color_mode_for_color_cycle() {
        let mut cycle = LightingSettings::for_effect(Effect::ColorCycle);
        cycle.color_mode = ColorMode::Gradient;
        cycle.colors.clear();
        assert_eq!(cycle.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_unsupported_background_and_direction() {
        let mut settings = LightingSettings::for_effect(Effect::Static);
        settings.background = Some(Rgb::BLUE);
        assert!(invalid(&settings).contains("background"));

        let mut quicksand = LightingSettings::for_effect(Effect::Quicksand);
        quicksand.direction = Direction::Left;
        assert!(invalid(&quicksand).contains("use: up, down"));

        let mut rain = LightingSettings::for_effect(Effect::RainDrop);
        rain.background = Some(Rgb::BLUE);
        rain.direction = Direction::Left; // ignored by non-directional effects
        assert_eq!(rain.validate(), Ok(()));
    }
}

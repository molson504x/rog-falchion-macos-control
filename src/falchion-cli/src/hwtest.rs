//! Interactive hardware tests.
//!
//! Each case applies a setting, asks the person at the keyboard whether it
//! looks right, then restores the baseline. Nothing is saved (`50 55` is never
//! sent), so the keyboard's stored profile is untouched. The keyboard cannot
//! report its current colors, so the baseline is the active effect (read from
//! the keyboard) with default options, unless one is given. Run with
//! `make hw-test`; see `docs/DEVELOPMENT.md`.

use std::io::{BufRead, Write};

use anyhow::{bail, Context, Result};
use clap::Parser;
use falchion_core::{
    Brightness, ColorMode, DeviceProvider, Direction, Effect, Keyboard, LightingSettings, Rgb,
    Speed, Thickness,
};

use crate::{Cli, Command};

/// How a test talks to the person at the keyboard.
pub trait Tester {
    /// Shows a message.
    fn say(&mut self, message: &str);
    /// Asks a yes/no question. `false` also means "no answer".
    fn ask(&mut self, question: &str) -> bool;
}

/// A [`Tester`] over text streams (stdin/stderr in the hardware test).
#[derive(Debug)]
pub struct StdioTester<R, W> {
    input: R,
    output: W,
}

impl<R: BufRead, W: Write> StdioTester<R, W> {
    /// Reads answers from `input` and writes prompts to `output`.
    pub fn new(input: R, output: W) -> Self {
        Self { input, output }
    }
}

impl<R: BufRead, W: Write> Tester for StdioTester<R, W> {
    fn say(&mut self, message: &str) {
        let _ = writeln!(self.output, "{message}");
    }

    fn ask(&mut self, question: &str) -> bool {
        loop {
            let _ = write!(self.output, "{question} [y/n] ");
            let _ = self.output.flush();
            let mut line = String::new();
            match self.input.read_line(&mut line) {
                Ok(0) | Err(_) => return false,
                Ok(_) => {}
            }
            match line.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" => return true,
                "n" | "no" => return false,
                _ => self.say("Please answer y or n."),
            }
        }
    }
}

/// One manual check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Short identifier, used by the case filter.
    pub name: &'static str,
    /// What the person should see.
    pub expect: &'static str,
    /// Settings to apply.
    pub settings: LightingSettings,
}

/// Result of one case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Case name.
    pub name: String,
    /// Whether the person confirmed it.
    pub passed: bool,
    /// Why it failed, if it did.
    pub note: Option<String>,
}

fn case(name: &'static str, expect: &'static str, settings: LightingSettings) -> Case {
    Case {
        name,
        expect,
        settings,
    }
}

fn level<T>(new: fn(u8) -> falchion_core::Result<T>, value: u8) -> T {
    new(value).expect("level in range")
}

/// Every lighting case, covering each effect and option.
pub fn lighting_cases() -> Vec<Case> {
    use ColorMode::{Double, Random};
    let base = LightingSettings::for_effect;
    let with = |effect, f: &dyn Fn(&mut LightingSettings)| {
        let mut settings = base(effect);
        f(&mut settings);
        settings
    };
    vec![
        case(
            "static-red",
            "Every key solid red at full brightness.",
            with(Effect::Static, &|s| s.colors = vec![Rgb::RED]),
        ),
        case(
            "brightness-25",
            "Every key dim white (25% brightness).",
            with(Effect::Static, &|s| {
                s.colors = vec![Rgb::WHITE];
                s.brightness = level(Brightness::new, 1);
            }),
        ),
        case(
            "brightness-off",
            "All key lights off.",
            with(Effect::Static, &|s| {
                s.brightness = level(Brightness::new, 0)
            }),
        ),
        case(
            "breathing-single",
            "All keys fade in and out in green.",
            with(Effect::Breathing, &|s| s.colors = vec![Rgb::GREEN]),
        ),
        case(
            "breathing-double",
            "Keys fade in and out, alternating red and blue.",
            with(Effect::Breathing, &|s| s.color_mode = Double),
        ),
        case(
            "breathing-random",
            "Keys fade in and out, in a different random color each time.",
            with(Effect::Breathing, &|s| s.color_mode = Random),
        ),
        case(
            "color-cycle",
            "The whole keyboard cycles smoothly through the rainbow.",
            base(Effect::ColorCycle),
        ),
        case(
            "reactive",
            "Keys are dark; press some: each one lights up cyan, then fades.",
            with(Effect::Reactive, &|s| s.colors = vec![Rgb::CYAN]),
        ),
        case(
            "wave-right",
            "A rainbow sweeps from left to right at a moderate speed.",
            base(Effect::Wave),
        ),
        case(
            "wave-up-slim-fast",
            "Thin rainbow bands move quickly from the bottom row to the top row.",
            with(Effect::Wave, &|s| {
                s.direction = Direction::Up;
                s.thickness = Thickness::Slim;
                s.speed = level(Speed::new, Speed::MAX);
            }),
        ),
        case(
            "wave-out-thick-slow",
            "Wide rainbow bands move slowly outward from the middle (J/K) column.",
            with(Effect::Wave, &|s| {
                s.direction = Direction::Out;
                s.thickness = Thickness::Thick;
                s.speed = level(Speed::new, 0);
            }),
        ),
        case(
            "wave-two-colors-diagonal",
            "A red/blue gradient moves diagonally from the top-left to the bottom-right.",
            with(Effect::Wave, &|s| {
                s.colors = vec![Rgb::RED, Rgb::BLUE];
                s.direction = Direction::DownRight;
            }),
        ),
        case(
            "ripple",
            "Keys are dark; press some: rainbow rings spread out from each key.",
            base(Effect::Ripple),
        ),
        case(
            "ripple-random",
            "Press some keys: rings in random colors spread out from each key.",
            with(Effect::Ripple, &|s| s.color_mode = Random),
        ),
        case(
            "starry-night",
            "White stars twinkle on random keys over a blue background.",
            with(Effect::StarryNight, &|s| {
                s.colors = vec![Rgb::WHITE];
                s.background = Some(Rgb::BLUE);
            }),
        ),
        case(
            "starry-night-sparse",
            "Fewer stars than before, in random colors, on dark keys.",
            with(Effect::StarryNight, &|s| {
                s.color_mode = Random;
                s.thickness = Thickness::Slim;
            }),
        ),
        case(
            "quicksand",
            "Rows fill up from the bottom; top row red, then orange, yellow, green, cyan.",
            base(Effect::Quicksand),
        ),
        case(
            "quicksand-down-random",
            "Rows fill from the top down, in random colors.",
            with(Effect::Quicksand, &|s| {
                s.color_mode = Random;
                s.direction = Direction::Down;
            }),
        ),
        case(
            "current",
            "Orange light flows across the keys.",
            with(Effect::Current, &|s| s.colors = vec![Rgb::ORANGE]),
        ),
        case(
            "rain-drop",
            "Green drops run down each column over a dark blue background.",
            with(Effect::RainDrop, &|s| {
                s.colors = vec![Rgb::GREEN];
                s.background = Some(Rgb::new(0, 0, 0x60));
            }),
        ),
    ]
}

/// Keeps cases whose name contains any comma-separated term of `filter`.
pub fn filter_cases(cases: Vec<Case>, filter: Option<&str>) -> Vec<Case> {
    let Some(filter) = filter.map(str::trim).filter(|f| !f.is_empty()) else {
        return cases;
    };
    let terms: Vec<&str> = filter.split(',').map(str::trim).collect();
    cases
        .into_iter()
        .filter(|c| terms.iter().any(|t| !t.is_empty() && c.name.contains(t)))
        .collect()
}

/// Parses a baseline written as `falchion lighting` options, e.g.
/// `--effect static --color red --brightness 2`.
pub fn parse_baseline(spec: &str) -> Result<LightingSettings> {
    let argv = ["falchion", "lighting"]
        .into_iter()
        .chain(spec.split_whitespace());
    let cli = Cli::try_parse_from(argv).context("invalid baseline")?;
    match cli.command {
        Command::Lighting(args) => {
            if args.save {
                bail!("the baseline must not use --save");
            }
            Ok(args.to_settings()?)
        }
        _ => unreachable!("always parsed as lighting"),
    }
}

/// Summarizes settings for a person, e.g. `wave, 7 colors, 50%, Normal speed`.
pub fn describe(settings: &LightingSettings) -> String {
    let effect = settings.effect;
    let mut parts = vec![effect.label().to_string()];
    if !effect.color_modes().is_empty() {
        let colors: Vec<String> = settings
            .active_colors()
            .iter()
            .map(Rgb::to_string)
            .collect();
        parts.push(match settings.color_mode {
            ColorMode::Random => "random colors".into(),
            _ => colors.join(" "),
        });
    }
    if let Some(background) = settings.background {
        parts.push(format!("background {background}"));
    }
    parts.push(format!("{}% brightness", settings.brightness.percent()));
    if effect.uses_speed() {
        parts.push(format!("{} speed", settings.speed.label()));
    }
    if effect.uses_direction() {
        parts.push(settings.direction.label().to_lowercase());
    }
    if effect.uses_thickness() {
        parts.push(format!("{} thickness", settings.thickness));
    }
    parts.join(", ")
}

/// Runs `cases` on the first keyboard, restoring `baseline` after each one.
/// Without a baseline, the keyboard's active effect with default options is
/// used. Ends by asking whether the keyboard looks as it did at the start.
pub fn run_cases<P: DeviceProvider, T: Tester>(
    provider: &P,
    tester: &mut T,
    cases: &[Case],
    baseline: Option<LightingSettings>,
) -> Result<Vec<Outcome>> {
    if cases.is_empty() {
        bail!("no test cases selected");
    }
    let (device, transport) = provider.open_first()?;
    let mut keyboard = Keyboard::new(transport);
    let status = keyboard.status().context("reading the active effect")?;
    let baseline = baseline.unwrap_or_else(|| {
        let effect = status.effect().unwrap_or(Effect::Wave);
        LightingSettings::factory_default().with_effect(effect)
    });
    tester.say(&format!("Testing {device}"));
    tester.say(&format!("Baseline to restore: {}", describe(&baseline)));
    tester.say("Nothing is saved to the keyboard during these tests.");

    let mut outcomes = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let total = cases.len();
        tester.say(&format!("\n[{}/{total}] {}", index + 1, case.name));
        let outcome = match keyboard.apply_lighting(&case.settings) {
            Ok(()) => {
                let question = format!("Expected: {}\nIs that what you see?", case.expect);
                let passed = tester.ask(&question);
                let note = (!passed).then(|| "the person said it looked wrong".to_string());
                Outcome {
                    name: case.name.to_string(),
                    passed,
                    note,
                }
            }
            Err(err) => Outcome {
                name: case.name.to_string(),
                passed: false,
                note: Some(format!("could not apply: {err}")),
            },
        };
        outcomes.push(outcome);
        keyboard
            .apply_lighting(&baseline)
            .context("restoring the baseline")?;
    }

    let restored = tester.ask(&format!(
        "\nRestored: {}\nDoes the keyboard look as it did before the tests?",
        describe(&baseline)
    ));
    outcomes.push(Outcome {
        name: "restore-baseline".into(),
        passed: restored,
        note: (!restored).then(|| {
            "set FALCHION_BASELINE to your usual lighting, or power-cycle the keyboard to \
             reload its saved profile"
                .to_string()
        }),
    });
    Ok(outcomes)
}

/// One line per outcome plus a total.
pub fn summary(outcomes: &[Outcome]) -> String {
    let mut lines: Vec<String> = outcomes
        .iter()
        .map(|o| {
            let mark = if o.passed { "PASS" } else { "FAIL" };
            match &o.note {
                Some(note) => format!("{mark}  {}: {note}", o.name),
                None => format!("{mark}  {}", o.name),
            }
        })
        .collect();
    let passed = outcomes.iter().filter(|o| o.passed).count();
    lines.push(format!("{passed}/{} passed", outcomes.len()));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use falchion_core::protocol::{Command as Wire, Report, REJECTED};
    use falchion_core::{Error, MockProvider, REPORT_LEN};
    use std::collections::VecDeque;

    /// Scripted answers; records everything said.
    #[derive(Default)]
    struct Script {
        answers: VecDeque<bool>,
        said: Vec<String>,
    }

    impl Script {
        fn new(answers: &[bool]) -> Self {
            Self {
                answers: answers.iter().copied().collect(),
                said: Vec::new(),
            }
        }
    }

    impl Tester for Script {
        fn say(&mut self, message: &str) {
            self.said.push(message.to_string());
        }
        fn ask(&mut self, question: &str) -> bool {
            self.said.push(question.to_string());
            self.answers.pop_front().unwrap_or(false)
        }
    }

    fn provider_with_effect(code: u8) -> MockProvider {
        let provider = MockProvider::with_device();
        let reply = Report::new(Wire::DeviceInfo, &[0, 0, 1, 2, 3, 0, 6, code, 1]).unwrap();
        provider.transport.queue_response(&reply);
        provider
    }

    fn sent_effects(provider: &MockProvider) -> Vec<u8> {
        let written = provider.transport.written().into_iter();
        written
            .filter(|raw| raw[..2] == Wire::SetEffect.bytes())
            .map(|raw| raw[2])
            .collect()
    }

    fn two_cases() -> Vec<Case> {
        filter_cases(lighting_cases(), Some("static-red,color-cycle"))
    }

    #[test]
    fn every_case_is_valid_and_uniquely_named() {
        let cases = lighting_cases();
        let mut names: Vec<&str> = cases.iter().map(|c| c.name).collect();
        for case in &cases {
            assert_eq!(case.settings.validate(), Ok(()), "{}", case.name);
        }
        for effect in Effect::ALL {
            assert!(
                cases.iter().any(|c| c.settings.effect == effect),
                "{effect}"
            );
        }
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), cases.len());
    }

    #[test]
    fn filter_selects_by_substring() {
        assert_eq!(
            filter_cases(lighting_cases(), None).len(),
            lighting_cases().len()
        );
        assert_eq!(
            filter_cases(lighting_cases(), Some(" ")).len(),
            lighting_cases().len()
        );
        let waves = filter_cases(lighting_cases(), Some("wave"));
        assert!(waves.len() >= 3 && waves.iter().all(|c| c.name.contains("wave")));
        assert_eq!(two_cases().len(), 2);
        assert!(filter_cases(lighting_cases(), Some("nothing,")).is_empty());
    }

    #[test]
    fn passing_run_applies_each_case_then_restores_baseline() {
        let provider = provider_with_effect(Effect::Breathing.code());
        let mut tester = Script::new(&[true, true, true]);
        let outcomes = run_cases(&provider, &mut tester, &two_cases(), None).unwrap();
        assert!(outcomes.iter().all(|o| o.passed), "{outcomes:?}");
        assert_eq!(outcomes.len(), 3);
        let (static_, cycle, breathing) = (0, 2, Effect::Breathing.code());
        assert_eq!(
            sent_effects(&provider),
            [static_, breathing, cycle, breathing]
        );
        assert!(tester
            .said
            .iter()
            .any(|s| s.contains("Baseline to restore: Breathing")));
        assert!(!provider
            .transport
            .written()
            .iter()
            .any(|raw| raw[..2] == Wire::Save.bytes()));
    }

    #[test]
    fn rejected_case_fails_and_still_restores() {
        let provider = provider_with_effect(15);
        let mut tester = Script::new(&[false, true, true]);
        let outcomes = run_cases(&provider, &mut tester, &two_cases(), None).unwrap();
        assert!(!outcomes[0].passed);
        assert!(outcomes[0]
            .note
            .as_deref()
            .unwrap()
            .contains("looked wrong"));
        assert!(outcomes[1].passed && outcomes[2].passed);
        // Unknown active effect (15 = direct) falls back to wave.
        assert_eq!(sent_effects(&provider)[1], Effect::Wave.code());
    }

    #[test]
    fn apply_error_is_a_failure_and_unrestored_baseline_is_reported() {
        // Device info, then reject the case, then acknowledge the restore.
        let provider = silent_provider(&[info_raw(), rejection(), ack(Wire::SetEffect)]);
        let baseline = parse_baseline("-e static -c blue").unwrap();
        let case = filter_cases(lighting_cases(), Some("static-red"));
        let mut tester = Script::new(&[false]);
        let outcomes = run_cases(&provider, &mut tester, &case, Some(baseline)).unwrap();
        let note = |i: usize| outcomes[i].note.clone().unwrap_or_default();
        assert!(note(0).contains("could not apply"));
        assert!(!outcomes[1].passed);
        assert!(note(1).contains("FALCHION_BASELINE"));
    }

    /// A non-echoing provider that replays exactly `replies`.
    fn silent_provider(replies: &[[u8; REPORT_LEN]]) -> MockProvider {
        let provider = MockProvider {
            transport: falchion_core::MockTransport::new(),
            ..MockProvider::with_device()
        };
        for reply in replies {
            provider.transport.queue_raw(*reply);
        }
        provider
    }

    fn info_raw() -> [u8; REPORT_LEN] {
        let info = Report::new(Wire::DeviceInfo, &[0, 0, 1, 2, 3, 0, 6, 0, 1]);
        info.unwrap().encode()
    }

    fn ack(command: Wire) -> [u8; REPORT_LEN] {
        Report::new(command, &[]).unwrap().encode()
    }

    fn rejection() -> [u8; REPORT_LEN] {
        let mut raw = [0u8; REPORT_LEN];
        raw[..2].copy_from_slice(&REJECTED);
        raw
    }

    #[test]
    fn restore_failure_aborts() {
        // The case is acknowledged; the restore gets no reply and times out.
        let provider = silent_provider(&[info_raw(), ack(Wire::SetEffect)]);
        let mut tester = Script::new(&[true]);
        let err = run_cases(&provider, &mut tester, &two_cases(), None).unwrap_err();
        assert!(format!("{err:#}").contains("restoring the baseline"));
    }

    #[test]
    fn run_fails_without_device_cases_or_status() {
        let mut tester = Script::default();
        let none = MockProvider::default();
        assert!(run_cases(&none, &mut tester, &two_cases(), None).is_err());
        let provider = MockProvider::with_device();
        assert!(run_cases(&provider, &mut tester, &[], None).is_err());
        provider.transport.fail_writes_with(Error::Timeout);
        let err = run_cases(&provider, &mut tester, &two_cases(), None).unwrap_err();
        assert!(format!("{err:#}").contains("active effect"));
    }

    #[test]
    fn parse_baseline_accepts_lighting_options() {
        let baseline = parse_baseline("--effect starry-night -c white -b blue -t slim").unwrap();
        assert_eq!(baseline.effect, Effect::StarryNight);
        assert_eq!(baseline.background, Some(Rgb::BLUE));
        assert_eq!(parse_baseline("").unwrap().effect, Effect::Static);
    }

    #[test]
    fn parse_baseline_rejects_bad_options_and_save() {
        assert!(parse_baseline("--effect sparkle").is_err());
        assert!(parse_baseline("--effect static --save").is_err());
        assert!(parse_baseline("-e static -m random").is_err());
    }

    #[test]
    fn describe_lists_relevant_options() {
        let wave = describe(&LightingSettings::factory_default());
        assert!(wave.starts_with("Rainbow wave, #ff0000"), "{wave}");
        assert!(wave.contains("50% brightness"));
        assert!(wave.contains("Normal speed"));
        assert!(wave.contains("left to right"));
        assert!(wave.contains("normal thickness"));
        let mut rain = LightingSettings::for_effect(Effect::RainDrop);
        rain.color_mode = ColorMode::Random;
        rain.background = Some(Rgb::BLUE);
        let rain = describe(&rain);
        assert!(rain.contains("random colors") && rain.contains("background #0000ff"));
        let stat = describe(&LightingSettings::for_effect(Effect::Static));
        assert!(!stat.contains("speed"), "{stat}");
        let cycle = describe(&LightingSettings::for_effect(Effect::ColorCycle));
        assert_eq!(cycle, "Color cycle, 100% brightness, Normal speed");
    }

    #[test]
    fn summary_counts_passes() {
        let outcomes = [
            Outcome {
                name: "a".into(),
                passed: true,
                note: None,
            },
            Outcome {
                name: "b".into(),
                passed: false,
                note: Some("why".into()),
            },
        ];
        assert_eq!(summary(&outcomes), "PASS  a\nFAIL  b: why\n1/2 passed");
    }

    #[test]
    fn stdio_tester_reads_yes_no_and_reprompts() {
        let input = b"maybe\nYES\nn\n" as &[u8];
        let mut output = Vec::new();
        let mut tester = StdioTester::new(input, &mut output);
        assert!(tester.ask("ok?"));
        assert!(!tester.ask("ok?"));
        assert!(!tester.ask("ok?"), "end of input counts as no");
        tester.say("bye");
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Please answer y or n."));
        assert!(text.ends_with("bye\n"));
    }
}

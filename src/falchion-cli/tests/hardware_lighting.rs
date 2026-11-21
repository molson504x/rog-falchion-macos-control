//! Interactive hardware test for lighting effects. Needs a connected ROG
//! Falchion and a person watching it, so it is `#[ignore]`d by `cargo test`.
//!
//! Run: `make hw-test` (optionally `FALCHION_CASES=wave,ripple` and
//! `FALCHION_BASELINE="--effect static --color red"`). Each case changes the
//! lighting, asks for confirmation, then restores the baseline. Nothing is
//! saved to the keyboard.

use falchion_cli::hwtest::StdioTester;
use falchion_cli::hwtest::{filter_cases, lighting_cases, parse_baseline, run_cases, summary};
use falchion_hid::HidProvider;

#[test]
#[ignore = "needs a ROG Falchion and a person watching it; run `make hw-test`"]
fn lighting_effects_on_hardware() {
    let baseline = std::env::var("FALCHION_BASELINE")
        .ok()
        .map(|spec| parse_baseline(&spec).expect("valid FALCHION_BASELINE"));
    let filter = std::env::var("FALCHION_CASES").ok();
    let cases = filter_cases(lighting_cases(), filter.as_deref());
    let stdin = std::io::stdin();
    let mut tester = StdioTester::new(stdin.lock(), std::io::stderr());
    let outcomes = run_cases(&HidProvider, &mut tester, &cases, baseline).expect("test run");
    let report = summary(&outcomes);
    eprintln!("\n{report}");
    assert!(outcomes.iter().all(|o| o.passed), "{report}");
}

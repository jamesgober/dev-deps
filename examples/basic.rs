//! Run both checks (unused + outdated) against the current crate and
//! print the resulting `Report` as JSON.
//!
//! ```text
//! cargo install cargo-udeps cargo-outdated
//! rustup toolchain install nightly      # cargo-udeps needs nightly
//! cargo run --example basic
//! ```
//!
//! If either tool is missing the example prints a clear error and
//! exits 0 so `cargo build --examples` still succeeds without them.

use dev_deps::{DepCheck, DepError, DepScope};

fn main() {
    let check = DepCheck::new("example", "0.1.0").scope(DepScope::All);
    let result = match check.execute() {
        Ok(r) => r,
        Err(DepError::UdepsToolNotInstalled) => {
            eprintln!("cargo-udeps (or nightly toolchain) is not installed; skipping.");
            eprintln!("Install: `cargo install cargo-udeps` + `rustup toolchain install nightly`");
            return;
        }
        Err(DepError::OutdatedToolNotInstalled) => {
            eprintln!("cargo-outdated is not installed; skipping.");
            eprintln!("Install: `cargo install cargo-outdated`");
            return;
        }
        Err(e) => {
            eprintln!("dependency check failed: {e}");
            return;
        }
    };
    println!(
        "{} unused, {} outdated, {} total findings",
        result.unused_count(),
        result.outdated_count(),
        result.total_findings()
    );
    let report = result.into_report();
    println!("{}", report.to_json().expect("serialize report"));
}

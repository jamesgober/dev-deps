//! Minimal example: check dependency health and emit a report.
//!
//! Run with: `cargo run --example basic`

use dev_deps::{DepCheck, DepScope};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let check = DepCheck::new("example", "0.1.0").scope(DepScope::All);
    let result = check.execute()?;
    println!("Total findings: {}", result.total_findings());
    let report = result.into_report();
    println!("{}", report.to_json()?);
    Ok(())
}

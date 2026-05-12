//! Run only the unused-dependency scanner (`cargo udeps`) and print
//! each finding.
//!
//! ```text
//! cargo install cargo-udeps
//! rustup toolchain install nightly
//! cargo run --example unused_only
//! ```

use dev_deps::{DepCheck, DepError, DepScope};

fn main() {
    let result = match DepCheck::new("example", "0.1.0")
        .scope(DepScope::Unused)
        .execute()
    {
        Ok(r) => r,
        Err(DepError::UdepsToolNotInstalled) => {
            eprintln!("cargo-udeps (or nightly) not installed; install and retry.");
            return;
        }
        Err(e) => {
            eprintln!("unused-deps check failed: {e}");
            return;
        }
    };
    println!("{} unused-dependency findings", result.unused_count());
    for u in &result.unused {
        println!("  {} (in {})", u.crate_name, u.kind.as_str());
    }
}

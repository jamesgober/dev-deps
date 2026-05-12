//! Run only the outdated-version scanner (`cargo outdated`) and print
//! each finding plus its major-version lag.
//!
//! ```text
//! cargo install cargo-outdated
//! cargo run --example outdated_only
//! ```

use dev_deps::{DepCheck, DepError, DepScope};

fn main() {
    let result = match DepCheck::new("example", "0.1.0")
        .scope(DepScope::Outdated)
        .execute()
    {
        Ok(r) => r,
        Err(DepError::OutdatedToolNotInstalled) => {
            eprintln!("cargo-outdated is not installed; install and retry.");
            return;
        }
        Err(e) => {
            eprintln!("outdated-deps check failed: {e}");
            return;
        }
    };
    println!("{} outdated-dependency findings", result.outdated_count());
    for o in &result.outdated {
        println!(
            "  {} {} -> {} ({} major behind)",
            o.crate_name, o.current, o.latest, o.major_behind
        );
    }
}

//! Demonstrate `DepProducer` — the `dev_report::Producer` adapter.
//!
//! Construction is always safe and cheap. The actual subprocess
//! invocation is gated behind the `DEV_DEPS_EXAMPLE_RUN` env var so
//! CI doesn't pay for a full dependency check on every example build.
//!
//! ```text
//! cargo run --example producer
//! DEV_DEPS_EXAMPLE_RUN=1 cargo run --example producer
//! ```

use dev_deps::{DepCheck, DepProducer, DepScope};
use dev_report::Producer;

fn main() {
    let producer = DepProducer::new(DepCheck::new("my-crate", "0.1.0").scope(DepScope::All));
    println!("Constructed DepProducer for 'my-crate' v0.1.0.");

    if std::env::var("DEV_DEPS_EXAMPLE_RUN").is_ok() {
        let report = producer.produce();
        println!("{}", report.to_json().expect("serialize report"));
    } else {
        println!("Set DEV_DEPS_EXAMPLE_RUN=1 to spawn `cargo udeps` + `cargo outdated`");
        println!("in the current directory and print the resulting JSON report.");
    }
}

//! `cargo udeps` subprocess invocation + JSON parsing.
//!
//! `cargo-udeps` requires the nightly toolchain; we invoke it via
//! `cargo +nightly udeps --output json`. The JSON shape is:
//!
//! ```text
//! { "success": false,
//!   "unused_deps": {
//!     "<package-id>": {
//!       "manifest_path": "...",
//!       "normal": ["foo"],
//!       "development": ["bar"],
//!       "build": []
//!     }
//!   } }
//! ```

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::{DepError, DepKind, UnusedDep};

pub(crate) fn run(workdir: Option<&Path>, workspace: bool) -> Result<Vec<UnusedDep>, DepError> {
    detect()?;
    let mut cmd = Command::new("cargo");
    cmd.args(["+nightly", "udeps", "--output", "json"]);
    if workspace {
        cmd.arg("--workspace");
    }
    if let Some(d) = workdir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .map_err(|e| DepError::SubprocessFailed(e.to_string()))?;
    // cargo-udeps exits non-zero when it finds unused deps — that's the
    // success path. Empty stdout + non-zero is the real failure signal.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(DepError::SubprocessFailed(stderr));
    }
    parse(&stdout)
}

fn detect() -> Result<(), DepError> {
    let probe = Command::new("cargo")
        .args(["+nightly", "udeps", "--version"])
        .output();
    match probe {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(DepError::UdepsToolNotInstalled),
    }
}

// ---------------------------------------------------------------------------
// JSON shape
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct UdepsReport {
    #[serde(default)]
    unused_deps: serde_json::Value,
}

#[derive(Deserialize, Default)]
struct UnusedKinds {
    #[serde(default)]
    normal: Vec<String>,
    #[serde(default)]
    development: Vec<String>,
    #[serde(default)]
    build: Vec<String>,
}

pub(crate) fn parse(json: &str) -> Result<Vec<UnusedDep>, DepError> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    let report: UdepsReport = serde_json::from_str(json).map_err(|e| {
        DepError::ParseError(format!("{e}; first 200 chars: {:?}", first_200(json)))
    })?;
    let mut findings = Vec::new();
    if let serde_json::Value::Object(map) = &report.unused_deps {
        for (_pkg, value) in map.iter() {
            let kinds: UnusedKinds = match serde_json::from_value(value.clone()) {
                Ok(k) => k,
                Err(_) => continue,
            };
            for name in kinds.normal {
                findings.push(UnusedDep {
                    crate_name: name,
                    kind: DepKind::Normal,
                });
            }
            for name in kinds.development {
                findings.push(UnusedDep {
                    crate_name: name,
                    kind: DepKind::Development,
                });
            }
            for name in kinds.build {
                findings.push(UnusedDep {
                    crate_name: name,
                    kind: DepKind::Build,
                });
            }
        }
    }
    Ok(findings)
}

fn first_200(s: &str) -> &str {
    if s.len() <= 200 {
        s
    } else {
        &s[..200]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_findings() {
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn parses_single_package_unused_deps() {
        let json = r#"{
            "success": false,
            "unused_deps": {
                "my-crate 0.1.0 (path+file:///some/dir)": {
                    "manifest_path": "/some/dir/Cargo.toml",
                    "normal": ["foo", "bar"],
                    "development": ["baz"],
                    "build": []
                }
            }
        }"#;
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 3);
        let names: Vec<&str> = findings.iter().map(|u| u.crate_name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
        assert!(names.contains(&"baz"));
    }

    #[test]
    fn assigns_kinds_correctly() {
        let json = r#"{
            "unused_deps": {
                "p": {
                    "normal": ["n"],
                    "development": ["d"],
                    "build": ["b"]
                }
            }
        }"#;
        let findings = parse(json).unwrap();
        let kinds: Vec<DepKind> = findings.iter().map(|u| u.kind).collect();
        assert!(kinds.contains(&DepKind::Normal));
        assert!(kinds.contains(&DepKind::Development));
        assert!(kinds.contains(&DepKind::Build));
    }

    #[test]
    fn handles_multiple_packages() {
        let json = r#"{
            "unused_deps": {
                "a 0.1.0": { "normal": ["x"] },
                "b 0.2.0": { "normal": ["y"] }
            }
        }"#;
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn missing_unused_deps_is_empty() {
        let findings = parse(r#"{ "success": true }"#).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn rejects_garbage_input() {
        let err = parse("not json").err().unwrap();
        assert!(matches!(err, DepError::ParseError(_)));
    }

    #[test]
    fn ignores_unparseable_package_entries() {
        let json = r#"{
            "unused_deps": {
                "good": { "normal": ["x"] },
                "bad":  "not an object"
            }
        }"#;
        let findings = parse(json).unwrap();
        // The "good" entry produces a finding; "bad" is skipped.
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].crate_name, "x");
    }
}

//! `cargo outdated` subprocess invocation + JSON parsing.
//!
//! `cargo outdated --format json` emits one JSON object per workspace
//! member, separated by blank lines (concatenated JSON, not NDJSON).
//! Each looks like:
//!
//! ```text
//! { "crate_name": "<root>",
//!   "dependencies": [
//!     { "name": "serde", "project": "1.0.0", "compat": "1.0.5",
//!       "latest": "2.0.0", "kind": "Normal", "platform": null } ] }
//! ```

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::{DepError, DepKind, OutdatedDep};

pub(crate) fn run(
    workdir: Option<&Path>,
    workspace: bool,
    excludes: &[String],
) -> Result<Vec<OutdatedDep>, DepError> {
    detect()?;
    let mut cmd = Command::new("cargo");
    cmd.args(["outdated", "--format", "json"]);
    if workspace {
        cmd.args(["--workspace", "--root-deps-only"]);
    }
    for ex in excludes {
        cmd.args(["--exclude", ex]);
    }
    if let Some(d) = workdir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .map_err(|e| DepError::SubprocessFailed(e.to_string()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(DepError::SubprocessFailed(stderr));
    }
    parse(&stdout)
}

fn detect() -> Result<(), DepError> {
    let probe = Command::new("cargo")
        .args(["outdated", "--version"])
        .output();
    match probe {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(DepError::OutdatedToolNotInstalled),
    }
}

// ---------------------------------------------------------------------------
// JSON shape
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct OutdatedReport {
    #[serde(default)]
    dependencies: Vec<OutdatedEntry>,
}

#[derive(Deserialize)]
struct OutdatedEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    project: String,
    #[serde(default)]
    latest: String,
    #[serde(default)]
    kind: Option<String>,
}

pub(crate) fn parse(json: &str) -> Result<Vec<OutdatedDep>, DepError> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // `cargo outdated --format json` concatenates JSON objects when there
    // are multiple workspace members. We parse them via the streaming
    // `Deserializer` so the multi-object case works the same as the
    // single-object case.
    let mut findings = Vec::new();
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<OutdatedReport>();
    let mut saw_any = false;
    for report in stream {
        let report = report.map_err(|e| {
            DepError::ParseError(format!("{e}; first 200 chars: {:?}", first_200(trimmed)))
        })?;
        saw_any = true;
        for dep in report.dependencies {
            if dep.project.is_empty() || dep.latest.is_empty() {
                continue;
            }
            // Skip when project == latest (some configs include current deps).
            if dep.project == dep.latest {
                continue;
            }
            // Skip the "---" sentinel rows some versions emit.
            if dep.name.is_empty() || dep.name == "---" {
                continue;
            }
            findings.push(OutdatedDep {
                major_behind: major_diff(&dep.project, &dep.latest),
                crate_name: dep.name,
                current: dep.project,
                latest: dep.latest,
                kind: dep_kind_from_label(dep.kind.as_deref()),
            });
        }
    }
    if !saw_any && !trimmed.is_empty() {
        return Err(DepError::ParseError(format!(
            "no JSON objects found in `cargo outdated` output; first 200 chars: {:?}",
            first_200(trimmed)
        )));
    }
    Ok(findings)
}

fn dep_kind_from_label(label: Option<&str>) -> Option<DepKind> {
    match label.map(str::to_ascii_lowercase).as_deref() {
        Some("normal") => Some(DepKind::Normal),
        Some("development") | Some("dev") => Some(DepKind::Development),
        Some("build") => Some(DepKind::Build),
        _ => None,
    }
}

/// Difference in the major version component, saturating at 0 when the
/// strings can't be parsed.
fn major_diff(current: &str, latest: &str) -> u32 {
    let c = major(current).unwrap_or(0);
    let l = major(latest).unwrap_or(0);
    l.saturating_sub(c)
}

fn major(s: &str) -> Option<u32> {
    s.trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .next()
        .and_then(|m| m.parse::<u32>().ok())
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
    fn parses_a_single_workspace_member() {
        let json = r#"{
            "crate_name": "root",
            "dependencies": [
                { "name": "serde", "project": "1.0.0", "compat": "1.0.5", "latest": "2.0.0", "kind": "Normal" },
                { "name": "tokio", "project": "1.2.0", "latest": "1.30.0", "kind": "Normal" }
            ]
        }"#;
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 2);
        let serde_finding = findings.iter().find(|f| f.crate_name == "serde").unwrap();
        assert_eq!(serde_finding.current, "1.0.0");
        assert_eq!(serde_finding.latest, "2.0.0");
        assert_eq!(serde_finding.major_behind, 1);
        let tokio_finding = findings.iter().find(|f| f.crate_name == "tokio").unwrap();
        assert_eq!(tokio_finding.major_behind, 0); // same major, different minor
    }

    #[test]
    fn parses_multiple_workspace_members_concatenated() {
        // cargo-outdated emits one object per member without separators.
        let json = concat!(
            r#"{"crate_name":"a","dependencies":[{"name":"serde","project":"1.0.0","latest":"2.0.0","kind":"Normal"}]}"#,
            r#"{"crate_name":"b","dependencies":[{"name":"tokio","project":"1.0.0","latest":"1.5.0","kind":"Normal"}]}"#,
        );
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn skips_entries_with_no_update() {
        let json = r#"{
            "dependencies": [
                { "name": "serde", "project": "1.0.5", "latest": "1.0.5", "kind": "Normal" }
            ]
        }"#;
        assert!(parse(json).unwrap().is_empty());
    }

    #[test]
    fn skips_sentinel_dashes() {
        let json = r#"{
            "dependencies": [
                { "name": "---", "project": "---", "latest": "---", "kind": "Normal" }
            ]
        }"#;
        assert!(parse(json).unwrap().is_empty());
    }

    #[test]
    fn major_diff_handles_multi_digit_majors() {
        assert_eq!(major_diff("1.0.0", "10.0.0"), 9);
        assert_eq!(major_diff("0.9.2", "0.9.5"), 0);
        assert_eq!(major_diff("3.0.0", "1.0.0"), 0); // saturating
    }

    #[test]
    fn major_strips_leading_caret_or_tilde() {
        assert_eq!(major("^1.2.3"), Some(1));
        assert_eq!(major("~2.0"), Some(2));
        assert_eq!(major("not-a-version"), None);
    }

    #[test]
    fn dep_kind_from_label_handles_known_strings() {
        assert_eq!(dep_kind_from_label(Some("Normal")), Some(DepKind::Normal));
        assert_eq!(
            dep_kind_from_label(Some("development")),
            Some(DepKind::Development)
        );
        assert_eq!(dep_kind_from_label(Some("dev")), Some(DepKind::Development));
        assert_eq!(dep_kind_from_label(Some("build")), Some(DepKind::Build));
        assert_eq!(dep_kind_from_label(Some("???")), None);
        assert_eq!(dep_kind_from_label(None), None);
    }

    #[test]
    fn rejects_garbage_input() {
        let err = parse("not json").err().unwrap();
        assert!(matches!(err, DepError::ParseError(_)));
    }
}

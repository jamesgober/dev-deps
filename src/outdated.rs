//! `cargo outdated` subprocess invocation + JSON parsing.
//!
//! `cargo outdated --format json` emits one JSON object per workspace
//! member, one per line (concatenated JSON, not NDJSON). Each looks like:
//!
//! ```text
//! { "crate_name": "<root>",
//!   "dependencies": [
//!     { "name": "serde", "project": "1.0.0", "compat": "---",
//!       "latest": "2.0.0", "kind": "Normal", "platform": null } ] }
//! ```
//!
//! We always pass `--root-deps-only`. Without it the tool also lists
//! transitive dependencies as `parent->child` rows, most of them with
//! `"latest": "Removed"`; those rows are skipped if they show up anyway.

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
    // `--root-deps-only`: report the dependencies declared in
    // Cargo.toml, not every transitive crate.
    cmd.args(["outdated", "--format", "json", "--root-deps-only"]);
    if workspace {
        cmd.arg("--workspace");
    }
    for ex in excludes {
        cmd.args(["--exclude", ex]);
    }
    if let Some(d) = workdir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .map_err(|e| DepError::SubprocessFailed(format!("could not spawn cargo outdated: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(DepError::SubprocessFailed(format!(
            "cargo outdated exited with {}: {}",
            output.status,
            stderr.trim()
        )));
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
            // `project` / `latest` hold "---" (no data) or "Removed" (the
            // dependency is gone in the latest version of its parent)
            // instead of a version for some rows. Only compare versions.
            if parse_version(&dep.project).is_none() || parse_version(&dep.latest).is_none() {
                continue;
            }
            // Skip when project == latest (some configs include current deps).
            if dep.project == dep.latest {
                continue;
            }
            // Skip sentinel names and transitive `parent->child` rows.
            if dep.name.is_empty() || dep.name == "---" || dep.name.contains("->") {
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

/// Number of semver-incompatible ("major") releases between `current`
/// and `latest`, using Cargo's compatibility rule: the left-most non-zero
/// component is the breaking one. So `1.4.0 -> 3.0.0` is 2 behind,
/// `0.7.3 -> 0.10.3` is 3 behind, `0.0.3 -> 0.0.5` is 2 behind, and
/// `0.9.2 -> 0.9.5` is 0 behind. Moving from `0.x` to `N.y` counts as
/// `N`. Saturates at 0 when `latest` is not newer or either string
/// cannot be parsed.
fn major_diff(current: &str, latest: &str) -> u32 {
    let (Some(c), Some(l)) = (parse_version(current), parse_version(latest)) else {
        return 0;
    };
    if c.0 != l.0 || c.0 > 0 {
        return l.0.saturating_sub(c.0);
    }
    if c.1 != l.1 || c.1 > 0 {
        return l.1.saturating_sub(c.1);
    }
    l.2.saturating_sub(c.2)
}

/// `(major, minor, patch)` from a version string such as `1.2.3`,
/// `^1.2`, `~2`, or `0.9.0+wasi-snapshot-preview1`. Missing minor /
/// patch components count as 0. `None` when there is no leading major
/// number (`"---"`, `"Removed"`, ...).
fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['^', '~', '=', 'v', ' ']);
    let mut parts = s.split('.');
    let num = |p: Option<&str>| -> Option<u32> {
        let p = p?;
        let end = p.find(|c: char| !c.is_ascii_digit()).unwrap_or(p.len());
        p[..end].parse::<u32>().ok()
    };
    let major = num(parts.next())?;
    Some((
        major,
        num(parts.next()).unwrap_or(0),
        num(parts.next()).unwrap_or(0),
    ))
}

/// At most the first 200 bytes of `s`, cut on a char boundary.
fn first_200(s: &str) -> &str {
    if s.len() <= 200 {
        return s;
    }
    let mut end = 200;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
        assert_eq!(major_diff("1.4.0", "1.9.0"), 0);
    }

    #[test]
    fn major_diff_follows_cargo_rules_for_zero_major() {
        // 0.x: the minor component is the breaking one.
        assert_eq!(major_diff("0.7.3", "0.10.3"), 3);
        assert_eq!(major_diff("0.6.14", "0.7.0"), 1);
        // 0.0.x: every patch is breaking.
        assert_eq!(major_diff("0.0.3", "0.0.5"), 2);
        // 0.x -> N.y counts the N major releases.
        assert_eq!(major_diff("0.6.14", "1.16.2"), 1);
        assert_eq!(major_diff("0.9.0", "3.1.0"), 3);
        // Downgrades and garbage saturate at 0.
        assert_eq!(major_diff("0.10.0", "0.9.0"), 0);
        assert_eq!(major_diff("Removed", "1.0.0"), 0);
        assert_eq!(major_diff("1.0.0", "---"), 0);
    }

    #[test]
    fn parse_version_accepts_requirements_and_build_metadata() {
        assert_eq!(parse_version("^1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("~2.0"), Some((2, 0, 0)));
        assert_eq!(parse_version("=3"), Some((3, 0, 0)));
        assert_eq!(
            parse_version("0.9.0+wasi-snapshot-preview1"),
            Some((0, 9, 0))
        );
        assert_eq!(parse_version("1.0.0-beta.2"), Some((1, 0, 0)));
        assert_eq!(parse_version("not-a-version"), None);
        assert_eq!(parse_version("Removed"), None);
        assert_eq!(parse_version("---"), None);
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("99999999999.0.0"), None);
    }

    /// Real `cargo outdated --format json --workspace --root-deps-only`
    /// output (cargo-outdated 0.19.0) for a two-member workspace.
    const REAL_WORKSPACE: &str = concat!(
        r#"{"crate_name":"app","dependencies":[{"name":"bitflags","project":"1.3.2","compat":"---","latest":"2.13.2","kind":"Normal","platform":null},{"name":"rand","project":"0.7.3","compat":"---","latest":"0.10.3","kind":"Normal","platform":null}]}"#,
        "\n",
        r#"{"crate_name":"lib2","dependencies":[{"name":"bitflags","project":"1.3.2","compat":"---","latest":"2.13.2","kind":"Normal","platform":null},{"name":"smallvec","project":"0.6.14","compat":"---","latest":"1.16.2","kind":"Normal","platform":null}]}"#,
        "\n",
    );

    #[test]
    fn parses_real_workspace_output() {
        let findings = parse(REAL_WORKSPACE).unwrap();
        assert_eq!(findings.len(), 4);
        let rand = findings.iter().find(|f| f.crate_name == "rand").unwrap();
        assert_eq!(rand.current, "0.7.3");
        assert_eq!(rand.latest, "0.10.3");
        assert_eq!(rand.major_behind, 3);
        assert_eq!(rand.kind, Some(DepKind::Normal));
        let smallvec = findings
            .iter()
            .find(|f| f.crate_name == "smallvec")
            .unwrap();
        assert_eq!(smallvec.major_behind, 1);
    }

    /// Real output without `--root-deps-only`: transitive rows use
    /// `parent->child` names and `"latest": "Removed"`.
    #[test]
    fn skips_transitive_and_removed_rows() {
        let json = r#"{"crate_name":"app","dependencies":[{"name":"bitflags","project":"1.3.2","compat":"---","latest":"2.13.2","kind":"Normal","platform":null},{"name":"getrandom->cfg-if","project":"1.0.5","compat":"---","latest":"Removed","kind":"Normal","platform":null},{"name":"getrandom->wasi","project":"0.9.0+wasi-snapshot-preview1","compat":"---","latest":"Removed","kind":"Normal","platform":"cfg(target_os = \"wasi\")"},{"name":"rand->getrandom","project":"0.1.16","compat":"---","latest":"0.4.3","kind":"Normal","platform":null},{"name":"rand->rand_hc","project":"0.2.0","compat":"---","latest":"Removed","kind":"Development","platform":null}]}"#;
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].crate_name, "bitflags");
        assert_eq!(findings[0].major_behind, 1);
    }

    #[test]
    fn crlf_separated_objects_parse() {
        let json = concat!(
            r#"{"crate_name":"a","dependencies":[{"name":"x","project":"1.0.0","latest":"2.0.0","kind":"Normal"}]}"#,
            "\r\n",
            r#"{"crate_name":"b","dependencies":[]}"#,
            "\r\n",
        );
        let findings = parse(json).unwrap();
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn parse_error_preview_does_not_split_multibyte_chars() {
        let mut s = "x".repeat(199);
        s.push_str("ééé");
        assert_eq!(first_200(&s).len(), 199);
        assert!(matches!(parse(&s), Err(DepError::ParseError(_))));
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

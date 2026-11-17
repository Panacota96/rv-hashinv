//! Link to an RV Triage collection (gap G29 of rv-triage's admissibility notes).
//!
//! An input folder that holds a `manifest.json` of format `rakvor.triage-manifest/1` is an RV Triage package. Its
//! collection's audit log sits in the standard layout, `<OUT>/logs/<RUN-ID>/audit.jsonl` for a package at
//! `<OUT>/reports/<RUN-ID>/package`. The intake verifies that log with the same code as `verify-log` (chain,
//! timestamp, re-hash of every packaged file it records), records the collection's run id and final audit head
//! in its own audit log (`collection_linked`), and compares every file it registers with the digest the
//! collection recorded for it on the machine where it was collected.
//!
//! ```text
//! package/manifest.json ──► ../../../logs/<RUN-ID>/audit.jsonl ──► verify_log ──► collection_linked
//!            every registered item under the package ──► digest recorded by the collection? ──► finding or none
//! ```

use crate::analyzer::{Finding, Severity};
use crate::hash::HashAlgo;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MANIFEST_FORMAT: &str = "rakvor.triage-manifest/1";
pub const COLLECTION_DIGEST_MISMATCH: &str = "COLLECTION_DIGEST_MISMATCH";
pub const COLLECTION_LOG_UNVERIFIED: &str = "COLLECTION_LOG_UNVERIFIED";

/// What the collection's audit log says, and whether it can be trusted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogState {
    /// Chain intact and every check of `verify-log` passed.
    Intact,
    /// Chain intact (the recorded digests are authentic) but `verify-log` reported problems, e.g. a packaged file
    /// edited or deleted after the collection, or a timestamp that does not verify.
    Problems(Vec<String>),
    /// Chain broken: nothing the log records can be trusted.
    Broken(Vec<String>),
    /// No collection log next to the package (copied without its `logs/` folder).
    Missing(String),
}

impl LogState {
    pub fn key(&self) -> &'static str {
        match self {
            LogState::Intact => "intact",
            LogState::Problems(_) => "problems",
            LogState::Broken(_) => "chain_broken",
            LogState::Missing(_) => "missing",
        }
    }
    /// True when the digests the log records can be used (its chain is intact).
    pub fn digests_usable(&self) -> bool {
        matches!(self, LogState::Intact | LogState::Problems(_))
    }
    fn reason(&self) -> String {
        match self {
            LogState::Intact => String::new(),
            LogState::Problems(p) | LogState::Broken(p) => {
                p.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
            }
            LogState::Missing(m) => m.clone(),
        }
    }
}

/// One package and its collection.
#[derive(Clone, Debug)]
pub struct PackageLink {
    /// The package folder, normalised like item paths (`case::normalize`).
    pub package: PathBuf,
    pub manifest_sha256: String,
    pub manifest: Value,
    pub log: PathBuf,
    pub state: LogState,
    /// Final audit entry of the collection (seq, entry hash), when the log is readable.
    pub head: Option<(u64, String)>,
    pub timestamps: Vec<String>,
    /// Collection tool, version and run id: from the log's `run_started` entry, else from the manifest (rv-triage
    /// 0.2.0 manifests do not carry them).
    pub tool: Option<String>,
    pub version: Option<String>,
    pub run_id: Option<String>,
    /// Normalised path of every packaged file → the digests the collection recorded (`output_written`).
    pub recorded: BTreeMap<String, BTreeMap<String, String>>,
}

/// Input folders that are RV Triage packages (a `manifest.json` of [`MANIFEST_FORMAT`] directly inside).
pub fn detect(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = inputs
        .iter()
        .filter(|p| p.is_dir())
        .filter(|p| {
            std::fs::read(p.join("manifest.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .is_some_and(|m| m["format"] == MANIFEST_FORMAT)
        })
        .map(|p| crate::case::normalize(p))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// `<OUT>/logs/<RUN-ID>/audit.jsonl` for a package at `<OUT>/reports/<RUN-ID>/package`.
pub fn collection_log(package: &Path) -> Option<PathBuf> {
    let run = package.parent()?;
    let reports = run.parent()?;
    if reports.file_name()? != "reports" {
        return None;
    }
    Some(
        reports
            .parent()?
            .join("logs")
            .join(run.file_name()?)
            .join("audit.jsonl"),
    )
}

fn read_entries(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Verifies the collection of `package` and reads what it recorded.
pub fn link(package: &Path) -> PackageLink {
    let manifest_bytes = std::fs::read(package.join("manifest.json")).unwrap_or_default();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap_or(Value::Null);
    let manifest_sha256 = crate::hash::hash_bytes(&manifest_bytes, HashAlgo::Sha256);
    let log = collection_log(package).unwrap_or_else(|| package.join("..").join("audit.jsonl"));
    let mut l = PackageLink {
        package: package.to_path_buf(),
        manifest_sha256,
        manifest,
        log: log.clone(),
        state: LogState::Missing(String::new()),
        head: None,
        timestamps: vec![],
        tool: None,
        version: None,
        run_id: None,
        recorded: BTreeMap::new(),
    };
    let text = |v: &Value| v.as_str().map(String::from);
    l.tool = text(&l.manifest["tool"]);
    l.version = text(&l.manifest["version"]);
    l.run_id = text(&l.manifest["run_id"]);
    if !log.is_file() {
        l.state = LogState::Missing(format!(
            "no collection audit log at {} (the package was copied without the collection's logs folder)",
            log.display()
        ));
        return l;
    }
    match crate::verify::verify_log(&log) {
        Err(e) => {
            l.state = LogState::Missing(e);
            return l;
        }
        Ok(v) => {
            l.timestamps = v.timestamps.clone();
            let chain_broken = v.entries == 0 || v.outputs.is_none();
            l.state = if chain_broken {
                LogState::Broken(if v.problems.is_empty() {
                    vec!["the collection audit chain does not verify".into()]
                } else {
                    v.problems.clone()
                })
            } else if v.problems.is_empty() {
                LogState::Intact
            } else {
                LogState::Problems(v.problems.clone())
            };
        }
    }
    l.head = crate::audit::heads(&log).into_iter().next_back();
    if l.state.digests_usable()
        && let Some(first) = read_entries(&log).into_iter().next()
        && first["action"] == "run_started"
    {
        l.tool = text(&first["details"]["tool"]).or(l.tool);
        l.version = text(&first["details"]["version"]).or(l.version);
        l.run_id = text(&first["run_id"]).or(l.run_id);
    }
    if l.state.digests_usable() {
        let run_dir = package.parent().unwrap_or(package);
        for e in read_entries(&log) {
            if e["action"] != "output_written" {
                continue;
            }
            let Some(file) = e["details"]["file"].as_str() else {
                continue;
            };
            let Some(rel) = file.strip_prefix("package/") else {
                continue;
            };
            let mut p = run_dir.join("package");
            for part in rel.split('/') {
                p.push(part);
            }
            let digests: BTreeMap<String, String> = e["details"]["hashes"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| {
                            v.as_str().map(|h| (k.clone(), h.to_ascii_lowercase()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            l.recorded.insert(crate::case::normalize_path(&p), digests);
        }
    }
    l
}

/// Per-package tally of [`check_item`] results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub matched: u64,
    pub mismatched: u64,
    pub not_in_collection: u64,
    pub unverified: u64,
}

impl Tally {
    pub fn add(&mut self, c: ItemCheck) {
        match c {
            ItemCheck::Matched => self.matched += 1,
            ItemCheck::Mismatched => self.mismatched += 1,
            ItemCheck::NotInCollection => self.not_in_collection += 1,
            ItemCheck::Unverified => self.unverified += 1,
        }
    }
}

/// The `collection_linked` audit details.
pub fn details(l: &PackageLink, t: Tally) -> Value {
    json!({
        "package": l.package.display().to_string(),
        "manifest_sha256": l.manifest_sha256,
        "collection": {
            "tool": l.tool, "version": l.version, "run_id": l.run_id,
            "log": l.log.display().to_string(),
            "audit_entries": l.head.as_ref().map(|h| h.0),
            "audit_head": l.head.as_ref().map(|h| h.1.clone()),
            "timestamps": l.timestamps,
        },
        "verification": l.state.key(),
        "problems": match &l.state {
            LogState::Problems(p) | LogState::Broken(p) => p.clone(),
            LogState::Missing(m) => vec![m.clone()],
            LogState::Intact => vec![],
        },
        "items": {"matched": t.matched, "mismatched": t.mismatched, "not_in_collection": t.not_in_collection,
                  "unverified": t.unverified},
    })
}

/// How one registered item relates to its collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemCheck {
    Matched,
    Mismatched,
    NotInCollection,
    Unverified,
}

/// Compares an item under `l.package` with the digests its collection recorded. Returns the check and, unless it
/// matched, the finding to add to the item.
pub fn check_item(
    l: &PackageLink,
    path: &Path,
    hashes: &[(HashAlgo, String)],
) -> (ItemCheck, Option<Finding>) {
    if !l.state.digests_usable() {
        return (
            ItemCheck::Unverified,
            Some(
                Finding::new(COLLECTION_LOG_UNVERIFIED, Severity::Medium)
                    .with("state", l.state.key())
                    .with("reason", l.state.reason())
                    .with("log", l.log.display().to_string()),
            ),
        );
    }
    let Some(rec) = l.recorded.get(&crate::case::normalize_path(path)) else {
        return (ItemCheck::NotInCollection, None);
    };
    for (alg, now) in hashes {
        if let Some(then) = rec.get(alg.name())
            && !then.eq_ignore_ascii_case(now)
        {
            return (
                ItemCheck::Mismatched,
                Some(
                    Finding::new(COLLECTION_DIGEST_MISMATCH, Severity::High)
                        .with("algorithm", alg.name())
                        .with("collected", then.clone())
                        .with("registered", now.clone())
                        .with("run_id", l.run_id.clone().unwrap_or_else(|| "?".into())),
                ),
            );
        }
    }
    if hashes.iter().any(|(a, _)| rec.contains_key(a.name())) {
        (ItemCheck::Matched, None)
    } else {
        // No algorithm in common: nothing to compare.
        (ItemCheck::NotInCollection, None)
    }
}

/// The package (if any) an item belongs to.
pub fn package_of<'a>(links: &'a [PackageLink], path: &Path) -> Option<&'a PackageLink> {
    let p = crate::case::normalize(path);
    links.iter().find(|l| p.starts_with(&l.package))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunConfig, run};
    use std::sync::atomic::AtomicBool;

    /// `<out>/reports/<RUN>/package` with two files and a manifest, and the collection's audit log at
    /// `<out>/logs/<RUN>/audit.jsonl` recording their digests, as RV Triage writes them.
    fn package(out: &Path) -> PathBuf {
        let run_id = "20261117T100000Z-c0ffee";
        let pkg = out.join("reports").join(run_id).join("package");
        let files = [
            (
                "artifacts/hives/Windows/System32/config/SYSTEM",
                &b"regf-synthetic"[..],
            ),
            (
                "artifacts/prefetch/Windows/Prefetch/A.pf",
                b"MAM\x04synthetic",
            ),
        ];
        let manifest = json!({"format": MANIFEST_FORMAT, "collected": []});
        let mut log =
            crate::audit::AuditLog::create(&out.join("logs").join(run_id), run_id, "tester")
                .unwrap();
        log.record(
            "run_started",
            None,
            json!({"tool": "rv-triage", "version": "0.2.0"}),
        )
        .unwrap();
        let mut all: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|(p, b)| (p.to_string(), b.to_vec()))
            .collect();
        all.push((
            "manifest.json".into(),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        ));
        for (rel, bytes) in &all {
            let p = rel.split('/').fold(pkg.clone(), |p, part| p.join(part));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, bytes).unwrap();
            let sha = crate::hash::hash_bytes(bytes, HashAlgo::Sha256);
            log.record(
                "output_written",
                Some(rel),
                json!({"file": format!("package/{rel}"), "hashes": {"sha256": sha}}),
            )
            .unwrap();
        }
        log.record("run_finished", None, json!({})).unwrap();
        pkg
    }

    fn intake(input: &Path, out: &Path) -> (crate::RunSummary, Vec<Value>) {
        let cfg = RunConfig {
            inputs: vec![input.to_path_buf()],
            recursive: true,
            out_dir: out.to_path_buf(),
            langs: vec!["en".into()],
            ..Default::default()
        };
        let s = run(&cfg, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        let entries = read_entries(&s.log_dir.join("audit.jsonl"));
        (s, entries)
    }

    fn linked(entries: &[Value]) -> Vec<&Value> {
        entries
            .iter()
            .filter(|e| e["action"] == "collection_linked")
            .map(|e| &e["details"])
            .collect()
    }

    fn codes(s: &crate::RunSummary) -> Vec<String> {
        let mut c: Vec<String> = Vec::new();
        for tj in std::fs::read_dir(s.report_dir.join("items")).unwrap() {
            let j: Value = serde_json::from_str(
                &std::fs::read_to_string(tj.unwrap().path().join("technical.json")).unwrap(),
            )
            .unwrap();
            c.extend(
                j["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| f["code"].as_str().unwrap().to_string()),
            );
        }
        c
    }

    #[test]
    fn an_intact_package_is_linked_to_its_collection_head() {
        let t = tempfile::tempdir().unwrap();
        let pkg = package(&t.path().join("usb"));
        let (s, e) = intake(&pkg, &t.path().join("out"));
        let l = linked(&e);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["verification"], "intact", "{}", l[0]);
        assert_eq!(l[0]["items"]["matched"], 3);
        assert_eq!(l[0]["collection"]["run_id"], "20261117T100000Z-c0ffee");
        assert_eq!(l[0]["collection"]["tool"], "rv-triage");
        let col = t
            .path()
            .join("usb/logs/20261117T100000Z-c0ffee/audit.jsonl");
        let (seq, head) = crate::audit::heads(&col).into_iter().next_back().unwrap();
        assert_eq!(l[0]["collection"]["audit_entries"], seq);
        assert_eq!(l[0]["collection"]["audit_head"], head);
        assert!(!codes(&s).iter().any(|c| c.starts_with("COLLECTION_")));
        assert_eq!(s.high, 0);
    }

    #[test]
    fn a_file_edited_after_the_collection_is_a_high_finding() {
        let t = tempfile::tempdir().unwrap();
        let pkg = package(&t.path().join("usb"));
        std::fs::write(
            pkg.join("artifacts/prefetch/Windows/Prefetch/A.pf"),
            b"edited",
        )
        .unwrap();
        let (s, e) = intake(&pkg, &t.path().join("out"));
        let l = linked(&e);
        assert_eq!(l[0]["verification"], "problems", "{}", l[0]);
        assert_eq!(l[0]["items"]["mismatched"], 1);
        assert_eq!(l[0]["items"]["matched"], 2);
        assert_eq!(
            codes(&s)
                .iter()
                .filter(|c| *c == COLLECTION_DIGEST_MISMATCH)
                .count(),
            1
        );
        assert!(s.high >= 1);
    }

    #[test]
    fn a_package_copied_without_its_logs_is_unverified() {
        let t = tempfile::tempdir().unwrap();
        let pkg = package(&t.path().join("usb"));
        let copy = t.path().join("copied").join("package");
        for e in walkdir::WalkDir::new(&pkg) {
            let e = e.unwrap();
            let dst = copy.join(e.path().strip_prefix(&pkg).unwrap());
            if e.file_type().is_dir() {
                std::fs::create_dir_all(&dst).unwrap();
            } else {
                std::fs::copy(e.path(), &dst).unwrap();
            }
        }
        let (s, e) = intake(&copy, &t.path().join("out"));
        let l = linked(&e);
        assert_eq!(l[0]["verification"], "missing", "{}", l[0]);
        assert_eq!(l[0]["items"]["unverified"], 3);
        assert_eq!(
            codes(&s)
                .iter()
                .filter(|c| *c == COLLECTION_LOG_UNVERIFIED)
                .count(),
            3
        );
        assert_eq!(s.high, 0);
    }

    #[test]
    fn an_ordinary_folder_has_no_collection_link() {
        let t = tempfile::tempdir().unwrap();
        let ev = t.path().join("ev");
        std::fs::create_dir_all(&ev).unwrap();
        std::fs::write(ev.join("manifest.json"), br#"{"format":"something-else"}"#).unwrap();
        let (_, e) = intake(&ev, &t.path().join("out"));
        assert!(linked(&e).is_empty());
        assert!(detect(&[ev]).is_empty());
    }

    #[test]
    fn collection_log_location_follows_the_standard_layout() {
        let p = Path::new("/x/out/reports/RUN-1/package");
        assert_eq!(
            collection_log(p),
            Some(PathBuf::from("/x/out/logs/RUN-1/audit.jsonl"))
        );
        assert_eq!(collection_log(Path::new("/x/elsewhere/package")), None);
    }
}

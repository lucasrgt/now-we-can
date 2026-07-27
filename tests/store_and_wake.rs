mod common;

use common::*;
use std::{fs, path::Path};
use wmw::{Cue, CueKind};

#[test]
fn init_installs_assets_idempotently_and_confines_paths() {
    let temp = repo();
    fs::write(
        temp.path().join(".gitignore"),
        "# Existing\r\n\r\n# Wake Me When local configuration and disposable state\r\n.wmw/config.local.toml\r\n",
    )
    .unwrap();
    wmw::init(temp.path(), &[Path::new("AGENTS.md").into(), Path::new("CLAUDE.md").into()]).unwrap();
    wmw::init(temp.path(), &[Path::new("AGENTS.md").into()]).unwrap();
    assert!(temp.path().join(".wmw/SKILL.md").is_file());
    assert!(temp.path().join(".wmw/config.local.toml").is_file());
    assert_eq!(
        fs::read_to_string(temp.path().join("AGENTS.md"))
            .unwrap()
            .matches("wmw:instructions:start")
            .count(),
        1
    );
    assert!(
        fs::read_to_string(temp.path().join(".gitignore"))
            .unwrap()
            .contains(".wmw/config.local.toml")
    );
    assert_eq!(
        fs::read_to_string(temp.path().join(".gitignore"))
            .unwrap()
            .matches(".wmw/config.local.toml")
            .count(),
        1
    );
    assert!(wmw::init(temp.path(), &[Path::new("../outside.md").into()]).is_err());
    assert!(wmw::repository(Path::new("Z:/definitely-not-a-repository")).is_err());
}

#[test]
fn init_migrates_not_yet_storage_and_managed_instructions() {
    let temp = repo();
    fs::create_dir_all(temp.path().join(".notyet/deferments")).unwrap();
    fs::write(temp.path().join(".notyet/deferments/kept.toml"), "preserved").unwrap();
    fs::write(
        temp.path().join("AGENTS.md"),
        "<!-- notyet:instructions:start -->\nold instructions\n<!-- notyet:instructions:end -->\n",
    )
    .unwrap();

    wmw::init(temp.path(), &[Path::new("AGENTS.md").into()]).unwrap();

    assert!(!temp.path().join(".notyet").exists());
    assert_eq!(
        fs::read_to_string(temp.path().join(".wmw/deferments/kept.toml")).unwrap(),
        "preserved"
    );
    let agents = fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("<!-- wmw:instructions:start -->"));
    assert!(!agents.contains("notyet:instructions"));
}

#[test]
fn collect_requires_two_identical_evidence_bounded_passes_and_deduplicates() {
    let temp = initialized();
    change(temp.path());
    let mut expected = candidate(CueKind::Event, "", "mobile-v1-retired", "event");
    expected.evidence[0] = "The dual-write remains\nuntil the named cue.".into();
    configure(temp.path(), std::slice::from_ref(&expected), "accept");
    let mut input = request();
    input.final_message = "The dual-write remains\nuntil the named cue.".into();
    let result = wmw::collect(temp.path(), input.clone()).unwrap();
    assert_eq!(result.candidates_found, 1);
    assert_eq!(result.recorded.len(), 1);
    assert_eq!(result.recorded[0].cue.value, "mobile-v1-retired");
    assert_eq!(result.recorded[0].recorded_by, "Future Author");
    assert!(
        temp.path()
            .join(format!(".wmw/deferments/{}.toml", result.recorded[0].id))
            .is_file()
    );

    let duplicate = wmw::collect(temp.path(), input).unwrap();
    assert_eq!(duplicate.duplicates, 1);
    assert!(duplicate.recorded.is_empty());

    configure(
        temp.path(),
        &[candidate(CueKind::Event, "", "another-event", "rejected")],
        "reject-second",
    );
    assert!(wmw::collect(temp.path(), request()).unwrap().recorded.is_empty());
}

#[test]
fn collect_rejects_invented_or_malformed_candidates_and_judge_failures() {
    let temp = initialized();
    change(temp.path());
    let mut invalid = candidate(CueKind::Event, "", "event", "invalid");
    invalid.evidence[0] = "invented evidence absent from envelope".into();
    configure(temp.path(), &[invalid], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());

    let mut bad_scope = candidate(CueKind::Event, "", "event", "scope");
    bad_scope.scopes = vec!["[".into()];
    configure(temp.path(), &[bad_scope], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());

    let mut bad_cue = candidate(CueKind::Event, "src/file", "event", "cue");
    configure(temp.path(), &[bad_cue.clone()], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());
    bad_cue.cue = Cue {
        kind: CueKind::PathExists,
        path: "../outside".into(),
        value: String::new(),
    };
    configure(temp.path(), &[bad_cue], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());

    configure(temp.path(), &[candidate(CueKind::Event, "", "event", "invalid-json")], "invalid");
    assert!(wmw::collect(temp.path(), request()).is_err());
    configure(temp.path(), &[candidate(CueKind::Event, "", "event", "exit")], "exit");
    assert!(wmw::collect(temp.path(), request()).is_err());
    configure(temp.path(), &[candidate(CueKind::Event, "", "event", "many")], "too-many");
    assert!(wmw::collect(temp.path(), request()).is_err());
}

#[test]
fn wake_evaluates_every_deterministic_cue_and_resolve_removes_due_work() {
    let temp = initialized();
    fs::write(temp.path().join("src/present.rs"), "warning_token\n").unwrap();
    write_deferment(
        temp.path(),
        "event",
        Cue {
            kind: CueKind::Event,
            path: String::new(),
            value: "release-ready".into(),
        },
    );
    write_deferment(
        temp.path(),
        "exists",
        Cue {
            kind: CueKind::PathExists,
            path: "src/present.rs".into(),
            value: String::new(),
        },
    );
    write_deferment(
        temp.path(),
        "absent",
        Cue {
            kind: CueKind::PathAbsent,
            path: "src/gone.rs".into(),
            value: String::new(),
        },
    );
    write_deferment(
        temp.path(),
        "contains",
        Cue {
            kind: CueKind::FileContains,
            path: "src/present.rs".into(),
            value: "warning_token".into(),
        },
    );
    write_deferment(
        temp.path(),
        "not-contains",
        Cue {
            kind: CueKind::FileNotContains,
            path: "src/present.rs".into(),
            value: "legacy".into(),
        },
    );
    write_deferment(
        temp.path(),
        "missing-file",
        Cue {
            kind: CueKind::FileNotContains,
            path: "src/missing.rs".into(),
            value: "legacy".into(),
        },
    );

    let sleeping = wmw::wake(temp.path(), &[]).unwrap();
    assert_eq!(sleeping.active, 6);
    assert_eq!(sleeping.due.len(), 4);
    let awake = wmw::wake(temp.path(), &["RELEASE-READY".into()]).unwrap();
    assert_eq!(awake.due.len(), 5);

    let resolved = wmw::resolve(temp.path(), "event", "commit abc proves completion").unwrap();
    assert!(resolved.resolved_at.is_some());
    assert_eq!(wmw::wake(temp.path(), &["release-ready".into()]).unwrap().due.len(), 4);
    assert!(wmw::resolve(temp.path(), "event", "again").is_err());
    assert!(wmw::resolve(temp.path(), "missing", "proof").is_err());
    assert!(wmw::resolve(temp.path(), "../outside", "proof").is_err());
    assert!(wmw::resolve(temp.path(), "exists", "").is_err());
}

#[test]
fn storage_and_request_boundaries_fail_closed() {
    let raw = repo();
    assert!(wmw::wake(raw.path(), &[]).is_err());
    let temp = initialized();
    change(temp.path());
    configure(temp.path(), &[candidate(CueKind::Event, "", "event", "boundary")], "accept");
    let mut invalid = request();
    invalid.task.clear();
    assert!(wmw::collect(temp.path(), invalid).is_err());
    let mut invalid = request();
    invalid.base = "--bad".into();
    assert!(wmw::collect(temp.path(), invalid).is_err());
    let mut missing = request();
    missing.base = "missing-revision".into();
    assert!(wmw::collect(temp.path(), missing).is_err());
    let mut huge = request();
    huge.final_message = "x".repeat(121_000);
    assert!(wmw::collect(temp.path(), huge).is_err());
    fs::write(temp.path().join(".wmw/deferments/bad.toml"), "not = [valid").unwrap();
    assert!(wmw::wake(temp.path(), &[]).is_err());
}

#[test]
fn collection_validates_file_cues_evidence_counts_and_config_schema() {
    let temp = initialized();
    change(temp.path());
    let file = candidate(CueKind::FileNotContains, "src/generated/User.ts", "name:", "file");
    configure(temp.path(), &[file], "accept");
    let mut file_request = request();
    file_request
        .plan
        .push_str(" Remove it when src/generated/User.ts no longer contains name:.");
    assert_eq!(wmw::collect(temp.path(), file_request).unwrap().recorded.len(), 1);

    let mut missing_scope = candidate(CueKind::Event, "", "event", "missing-scope");
    missing_scope.scopes.clear();
    configure(temp.path(), &[missing_scope], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());
    let mut one_evidence = candidate(CueKind::Event, "", "event", "one-evidence");
    one_evidence.evidence.truncate(1);
    configure(temp.path(), &[one_evidence], "accept");
    assert!(wmw::collect(temp.path(), request()).is_err());

    fs::write(temp.path().join(".wmw/config.local.toml"), "schema = 2\n[judge]\ncommand = []\n").unwrap();
    assert!(wmw::collect(temp.path(), request()).is_err());
}

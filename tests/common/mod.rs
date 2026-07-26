#![allow(dead_code)]

use chrono::Utc;
use notyet::{Candidate, Cue, CueKind, Deferment};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

pub fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git").args(args).current_dir(root).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

pub fn repo() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    git(temp.path(), &["init", "-b", "main"]);
    git(temp.path(), &["config", "user.name", "Future Author"]);
    git(temp.path(), &["config", "user.email", "future@example.test"]);
    fs::create_dir_all(temp.path().join("src/generated")).unwrap();
    fs::write(temp.path().join("src/generated/User.ts"), "export type User = { name: string };\n").unwrap();
    fs::write(temp.path().join("README.md"), "# Fixture\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "initial"]);
    temp
}

pub fn initialized() -> TempDir {
    let temp = repo();
    notyet::init(temp.path(), &[Path::new("AGENTS.md").into()]).unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "adopt not yet"]);
    temp
}

pub fn candidate(kind: CueKind, path: &str, value: &str, suffix: &str) -> Candidate {
    Candidate {
        title: format!("Remove temporary compatibility {suffix}"),
        action: format!("Remove the LegacyName dual-write {suffix}"),
        blocker: "mobile v1 still reads LegacyName".into(),
        cue: Cue {
            kind,
            path: path.into(),
            value: value.into(),
        },
        scopes: vec!["src/**".into()],
        evidence: vec!["mobile v1 still reads LegacyName".into(), "customer.LegacyName = input.Name".into()],
    }
}

pub fn configure(root: &Path, candidates: &[Candidate], mode: &str) {
    let candidates_path = root.join(".notyet/judge-candidates.json");
    fs::write(&candidates_path, serde_json::to_string(candidates).unwrap()).unwrap();
    let script = root.join(".notyet/judge.py");
    fs::write(
        &script,
        r#"import json,sys
mode,path=sys.argv[1:3]
prompt=sys.stdin.read()
if mode=="exit": sys.exit(7)
if mode=="invalid": print("not json"); sys.exit(0)
items=json.load(open(path,encoding="utf-8"))
if mode=="reject-second" and "Confirm only supported" in prompt: items=[]
if mode=="too-many": items=items*21
print(json.dumps({"deferments":items}))
"#,
    )
    .unwrap();
    let command = serde_json::to_string(&vec![
        "python".to_string(),
        script.display().to_string(),
        mode.into(),
        candidates_path.display().to_string(),
    ])
    .unwrap();
    fs::write(
        root.join(".notyet/config.local.toml"),
        format!("schema = 1\n[judge]\ncommand = {command}\n"),
    )
    .unwrap();
}

pub fn request() -> notyet::CollectRequest {
    notyet::CollectRequest {
        task: "Migrate customer writes while mobile v1 remains active".into(),
        plan: "Keep compatibility because mobile v1 still reads LegacyName.".into(),
        final_message: "The dual-write remains until the named cue.".into(),
        base: "HEAD".into(),
    }
}

pub fn change(root: &Path) {
    fs::write(root.join("src/customer.rs"), "customer.LegacyName = input.Name;\n").unwrap();
}

pub fn write_deferment(root: &Path, id: &str, cue: Cue) {
    let item = Deferment {
        schema: 1,
        id: id.into(),
        title: format!("Deferred {id}"),
        action: format!("Complete {id}"),
        blocker: "A prerequisite is false".into(),
        cue,
        scopes: vec!["src/**".into()],
        evidence: vec!["evidence one".into(), "evidence two".into()],
        recorded_at: Utc::now(),
        recorded_by: "Fixture".into(),
        recorded_commit: git(root, &["rev-parse", "HEAD"]).trim().into(),
        resolved_at: None,
        resolution_evidence: None,
    };
    fs::write(
        root.join(format!(".notyet/deferments/{id}.toml")),
        toml::to_string_pretty(&item).unwrap(),
    )
    .unwrap();
}

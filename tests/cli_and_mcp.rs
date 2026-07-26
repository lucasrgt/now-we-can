mod common;

use common::*;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::{self, Cursor, Write},
    process::Command,
};
use wmw::CueKind;

struct FailWriter;

impl Write for FailWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("writer failed"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn os_args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn cli(root: &std::path::Path, args: &[&str]) -> anyhow::Result<(i32, String)> {
    let mut arguments = vec!["wmw"];
    arguments.extend_from_slice(args);
    let mut output = Vec::new();
    let code = wmw::run_cli_at(os_args(&arguments), root, &mut Cursor::new(""), &mut output)?;
    Ok((code, String::from_utf8(output).unwrap()))
}

#[test]
fn cli_covers_init_collect_wake_resolve_check_and_entrypoint() {
    assert!(wmw::run_cli_env().is_err());
    let temp = repo();
    assert!(
        cli(temp.path(), &["init", "--agent-file", "CLAUDE.md"])
            .unwrap()
            .1
            .contains("initialized")
    );
    change(temp.path());
    let expected = candidate(CueKind::Event, "", "mobile-v1-retired", "cli");
    configure(temp.path(), &[expected], "accept");
    std::fs::write(
        temp.path().join("PLAN.md"),
        "Keep compatibility because mobile v1 still reads LegacyName.",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("FINAL.md"),
        "customer.LegacyName = input.Name remains until retirement.",
    )
    .unwrap();
    let (code, output) = cli(
        temp.path(),
        &[
            "collect",
            "--task",
            "Migrate customer writes",
            "--plan",
            "PLAN.md",
            "--final-message",
            "FINAL.md",
            "--json",
        ],
    )
    .unwrap();
    assert_eq!(code, 0);
    let collected: Value = serde_json::from_str(&output).unwrap();
    let id = collected["recorded"][0]["id"].as_str().unwrap();
    assert!(
        cli(
            temp.path(),
            &[
                "collect",
                "--task",
                "Migrate customer writes",
                "--plan",
                "PLAN.md",
                "--final-message",
                "FINAL.md"
            ]
        )
        .unwrap()
        .1
        .contains("Collected")
    );
    assert!(cli(temp.path(), &["wake"]).unwrap().1.contains("No deferments"));
    assert_eq!(cli(temp.path(), &["check", "--event", "mobile-v1-retired"]).unwrap().0, 1);
    assert!(
        cli(temp.path(), &["wake", "--event", "mobile-v1-retired"])
            .unwrap()
            .1
            .contains("> WOKE:")
    );
    assert!(
        cli(temp.path(), &["resolve", "--id", id, "--evidence", "completed"])
            .unwrap()
            .1
            .contains("Resolved")
    );
    assert_eq!(cli(temp.path(), &["check", "--event", "mobile-v1-retired", "--json"]).unwrap().0, 0);
    assert!(cli(temp.path(), &["collect"]).is_err());
    assert!(cli(temp.path(), &["collect", "--task", "x", "--plan", "../outside"]).is_err());

    let mut binary = std::env::current_exe().unwrap();
    binary.pop();
    binary.pop();
    binary.push(if cfg!(windows) { "wmw.exe" } else { "wmw" });
    let version = Command::new(binary).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        format!("wmw {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn cli_and_mcp_propagate_protocol_and_output_failures() {
    let temp = initialized();
    assert!(wmw::run_cli_at(os_args(&["wmw", "wake"]), temp.path(), &mut Cursor::new(""), &mut FailWriter).is_err());
    assert!(wmw::run_cli_at(os_args(&["wmw", "--help"]), temp.path(), &mut Cursor::new(""), &mut Vec::new()).is_ok());
    assert!(wmw::mcp_stream(&mut Cursor::new("not json\n"), &mut Vec::new()).is_err());
    assert!(
        wmw::mcp_stream(
            &mut Cursor::new("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n"),
            &mut FailWriter
        )
        .is_err()
    );
}

#[test]
fn mcp_exposes_every_operation_through_the_shared_core() {
    let temp = initialized();
    change(temp.path());
    configure(temp.path(), &[candidate(CueKind::Event, "", "mobile-v1-retired", "mcp")], "accept");
    let repository = temp.path().display().to_string();
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"wmw_collect","arguments":{"repository":repository,"task":"Migrate customer writes","plan":"mobile v1 still reads LegacyName","final_message":"customer.LegacyName = input.Name"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"wmw_wake","arguments":{"repository":repository,"events":["mobile-v1-retired"]}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"wmw_check","arguments":{"repository":repository,"events":[]}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"unknown","arguments":{"repository":repository}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"unknown","params":{}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"wmw_resolve","arguments":{"repository":repository,"id":"missing"}}}),
    ];
    let input = requests
        .iter()
        .map(|request| serde_json::to_string(request).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let mut output = Vec::new();
    wmw::mcp_stream(&mut Cursor::new(input), &mut output).unwrap();
    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 9);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "wake-me-when");
    assert_eq!(responses[2]["result"]["tools"].as_array().unwrap().len(), 4);
    assert_eq!(responses[3]["result"]["structuredContent"]["recorded"].as_array().unwrap().len(), 1);
    assert_eq!(responses[4]["result"]["structuredContent"]["due"].as_array().unwrap().len(), 1);
    assert_eq!(responses[6]["result"]["isError"], true);
    assert_eq!(responses[7]["error"]["code"], -32601);
    assert_eq!(responses[8]["result"]["isError"], true);

    let request = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}\n";
    let mut cli_output = Vec::new();
    assert_eq!(
        wmw::run_cli_at(os_args(&["wmw", "mcp"]), temp.path(), &mut Cursor::new(request), &mut cli_output).unwrap(),
        0
    );
    assert!(String::from_utf8(cli_output).unwrap().contains("wmw_collect"));
}

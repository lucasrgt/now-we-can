use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[rustfmt::skip]
use std::{collections::HashSet, ffi::OsString, fs, io::{self, BufRead, Write}, path::{Component, Path, PathBuf}, process::{Command, Stdio}};
use ulid::Ulid;

const SKILL: &str = include_str!("../assets/now-we-can/SKILL.md");
const CONFIG: &str = include_str!("../assets/config.toml");
const IGNORE: &str = include_str!("../assets/gitignore");
const INSTRUCTIONS: &str = include_str!("../assets/AGENT_INSTRUCTIONS.md");
const START: &str = "<!-- nwc:instructions:start -->";
const END: &str = "<!-- nwc:instructions:end -->";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[rustfmt::skip]
pub enum CueKind { Event, PathExists, PathAbsent, FileContains, FileNotContains }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub struct Cue { pub kind: CueKind, #[serde(default)] pub path: String, #[serde(default)] pub value: String }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub struct Candidate { pub title: String, pub action: String, pub blocker: String, pub cue: Cue, pub scopes: Vec<String>, pub evidence: Vec<String> }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub struct Deferment { pub schema: u8, pub id: String, pub title: String, pub action: String, pub blocker: String, pub cue: Cue, pub scopes: Vec<String>, pub evidence: Vec<String>, pub recorded_at: DateTime<Utc>, pub recorded_by: String, pub recorded_commit: String, pub resolved_at: Option<DateTime<Utc>>, pub resolution_evidence: Option<String> }

#[derive(Clone, Debug, Default)]
#[rustfmt::skip]
pub struct CollectRequest { pub task: String, pub plan: String, pub final_message: String, pub base: String }

#[derive(Clone, Debug, Serialize, PartialEq)]
#[rustfmt::skip]
pub struct CollectResult { pub candidates_found: usize, pub duplicates: usize, pub recorded: Vec<Deferment> }

#[derive(Clone, Debug, Serialize, PartialEq)]
#[rustfmt::skip]
pub struct WakeResult { pub active: usize, pub due: Vec<Deferment> }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: u8,
    judge: Judge,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Judge {
    command: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Extraction {
    deferments: Vec<Candidate>,
}

pub fn repository(path: &Path) -> Result<PathBuf> {
    let root = git(path, &["rev-parse", "--show-toplevel"])?;
    fs::canonicalize(root.trim()).context("resolve repository root")
}

pub fn init(root: &Path, agent_files: &[PathBuf]) -> Result<()> {
    let root = repository(root)?;
    for legacy in [".wmw", ".notyet"] {
        if root.join(legacy).exists() && !root.join(".nwc").exists() {
            fs::rename(root.join(legacy), root.join(".nwc"))?;
        }
    }
    fs::create_dir_all(root.join(".nwc/deferments"))?;
    write_new(root.join(".nwc/config.local.toml"), CONFIG)?;
    fs::write(root.join(".nwc/SKILL.md"), SKILL)?;
    append_once(root.join(".gitignore"), IGNORE)?;
    for file in agent_files {
        safe_relative(file)?;
        upsert_block(root.join(file), INSTRUCTIONS)?;
    }
    Ok(())
}

pub fn collect(root: &Path, request: CollectRequest) -> Result<CollectResult> {
    let root = repository(root)?;
    require_text("task", &request.task)?;
    validate_revision(&request.base)?;
    git(&root, &["rev-parse", "--verify", &request.base])?;
    let diff = diff(&root, &request.base)?;
    let envelope = json!({"task":request.task,"plan":request.plan,"final_message":request.final_message,"diff":diff});
    let evidence = serde_json::to_string(&envelope)?;
    let literal = format!(
        "{}\n{}\n{}\n{}",
        request.task,
        request.plan,
        request.final_message,
        envelope["diff"].as_str().unwrap_or_default()
    );
    if evidence.len() > 120_000 {
        bail!("collection envelope exceeds 120000 bytes")
    }
    let first = validated(judge(&root, &collect_prompt(&evidence, None)?)?.deferments, &literal)?;
    let second = validated(judge(&root, &collect_prompt(&evidence, Some(&first))?)?.deferments, &literal)?;
    let confirmed = first.into_iter().filter(|item| second.contains(item)).collect::<Vec<_>>();
    let existing = load(&root)?;
    let mut recorded = Vec::new();
    let mut duplicates = 0;
    for candidate in confirmed.iter().cloned() {
        if existing.iter().chain(recorded.iter()).any(|item| same(item, &candidate)) {
            duplicates += 1;
            continue;
        }
        let deferment = Deferment {
            schema: 1,
            id: Ulid::generate().to_string().to_lowercase(),
            title: candidate.title,
            action: candidate.action,
            blocker: candidate.blocker,
            cue: candidate.cue,
            scopes: candidate.scopes,
            evidence: candidate.evidence,
            recorded_at: Utc::now(),
            recorded_by: git(&root, &["config", "user.name"])
                .unwrap_or_else(|_| "unknown".into())
                .trim()
                .into(),
            recorded_commit: git(&root, &["rev-parse", "HEAD"])?.trim().into(),
            resolved_at: None,
            resolution_evidence: None,
        };
        write_new(
            root.join(format!(".nwc/deferments/{}.toml", deferment.id)),
            &toml::to_string_pretty(&deferment)?,
        )?;
        recorded.push(deferment);
    }
    Ok(CollectResult {
        candidates_found: confirmed.len(),
        duplicates,
        recorded,
    })
}

pub fn wake(root: &Path, events: &[String]) -> Result<WakeResult> {
    let root = repository(root)?;
    let all = load(&root)?;
    let active = all.iter().filter(|item| item.resolved_at.is_none()).count();
    let events = events.iter().map(|item| item.trim().to_lowercase()).collect::<HashSet<_>>();
    let due = all
        .into_iter()
        .filter(|item| item.resolved_at.is_none() && cue_is_due(&root, &item.cue, &events))
        .collect();
    Ok(WakeResult { active, due })
}

pub fn resolve(root: &Path, id: &str, evidence: &str) -> Result<Deferment> {
    let root = repository(root)?;
    require_text("id", id)?;
    require_text("evidence", evidence)?;
    let path = root.join(format!(".nwc/deferments/{id}.toml"));
    safe_relative(Path::new(&format!(".nwc/deferments/{id}.toml")))?;
    let mut item: Deferment = toml::from_str(&fs::read_to_string(&path).with_context(|| format!("unknown deferment {id}"))?)?;
    if item.resolved_at.is_some() {
        bail!("deferment {id} is already resolved")
    }
    item.resolved_at = Some(Utc::now());
    item.resolution_evidence = Some(evidence.trim().into());
    fs::write(path, toml::to_string_pretty(&item)?)?;
    Ok(item)
}

fn load(root: &Path) -> Result<Vec<Deferment>> {
    let directory = root.join(".nwc/deferments");
    if !directory.exists() {
        bail!("Now We Can is not initialized; run nwc init")
    }
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    paths.sort();
    paths
        .into_iter()
        .filter(|path| path.extension().is_some_and(|value| value == "toml"))
        .map(|path| toml::from_str(&fs::read_to_string(&path)?).with_context(|| format!("invalid deferment {}", path.display())))
        .collect()
}

fn collect_prompt(envelope: &str, candidates: Option<&[Candidate]>) -> Result<String> {
    let phase = if let Some(items) = candidates {
        format!(
            "Confirm only supported candidates from this list and copy every accepted object byte-for-byte in meaning and fields: {}.",
            serde_json::to_string(items)?
        )
    } else {
        "Extract conditional deferments from the envelope.".into()
    };
    Ok(format!(
        "You are the bounded Now We Can collector. {phase} A deferment requires a concrete action intentionally left undone because a currently false prerequisite blocks it, a machine-checkable cue, at least one reusable glob scope, and at least two evidence strings copied verbatim from the envelope. Allowed cue kinds: event (empty path, stable event value), path_exists/path_absent (repository-relative path, empty value), file_contains/file_not_contains (repository-relative path and literal value). Reject aspirations, optional improvements, unfinished current scope, permanent behavior, vague later work, completed work, and invented facts. Return strict JSON {{\"deferments\":[{{\"title\":\"\",\"action\":\"\",\"blocker\":\"\",\"cue\":{{\"kind\":\"event\",\"path\":\"\",\"value\":\"\"}},\"scopes\":[\"src/**\"],\"evidence\":[\"verbatim fragment\",\"verbatim fragment\"]}}]}} and nothing else.\nENVELOPE:\n{envelope}"
    ))
}

fn validated(items: Vec<Candidate>, envelope: &str) -> Result<Vec<Candidate>> {
    if items.len() > 20 {
        bail!("judge returned too many deferments")
    }
    let haystack = envelope.to_lowercase();
    let mut output = Vec::new();
    for mut item in items {
        for (name, value) in [("title", &item.title), ("action", &item.action), ("blocker", &item.blocker)] {
            require_text(name, value)?;
        }
        item.scopes = normalized(item.scopes);
        item.evidence = normalized(item.evidence);
        if item.scopes.is_empty() || item.evidence.len() < 2 {
            bail!("deferment requires scopes and two evidence fragments")
        }
        for scope in &item.scopes {
            Pattern::new(scope).with_context(|| format!("invalid scope {scope}"))?;
        }
        if item
            .evidence
            .iter()
            .any(|value| value.len() < 8 || !haystack.contains(&value.to_lowercase()))
        {
            bail!("judge returned invented evidence")
        }
        validate_cue(&mut item.cue)?;
        if !output.contains(&item) {
            output.push(item);
        }
    }
    Ok(output)
}

fn validate_cue(cue: &mut Cue) -> Result<()> {
    cue.path = cue.path.trim().replace('\\', "/");
    cue.value = cue.value.trim().into();
    match cue.kind {
        CueKind::Event if cue.path.is_empty() && !cue.value.is_empty() => cue.value = cue.value.to_lowercase(),
        CueKind::PathExists | CueKind::PathAbsent if !cue.path.is_empty() && cue.value.is_empty() => safe_relative(Path::new(&cue.path))?,
        CueKind::FileContains | CueKind::FileNotContains if !cue.path.is_empty() && !cue.value.is_empty() => {
            safe_relative(Path::new(&cue.path))?
        }
        _ => bail!("cue fields do not match cue kind"),
    }
    Ok(())
}

fn cue_is_due(root: &Path, cue: &Cue, events: &HashSet<String>) -> bool {
    let path = root.join(&cue.path);
    match cue.kind {
        CueKind::Event => events.contains(&cue.value.to_lowercase()),
        CueKind::PathExists => path.exists(),
        CueKind::PathAbsent => !path.exists(),
        CueKind::FileContains => fs::read_to_string(path).is_ok_and(|value| value.contains(&cue.value)),
        CueKind::FileNotContains => fs::read_to_string(path).is_ok_and(|value| !value.contains(&cue.value)),
    }
}

fn judge(root: &Path, prompt: &str) -> Result<Extraction> {
    let config = load_config(root)?;
    if config.schema != 1 || config.judge.command.is_empty() {
        bail!("unsupported or empty judge configuration")
    }
    let mut child = Command::new(&config.judge.command[0])
        .args(&config.judge.command[1..])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("start judge")?;
    child.stdin.take().context("open judge stdin")?.write_all(prompt.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("judge exited with {}", output.status)
    }
    serde_json::from_slice(&output.stdout).context("judge returned invalid JSON")
}

#[rustfmt::skip]
fn load_config(root: &Path) -> Result<Config> {
    let user = dirs::config_dir().map(|path| path.join("now-we-can/config.toml"));
    let path = [Some(root.join(".nwc/config.local.toml")), Some(root.join(".nwc/config.toml")), user].into_iter().flatten().find(|path| path.exists()).ok_or_else(|| anyhow!("missing judge configuration"))?;
    toml::from_str(&fs::read_to_string(path)?).context("invalid judge configuration")
}

#[rustfmt::skip]
fn diff(root: &Path, base: &str) -> Result<String> {
    let mut value = git(root, &["diff", "--no-ext-diff", "--unified=3", base, "--", ".", ":(exclude).nwc/**", ":(exclude).wmw/**", ":(exclude).notyet/**"])?;
    for path in git(root, &["ls-files", "--others", "--exclude-standard", "--", ".", ":(exclude).nwc/**", ":(exclude).wmw/**", ":(exclude).notyet/**"])?.lines() {
        let contents = fs::read_to_string(root.join(path)).with_context(|| format!("untracked file is not auditable text: {path}"))?;
        value.push_str(&format!("\ndiff --git a/{path} b/{path}\n--- /dev/null\n+++ b/{path}\n"));
        for line in contents.lines() {
            value.push_str(&format!("+{line}\n"));
        }
    }
    Ok(value)
}

fn same(item: &Deferment, candidate: &Candidate) -> bool {
    item.resolved_at.is_none() && item.action == candidate.action && item.blocker == candidate.blocker && item.cue == candidate.cue
}

fn git(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(["-c", "core.quotePath=false"])
        .args(arguments)
        .current_dir(root)
        .output()
        .context("start git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
    String::from_utf8(output.stdout).context("git output was not UTF-8")
}

#[rustfmt::skip]
fn safe_relative(path: &Path) -> Result<()> { if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|part| matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { bail!("path must stay inside the repository: {}", path.display()) } Ok(()) }
#[rustfmt::skip]
fn require_text(name: &str, value: &str) -> Result<()> { if value.trim().is_empty() { bail!("{name} must not be empty") } else { Ok(()) } }
#[rustfmt::skip]
fn normalized(values: Vec<String>) -> Vec<String> { let mut values = values.into_iter().map(|value| value.trim().replace('\\', "/")).filter(|value| !value.is_empty()).collect::<Vec<_>>(); values.sort(); values.dedup(); values }
#[rustfmt::skip]
fn validate_revision(value: &str) -> Result<()> { if value.is_empty() || value.starts_with('-') || !value.chars().all(|character| character.is_ascii_alphanumeric() || "_./~^-".contains(character)) { bail!("invalid base revision") } Ok(()) }
#[rustfmt::skip]
fn write_new(path: PathBuf, contents: &str) -> Result<()> { if !path.exists() { fs::write(path, contents)?; } Ok(()) }
#[rustfmt::skip]
fn append_once(path: PathBuf, block: &str) -> Result<()> { let current = fs::read_to_string(&path).unwrap_or_default().replace("# Wake Me When local configuration and disposable state\r\n.wmw/config.local.toml", block.trim()).replace("# Wake Me When local configuration and disposable state\n.wmw/config.local.toml", block.trim()); let block = block.replace("\r\n", "\n"); if !current.replace("\r\n", "\n").contains(block.trim()) { fs::write(path, format!("{}{}{}\n", current, if current.is_empty() || current.ends_with('\n') { "" } else { "\n" }, block.trim()))?; } else { fs::write(path, current)?; } Ok(()) }
#[rustfmt::skip]
fn upsert_block(path: PathBuf, block: &str) -> Result<()> { let current = fs::read_to_string(&path).unwrap_or_default().replace("<!-- notyet:instructions:start -->", START).replace("<!-- notyet:instructions:end -->", END).replace("<!-- wmw:instructions:start -->", START).replace("<!-- wmw:instructions:end -->", END); let updated = if let (Some(start), Some(end)) = (current.find(START), current.find(END)) { format!("{}{}{}", &current[..start], block.trim(), &current[end + END.len()..]) } else { format!("{}{}{}\n", current, if current.is_empty() || current.ends_with('\n') { "" } else { "\n" }, block.trim()) }; fs::write(path, updated)?; Ok(()) }

#[derive(Parser)]
#[command(name = "nwc", version, about = "Wake deferred agent intentions when their cue becomes true")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init(InitArgs),
    Collect(CollectArgs),
    Wake(EventArgs),
    Resolve(ResolveArgs),
    Check(EventArgs),
    Mcp,
}

#[derive(Args)]
struct InitArgs {
    #[arg(long, default_value = "AGENTS.md")]
    agent_file: Vec<PathBuf>,
}

#[derive(Args)]
#[rustfmt::skip]
struct CollectArgs { #[arg(long)] task: String, #[arg(long)] plan: Option<PathBuf>, #[arg(long)] final_message: Option<PathBuf>, #[arg(long, default_value = "HEAD")] base: String, #[arg(long)] json: bool }

#[derive(Args)]
#[rustfmt::skip]
struct EventArgs { #[arg(long)] event: Vec<String>, #[arg(long)] json: bool }

#[derive(Args)]
#[rustfmt::skip]
struct ResolveArgs { #[arg(long)] id: String, #[arg(long)] evidence: String, #[arg(long)] json: bool }

pub fn run_cli_env() -> Result<i32> {
    let current = std::env::current_dir()?;
    run_cli_at(std::env::args_os().collect(), &current, &mut io::stdin().lock(), &mut io::stdout())
}

pub fn run_cli_at(arguments: Vec<OsString>, current: &Path, input: &mut dyn BufRead, output: &mut dyn Write) -> Result<i32> {
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) if error.use_stderr() => return Err(error.into()),
        Err(error) => {
            write!(output, "{error}")?;
            return Ok(0);
        }
    };
    match cli.command {
        Commands::Init(args) => {
            init(current, &args.agent_file)?;
            writeln!(output, "Now We Can initialized.")?;
        }
        Commands::Collect(args) => {
            let root = repository(current)?;
            let request = CollectRequest {
                task: args.task,
                plan: read_optional(&root, args.plan.as_deref())?,
                final_message: read_optional(&root, args.final_message.as_deref())?,
                base: args.base,
            };
            print_value(&collect(&root, request)?, args.json, output)?;
        }
        Commands::Wake(args) => print_value(&wake(current, &args.event)?, args.json, output)?,
        Commands::Resolve(args) => print_value(&resolve(current, &args.id, &args.evidence)?, args.json, output)?,
        Commands::Check(args) => {
            let result = wake(current, &args.event)?;
            print_value(&result, args.json, output)?;
            return Ok(i32::from(!result.due.is_empty()));
        }
        Commands::Mcp => mcp_stream(input, output)?,
    }
    Ok(0)
}

fn read_optional(root: &Path, path: Option<&Path>) -> Result<String> {
    let Some(path) = path else { return Ok(String::new()) };
    safe_relative(path)?;
    fs::read_to_string(root.join(path)).with_context(|| format!("read {}", path.display()))
}

fn print_value<T: Serialize>(value: &T, json_output: bool, output: &mut dyn Write) -> Result<()> {
    if json_output {
        writeln!(output, "{}", serde_json::to_string_pretty(value)?)?;
    } else {
        let value = serde_json::to_value(value)?;
        if let Some(due) = value.get("due").and_then(Value::as_array) {
            if due.is_empty() {
                writeln!(output, "No deferments are due.")?;
            }
            for item in due {
                writeln!(
                    output,
                    "> WOKE: {}\n  {}\n  Cue: {}",
                    item["title"].as_str().unwrap_or("Deferment"),
                    item["action"].as_str().unwrap_or(""),
                    serde_json::to_string(&item["cue"])?
                )?;
            }
        } else if let Some(recorded) = value.get("recorded").and_then(Value::as_array) {
            writeln!(output, "Collected {} deferment(s).", recorded.len())?;
            for item in recorded {
                writeln!(output, "> {}", item["title"].as_str().unwrap_or("Deferment"))?;
            }
        } else if value.get("resolved_at").is_some() {
            writeln!(output, "Resolved {}.", value["id"].as_str().unwrap_or("deferment"))?;
        }
    }
    Ok(())
}

pub fn mcp_stream(reader: &mut dyn BufRead, output: &mut dyn Write) -> Result<()> {
    for line in reader.lines() {
        let request: Value = serde_json::from_str(&line?)?;
        if request.get("id").is_none() {
            continue;
        }
        let id = request["id"].clone();
        let response = match request["method"].as_str().unwrap_or_default() {
            "initialize" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"now-we-can","version":env!("CARGO_PKG_VERSION")}}})
            }
            "ping" => json!({"jsonrpc":"2.0","id":id,"result":{}}),
            "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools":mcp_tools()}}),
            "tools/call" => {
                let result = mcp_call(&request["params"]);
                match result {
                    Ok(value) => {
                        json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":serde_json::to_string(&value)?}],"structuredContent":value}})
                    }
                    Err(error) => {
                        json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":error.to_string()}],"isError":true}})
                    }
                }
            }
            _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
        };
        writeln!(output, "{}", serde_json::to_string(&response)?)?;
        output.flush()?;
    }
    Ok(())
}

fn mcp_tools() -> Value {
    json!([
        {"name":"nwc_collect","description":"Collect evidence-backed conditional deferments from completed work","inputSchema":{"type":"object","required":["repository","task"],"properties":{"repository":{"type":"string"},"task":{"type":"string"},"plan":{"type":"string"},"final_message":{"type":"string"},"base":{"type":"string"}}}},
        {"name":"nwc_wake","description":"Return active deferments whose deterministic cue is now true","inputSchema":{"type":"object","required":["repository"],"properties":{"repository":{"type":"string"},"events":{"type":"array","items":{"type":"string"}}}}},
        {"name":"nwc_resolve","description":"Resolve a completed deferment with evidence","inputSchema":{"type":"object","required":["repository","id","evidence"],"properties":{"repository":{"type":"string"},"id":{"type":"string"},"evidence":{"type":"string"}}}},
        {"name":"nwc_check","description":"Fail closed when a due deferment remains unresolved","inputSchema":{"type":"object","required":["repository"],"properties":{"repository":{"type":"string"},"events":{"type":"array","items":{"type":"string"}}}}}
    ])
}

fn mcp_call(parameters: &Value) -> Result<Value> {
    let name = parameters["name"].as_str().unwrap_or_default();
    let arguments = &parameters["arguments"];
    let root = Path::new(arguments["repository"].as_str().context("repository is required")?);
    match name {
        "nwc_collect" => {
            let request = CollectRequest {
                task: arguments["task"].as_str().context("task is required")?.into(),
                plan: arguments["plan"].as_str().unwrap_or_default().into(),
                final_message: arguments["final_message"].as_str().unwrap_or_default().into(),
                base: arguments["base"].as_str().unwrap_or("HEAD").into(),
            };
            Ok(serde_json::to_value(collect(root, request)?)?)
        }
        "nwc_wake" | "nwc_check" => {
            let events: Vec<String> = arguments["events"]
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default();
            Ok(serde_json::to_value(wake(root, &events)?)?)
        }
        "nwc_resolve" => Ok(serde_json::to_value(resolve(
            root,
            arguments["id"].as_str().context("id is required")?,
            arguments["evidence"].as_str().context("evidence is required")?,
        )?)?),
        _ => bail!("unknown tool {name}"),
    }
}

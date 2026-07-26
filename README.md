<h1 align="center">Wake Me When</h1>

<p align="center"><strong>Wake future work exactly when it becomes possible.</strong></p>

Coding agents routinely finish a task with polished future work: phase two,
remove a fallback after migration, adopt the official API when it ships. That
prose usually disappears with the session.

Wake Me When automatically collects only **evidence-backed conditional deferments**,
wakes them when their machine-checkable cue becomes true, and fails completion
while due work remains unresolved.

```text
Completed work contains a proven deferment -> wmw collect
A task or external event begins            -> wmw wake
The deferred action is completed           -> wmw resolve
Work is about to finish                    -> wmw check
```

There is deliberately no `wmw add`. Capture and delivery belong to the
harness, not to an agent remembering to maintain another memory tool.

| Property | Contract |
| --- | --- |
| One durable concept | A deferment is a concrete future action, its current blocker, an observable cue, scope, and verbatim evidence. |
| Evidence bounded | Two isolated passes must return the same candidate; every evidence fragment must exist in the task envelope. |
| Deterministic waking | Events, path presence/absence, and literal file-content cues require no model. |
| Repository native | Readable TOML files are versioned with the team. |
| Agent independent | Native CLI, portable skill, and local stdio MCP call the same core. |
| Fail closed | Invalid evidence, judge failure, malformed storage, and due unresolved work cannot become a pass. |

## Install

Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/lucasrgt/wake-me-when/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/lucasrgt/wake-me-when/main/scripts/install.ps1 | iex
```

From source:

```bash
cargo install --git https://github.com/lucasrgt/wake-me-when --locked
```

## Quick start

```bash
wmw init --agent-file AGENTS.md --agent-file CLAUDE.md
```

Repositories initialized by Not Yet 0.1 migrate automatically on the first
`wmw init`: `.notyet` becomes `.wmw`, existing deferments are preserved, and
managed agent instructions are replaced in place. MCP hosts must update tool
names from `notyet_*` to `wmw_*`.

At task completion, a harness supplies the task artifacts. Codex can write its
last response with `--output-last-message`; other hosts can pass equivalent
content through MCP.

```bash
wmw collect \
  --task "Migrate customers while mobile v1 remains active" \
  --plan ROADMAP.md \
  --final-message .agent-last.md
```

An accepted deferment looks like:

```toml
schema = 1
id = "01k..."
title = "Remove the LegacyName dual-write"
action = "Remove the old DTO field, write path, and database column."
blocker = "Mobile v1 still reads LegacyName."
scopes = ["src/customers/**"]
evidence = [
  "mobile v1 still reads LegacyName",
  "customer.LegacyName = input.Name"
]

[cue]
kind = "event"
path = ""
value = "mobile-v1-retired"
```

When the host observes the event:

```bash
wmw wake --event mobile-v1-retired
```

After doing the work:

```bash
wmw resolve --id 01k... --evidence "commit abc123 and AVP customer-write verdict"
wmw check --event mobile-v1-retired
```

`check` exits `1` while any supplied event or current repository state makes an
active deferment due. Protocol or storage failures exit `2`.

## Cue kinds

| Kind | Becomes due when |
| --- | --- |
| `event` | The host supplies the exact stable event name |
| `path_exists` | A repository-relative path exists |
| `path_absent` | A repository-relative path no longer exists |
| `file_contains` | A repository file contains the literal value |
| `file_not_contains` | An existing repository file no longer contains the literal value |

The collector may not invent a path or event. A vague statement such as
“improve this later” is rejected. Work left incomplete inside the current task
is also rejected: Wake Me When must never launder incomplete work into a roadmap.

## Automatic collection

The collection envelope contains only:

- current task;
- selected plan or roadmap text;
- final agent response;
- Git diff, including untracked text files;
- first-pass candidates during confirmation.

`.wmw/**` is always excluded from the diff so local judge files and prior
memory can never prove their own claims. The first pass extracts candidates.
The second isolated pass receives only those candidates and the same envelope.
Only structurally identical candidates survive. Evidence strings are then
checked locally as verbatim substrings before TOML is written.

## Wake Me When, NYA, RTW, and AVP

The projects remain independent:

| Project | Durable question |
| --- | --- |
| Wake Me When | Which proven future action is due now? |
| [Right This Way](https://github.com/lucasrgt/right-this-way) | How does this repository already implement this correctly? |
| [Not You Again](https://github.com/lucasrgt/not-you-again) | Which corrected failure must not recur? |
| [AVP](https://github.com/lucasrgt/acceptance-verification-protocol) | Which acceptance behavior must hold? |

An AVP verdict can arrive as a Wake Me When event, waking the next roadmap phase.
RTW then supplies the positive precedent, NYA supplies relevant scars, and AVP
proves the new phase.

## Genesis

The pre-product harness froze eight backend, frontend, UI, and roadmap
envelopes with four positive and four negative controls. The valid Codex run
achieved 100% precision, recall, and exact-cue accuracy. Two earlier attempts
remain versioned because they exposed invalid benchmark evidence and an expired
Claude OAuth session. This proves bounded extraction feasibility, not future
execution. See [`genesis/`](genesis/).

## Scope

Wake Me When does not predict the future, schedule jobs, execute actions, manage a
backlog, accept generic TODOs, or provide open-ended review. It is an `if`
statement that survives sessions:

```text
if observable_cue_is_true:
    deliver_evidence_backed_deferment
```

## Benchmarks

The published v0.1.0 suite, recorded under the former Not Yet name, keeps
capture, delivery, and scale separate:

- the paired agent run turned 5/5 observed baseline misses into passing Wake Me When
  arms, with every wake, resolve, and check evidenced;
- the 1,024- and 10,000-deferment stress runs recovered every target, exercised
  all five cue kinds, and passed corruption and collection-bound checks.

Raw events, diffs, summaries, limitations, and reproduction commands live in
[`benchmarks/`](benchmarks/).

## Build

```bash
cargo install cargo-llvm-cov tokei --locked
cargo xtask verify
```

The canonical gate enforces formatting, Clippy, at most 500 production Rust
lines, the complete test suite, at least 95% line coverage, and a packaged
entrypoint smoke. CI and release call this same command.

## License

Wake Me When is available under the [MIT License](LICENSE).

# Not Yet Architecture

## Purpose

Not Yet is repository-local prospective memory for coding agents. It converts
conditional deferments found at task completion into versioned obligations and
delivers them when a deterministic cue becomes true.

Capture and delivery are harness properties. An agent is never expected to
remember to add or retrieve future work voluntarily.

## Public model

The system contains one durable concept:

> A deferment is a concrete future action intentionally blocked by a currently
> false prerequisite, bound to a machine-checkable cue and verbatim evidence.

There are four operations:

```text
Completed work is inspected -> notyet collect
Context or an event arrives  -> notyet wake
The action is completed      -> notyet resolve
Work is finishing            -> notyet check
```

`init` installs repository assets. `mcp` exposes the same four operations.
There is no manual add operation.

## Storage

```text
.notyet/
├── config.local.toml
├── config.toml
├── SKILL.md
└── deferments/
    └── <ulid>.toml
```

TOML is versioned and authoritative. Local configuration selects an isolated
judge command and is ignored by Git. The project configuration is an optional
team override; user configuration is the final fallback.

## Collection

The harness supplies a bounded envelope: task, selected plan, final response,
and Git diff. Internal `.notyet/**` files are excluded before the envelope is
formed.

The first judge pass proposes at most 20 deferments. Local validation requires:

1. non-empty action, blocker, and title;
2. reusable valid glob scopes;
3. at least two verbatim evidence fragments present in the envelope;
4. one well-formed deterministic cue;
5. repository-confined paths.

A second isolated pass sees the same envelope and proposed candidates. Only
candidates returned identically in both passes are stored. Existing active
action/blocker/cue triples are deduplicated.

## Waking

Waking is model-free. The host supplies observed event names; path and file
cues are evaluated against the current repository. `file_not_contains` requires
the file to exist so a missing or unreadable file cannot produce a vacuous
green.

`wake` reports due work. `check` returns the same evidence and exits nonzero
while any deferment is due. `resolve` retains the original record and adds
resolution time and evidence.

## Surfaces

CLI and local stdio MCP call the same Rust functions. The portable skill and
managed agent-instruction block define when hosts must invoke those surfaces.
Judge execution is a resolved subprocess reading one prompt on stdin and
returning strict JSON on stdout.

## Product boundaries

Not Yet is not a scheduler, issue tracker, roadmap generator, semantic reminder
store, or autonomous executor. It does not infer unspecified future events and
does not turn unfinished current scope into a deferment.

NYA owns corrected failure memory. RTW owns proven implementation patterns. AVP
owns executable acceptance. Not Yet owns only evidence-backed conditional
future work.

## Engineering constitution

```text
Production code:         <= 500 LOC
Shared runtime coverage: >= 95%
Packaged entrypoint:     end-to-end smoke tested
```

`cargo xtask verify` is the canonical local, CI, and release gate.

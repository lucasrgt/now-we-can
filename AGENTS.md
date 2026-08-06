# Now We Can Engineering Guide

All repository artifacts must be written in English.

## Product contract

Now We Can exposes one public concept, the deferment, and four operations:

1. `nwc collect`
2. `nwc wake`
3. `nwc resolve`
4. `nwc check`

`nwc init` installs assets. `nwc mcp` exposes the same operations.

Do not introduce manual add, generic memories, TODOs, reminders, schedules,
semantic cues, candidate states, policies, or autonomous execution.

## Engineering constitution

1. Production code under `src/` must remain at or below 500 code lines as
   measured by `tokei`.
2. Shared runtime line coverage must remain at or above 95 percent without
   rounding. The process entrypoint is verified end to end.
3. Test code is unlimited and must live under `tests/`.
4. Production behavior may not move into scripts, generated files,
   integrations, or test helpers to evade the line budget.
5. Git TOML files are the durable source of truth.
6. CLI and MCP must call the same core operations.
7. Judge, evidence, storage, and protocol failures must fail closed.
8. `.nwc/**` must never enter its own collection envelope.

## Change discipline

Prefer the smallest complete implementation. Add a dependency or abstraction
only when it removes more maintained behavior than it introduces.

Before reporting implementation complete, run `cargo xtask verify`. It owns
formatting, Clippy, the production line budget, tests, and coverage.

<!-- nwc:instructions:start -->
## Now We Can

This repository uses Now We Can (`nwc`) to turn evidence-backed conditional
deferments into obligations that reappear when their machine-checkable cue is
observed.

1. At task start and after context changes, run `nwc wake` with every event
   supplied by the host. Treat returned deferments as due work, not suggestions.
2. Resolve a completed deferment with `nwc resolve --id <id> --evidence "<proof>"`.
3. At task completion, the host must run `nwc collect` over the task, plan,
   final response, and diff. Agents must not add deferments manually.
4. Collection may preserve only a concrete action intentionally blocked by a
   currently false prerequisite and bound to an event, path, or file-content
   cue evidenced in the supplied task artifacts.
5. Aspirations, optional improvements, unfinished current scope, permanent
   behavior, vague future work, and invented cues are not deferments.
6. Run `nwc check` before completion with the same observed events. Exit code
   1 means a due deferment remains unresolved. Exit code 2 means the check did
   not complete and must never be reported as a pass.

Tests and roadmaps do not replace Now We Can. The host owns collection and
delivery; neither may depend on an agent voluntarily remembering the tool.
<!-- nwc:instructions:end -->


## Optional Prime Agent adapter

`integrations/prime-agent` is a thin optional host adapter. It may invoke only
the `nwc` CLI with literal argv and must never parse semantic records or
reimplement Rust behavior. It activates only for `.nwc/SKILL.md` and
must remain completely inactive when the Git root contains `csm.toml`. CSM has
absolute Prime-integration precedence.

When changing the adapter, run `npm ci`, `npm test`, `npm run typecheck`, and
`npm pack --dry-run` from `integrations/prime-agent` in addition to
`cargo xtask verify`.

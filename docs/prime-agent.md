# Prime Agent integration

The optional package at `integrations/prime-agent` wraps the standalone `nwc`
CLI without reading `.nwc` records or reproducing Rust semantics.

Install it after placing `nwc` on `PATH`:

```bash
prime-agent package install /absolute/path/to/now-we-can/integrations/prime-agent
```

Run `/reload` in an active Prime session. The adapter activates only when the Git
root contains `.nwc/SKILL.md`. A root `csm.toml` always suppresses it;
CSM then owns Prime retrieval and checks while the standalone CLI stays usable.

The adapter exposes `/nwc status`, `/nwc wake`, explicit `/nwc check`,
and a session-only `/nwc auto wake on|off` toggle. Automatic
`wake` defaults to on and can be disabled at launch with
`--nwc-auto-wake off`. It never exposes repository adoption or semantic
record mutation commands.

All subprocesses use literal argv, the Git root as cwd, cancellation, a timeout,
and a 64 KiB UTF-8 output cap. Nonzero exits, killed processes, and truncation
remain explicit. Injected output is delimited as repository knowledge rather
than higher-priority instructions.

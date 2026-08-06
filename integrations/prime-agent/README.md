# Now We Can for Prime Agent

This optional capability package is a thin adapter around the standalone `nwc`
Rust CLI. It adds bounded automatic `wake`, explicit operator commands, and a
conditional model skill without reading semantic records or reimplementing
Now We Can behavior.

## Install

Install `nwc` on `PATH`, then run:

```bash
prime-agent package install /absolute/path/to/now-we-can/integrations/prime-agent
```

Use `/reload` in a live Prime session. Set `NWC_BIN` or pass
`--nwc-bin /absolute/path/to/nwc` when needed.

## Activation and precedence

The package activates only when the Git root contains `.nwc/SKILL.md`. It is
fully suppressed when `<git-root>/csm.toml` exists, even if the standalone marker
also remains. CSM then owns Prime retrieval and verification; direct standalone
CLI use remains available. In inactive repositories the package invokes no
`nwc` process, exposes no command or skill, and paints no status.

## Surface

- ``/nwc wake [EVENT...]` and `/nwc check [EVENT...]``
- `/nwc status`
- `/nwc auto wake on|off`

Automatic `wake` is enabled by default and can be disabled at launch with
`--nwc-auto-wake off`. Checks are always explicit. The adapter exposes no
repository adoption or semantic-record mutation command.

Every process uses a literal argv array, the resolved Git root as cwd, a
configurable timeout, cancellation, control-sequence sanitization, and a 64 KiB
UTF-8 output cap. Nonzero exits, cancellation, and truncation remain visible.
Repository output is delimited as lower-priority project knowledge.

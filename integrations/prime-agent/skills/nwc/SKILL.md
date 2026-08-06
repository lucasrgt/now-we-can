---
name: nwc
description: Use standalone Now We Can repository knowledge before editing and its explicit semantic gate before completion.
---

# Now We Can

This skill is available only because the Git root contains `.nwc/SKILL.md`
and does not contain `csm.toml`. If CSM is adopted, use only the CSM integration;
do not invoke the standalone adapter and duplicate retrieval or checks.

At task start, retrieve due deferments using every stable event actually observed by the host; invent none:

```bash
"${NWC_BIN:-nwc}" wake --event <stable-event>
```

With no external event, `wake` still evaluates repository path and file-content cues. The Prime extension automatically performs that empty-event retrieval when auto-wake is enabled. Rerun with explicit events when the host observes them.

Before completion, run:

```bash
"${NWC_BIN:-nwc}" check --event <stable-event>
```

Exit code 1 means repository findings remain; fix or report them and rerun. Exit
code 2 or a killed, failed, or truncated provider means the operation did not
complete and must never be reported as a pass.

Never run `nwc init`, `nwc collect`, or `nwc resolve` unless the user explicitly requests that operation and provides its required evidence.

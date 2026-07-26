# Wake Me When genesis harness

This harness tests the product's first claim before the product exists:

> A bounded judge can identify evidence-backed conditional deferments in a
> completed agent task without turning aspirations, unfinished work, or
> permanent behavior into future obligations.

The cases contain backend, frontend, UI, and roadmap envelopes. Expected labels
remain in `cases.json` for auditability, but the runner removes them before
calling each provider. Providers run without repository tools in a temporary
directory so they cannot read the hidden labels.

```powershell
python genesis/mini_harness.py `
  --provider codex --provider claude `
  --output genesis/results/v0-genesis
```

The harness reports binary precision/recall, exact cue accuracy, raw provider
output, and every false positive or false negative. It is a feasibility probe,
not a product benchmark and not evidence that an agent will execute a deferred
intention later.

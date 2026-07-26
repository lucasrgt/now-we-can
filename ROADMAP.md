# Roadmap

The v0.1 product scope is complete. One external validation remains
intentionally deferred:

- Action: rerun the frozen genesis cases with Claude and publish the paired
  provider comparison.
- Current blocker: the local Claude CLI reports that its OAuth session expired
  and cannot refresh non-interactively.
- Observable cue: event `claude-auth-restored`.
- Scope: `genesis/**`.
- Evidence: the two preserved invalid attempts document why they do not count;
  the valid Codex arm remains independently reproducible.

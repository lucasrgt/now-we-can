# Invalid genesis attempt 2

This attempt is preserved but does not count as evidence.

- The Claude-specific schema compatibility issue was fixed.
- Claude then exited before inference because its OAuth session had expired
  and could not be refreshed non-interactively.
- The Codex output is retained only as raw evidence; the paired run was
  incomplete.

There is no aggregate report because the original runner stopped at the first
provider failure.

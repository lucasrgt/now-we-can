# Invalid genesis attempt 1

This attempt is preserved but does not count as evidence.

- Codex returned 100% precision but only 50% recall.
- Claude rejected the JSON Schema because its CLI did not accept the
  `$schema` draft declaration.
- The input envelopes did not contain enough explicit path and event evidence
  for two expected positives. The cases were corrected instead of weakening
  the evidence rule.

There is no aggregate report because the original runner stopped at the first
provider failure.

# Not Yet Paired Agent Benchmark

Run from `2026-07-26T19:09:35.663315+00:00` to `2026-07-26T19:12:27.035900+00:00` with `codex-cli 0.144.0` on `Linux-6.18.33.1-microsoft-standard-WSL2-x86_64-with-glibc2.36`.

| Case | Baseline | Not Yet | Wake | Resolve | Check | Paired improvement |
| --- | --- | --- | --- | --- | --- | --- |
| `backend-dual-write` | deferment_missed | pass | yes | yes | pass | yes |
| `frontend-contract-fallback` | deferment_missed | pass | yes | yes | pass | yes |
| `ui-warning-token` | deferment_missed | pass | yes | yes | pass | yes |
| `roadmap-pagination` | deferment_missed | pass | yes | yes | pass | yes |
| `retired-feature-flag` | deferment_missed | pass | yes | yes | pass | yes |

Baseline deferments missed: **5**.

Not Yet deferments missed: **0**.

Paired improvements: **5 of 5 observed baseline misses**.

Regressions against passing baselines: **0**.

Overall protocol result: **PASS**.

A paired improvement counts only when the baseline completes the requested task but misses the previously captured obligation, while the Not Yet arm completes both. Baseline passes are ties, never attributed preventions.

The corpus is synthetic and the deferments are disclosed pre-captured fixtures. The genesis harness measures capture separately; this benchmark isolates deterministic wake-up and agent execution.

## Scenario sources

| Case | Domain | Primary source |
| --- | --- | --- |
| `backend-dual-write` | backend | https://martinfowler.com/articles/evodb.html |
| `frontend-contract-fallback` | frontend | https://www.typescriptlang.org/docs/handbook/2/everyday-types.html |
| `ui-warning-token` | ui | https://www.w3.org/WAI/ARIA/apg/practices/names-and-descriptions/ |
| `roadmap-pagination` | roadmap | https://www.rfc-editor.org/rfc/rfc9110 |
| `retired-feature-flag` | frontend | https://martinfowler.com/articles/feature-toggles.html |

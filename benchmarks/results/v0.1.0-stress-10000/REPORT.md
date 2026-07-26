# Not Yet Large-Corpus Stress Benchmark

Run from `2026-07-26T19:13:09.475065+00:00` to `2026-07-26T19:16:00.941646+00:00` on `Windows-11-10.0.26200-SP0`.

| Metric | Result |
| --- | ---: |
| Versioned deferments | 10000 |
| Event probes exact | 64 / 64 |
| Unrelated events empty | 8 / 8 |
| Deterministic cue kinds exact | 5 / 5 |
| Resolved obligation excluded | yes |
| First cold wake | 80.937 s |
| Warm wake p50 | 0.703 s |
| Warm wake p95 | 0.797 s |
| Warm wake maximum | 0.875 s |
| Sleeping check exit | 0 |
| Due check exit | 1 |
| Corrupt store exit | 2 |
| Collector maximum accepted | 20 / 20 |
| Duplicate candidates suppressed | 20 / 20 |
| 21-candidate overflow exit | 2 |

Overall protocol result: **PASS**.

Latency includes process startup and a full scan of the versioned TOML corpus. The first cold probe is reported separately; warm figures exclude only that probe. Timings are not a universal service-level guarantee.

# Not Yet benchmarks

The suite tests three different claims without blending their metrics:

1. the genesis harness tests whether a bounded model can capture conditional
   deferments from completed work;
2. the paired benchmark tests whether deterministic wake-up changes what a
   fresh coding agent completes;
3. the stress benchmark tests storage, cue evaluation, fail-closed behavior,
   and collection bounds at large synthetic scale.

## Published v0.1.0 results

| Run | Result | Evidence |
| --- | --- | --- |
| Genesis capture | PASS | 4/4 positives, 4/4 negatives, and 4/4 exact cues with Codex |
| [Paired agent](results/v0.1.0-paired-gpt-5.3-codex-spark/REPORT.md) | PASS | 5/5 observed baseline misses became passing Not Yet arms |
| [1,024-deferment stress](results/v0.1.0-stress-1024/REPORT.md) | PASS | 128/128 exact event probes and all five cue kinds |
| [10,000-deferment stress](results/v0.1.0-stress-10000/REPORT.md) | PASS | 64/64 exact event probes and all fail-closed checks |

The 10,000-item Windows run measured a cold first wake of 80.937 seconds,
followed by 0.703 seconds p50, 0.797 seconds p95, and 0.875 seconds maximum for
warm wakes. The cold result remains visible rather than being discarded. Not
Yet deliberately keeps TOML as the only store in v0.1; an index is not
justified until normal repositories demonstrate this scale.

## Paired agent protocol

For each case, the protocol creates two repositories from the same completed
earlier work:

| Arm | Additional state |
| --- | --- |
| Baseline | Ordinary repository instructions only |
| Not Yet | The same seed plus one disclosed pre-captured deferment and managed instructions |

Both arms receive the same immediate task, model, limits, and randomized
ordering. In the Not Yet arm, the harness runs `notyet wake`, injects exactly
the due obligation, and runs `notyet check` after the agent. A deterministic
evaluator outside both repositories inspects the requested change and the
deferred change.

A paired improvement counts only when the baseline completes the immediate
task but misses the old obligation and the Not Yet arm completes both. A
baseline pass is a tie, never an attributed prevention. The protocol also
requires every Not Yet arm to wake exactly one item, execute `notyet resolve`,
pass the external evaluator, and finish with a zero-exit `notyet check`.

The five cases cover backend dual-write retirement, a frontend generated
contract fallback, a UI design token, a roadmap phase, and a retired feature
flag. Their declared sources live in [`catalog.json`](catalog.json).

The Codex CLI runs inside a fresh Docker container with an
authentication-only home, workspace-write sandboxing, no host source checkout,
and no Docker socket. Every result retains agent events, stderr, final text,
the exact diff, wake output, and check output.

### Reproduce

Build a Linux binary and the benchmark image, then run:

```bash
docker build \
  --tag notyet-benchmark:local \
  --file benchmarks/Dockerfile \
  benchmarks

mkdir -p benchmarks/results/local-paired

docker run --rm \
  --security-opt seccomp=unconfined \
  --mount type=bind,src="$HOME/.codex/auth.json",dst=/seed/auth.json,readonly \
  --mount type=bind,src="$(pwd)/benchmarks/paired.py",dst=/benchmarks/paired.py,readonly \
  --mount type=bind,src="$(pwd)/target/x86_64-unknown-linux-gnu/release/notyet",dst=/usr/local/bin/notyet,readonly \
  --mount type=bind,src="$(pwd)/benchmarks/results/local-paired",dst=/output \
  --workdir /work \
  notyet-benchmark:local \
  python3 /benchmarks/paired.py \
  --notyet /usr/local/bin/notyet \
  --output /output \
  --model gpt-5.3-codex-spark \
  --work-parent /work
```

The model is an explicit benchmark input, not a Not Yet dependency.

## Large-corpus stress protocol

The stress runner creates versioned TOML deferments with all five cue kinds.
Every non-target cue starts false. Positive event probes must return exactly
one expected id; unrelated events must return none. Separate state transitions
exercise `path_exists`, `path_absent`, `file_contains`, and
`file_not_contains`.

It also verifies:

- resolved deferments never wake;
- `check` exits 0 while sleeping and 1 when due;
- corrupt TOML exits 2 instead of becoming a silent pass;
- the collector accepts 20 candidates, suppresses the same 20 as duplicates,
  and rejects 21 candidates.

```bash
python3 benchmarks/stress.py \
  --notyet target/release/notyet \
  --count 1024 \
  --probes 128 \
  --output benchmarks/results/local-stress-1024

python3 benchmarks/stress.py \
  --notyet target/release/notyet \
  --count 10000 \
  --probes 64 \
  --output benchmarks/results/local-stress-10000
```

## Limits

The repositories and large corpus are synthetic. The paired result is one
model, one seed, and five tasks; it is evidence for this protocol, not a
universal agent-success rate. Pre-captured fixtures intentionally isolate
delivery from capture, which is measured separately by the genesis harness.
Timing depends heavily on filesystem cache and host security software.

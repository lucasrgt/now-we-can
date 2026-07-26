#!/usr/bin/env python3
"""Deterministic large-corpus stress benchmark for Wake Me When."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path


KINDS = ("event", "path_exists", "path_absent", "file_contains", "file_not_contains")


def run(command, cwd: Path, timeout=300, check=True):
    result = subprocess.run(
        [str(item) for item in command],
        cwd=cwd,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        timeout=timeout,
    )
    if check and result.returncode:
        raise RuntimeError(
            f"{' '.join(map(str, command))} failed with {result.returncode}\n"
            f"{result.stdout}\n{result.stderr}"
        )
    return result


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, math.ceil(len(ordered) * fraction) - 1)]


def identifier(index):
    return f"wmw-stress-{index:05d}"


def cue(index):
    kind = KINDS[index % len(KINDS)]
    path = f"signals/{kind}-{index:05d}.txt" if kind != "event" else ""
    value = (
        f"event-{index:05d}"
        if kind == "event"
        else f"needle-{index:05d}"
        if kind.startswith("file_")
        else ""
    )
    return kind, path, value


def deferment_text(index, commit, resolved=False):
    kind, path, value = cue(index)
    lines = [
        "schema = 1",
        f"id = {json.dumps(identifier(index))}",
        f"title = {json.dumps(f'Complete deferred action {index:05d}')}",
        f"action = {json.dumps(f'Remove temporary mechanism {index:05d}')}",
        f"blocker = {json.dumps(f'Prerequisite {index:05d} is currently false')}",
        f"scopes = [{json.dumps(f'surface/{index % 64:02d}/**')}]",
        'evidence = ["stress evidence one", "stress evidence two"]',
        'recorded_at = "2026-01-01T00:00:00Z"',
        'recorded_by = "Wake Me When benchmark"',
        f"recorded_commit = {json.dumps(commit)}",
    ]
    if resolved:
        lines += [
            'resolved_at = "2026-01-02T00:00:00Z"',
            'resolution_evidence = "benchmark resolution proof"',
        ]
    lines += [
        "",
        "[cue]",
        f"kind = {json.dumps(kind)}",
        f"path = {json.dumps(path)}",
        f"value = {json.dumps(value)}",
        "",
    ]
    return "\n".join(lines)


def initialize(root: Path, binary: Path, count: int):
    root.mkdir(parents=True, exist_ok=True)
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "Wake Me When Benchmark"], root)
    run(["git", "config", "user.email", "benchmark@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)
    (root / "README.md").write_text("# Stress fixture\n", encoding="utf-8", newline="\n")
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "seed fixture"], root)
    commit = run(["git", "rev-parse", "HEAD"], root).stdout.strip()
    directory = root / ".wmw" / "deferments"
    directory.mkdir(parents=True)
    signals = root / "signals"
    signals.mkdir()
    for index in range(count):
        kind, path, value = cue(index)
        target = root / path if path else None
        if kind == "path_absent":
            target.write_text("blocker remains\n", encoding="utf-8", newline="\n")
        elif kind == "file_contains":
            target.write_text("token not shipped\n", encoding="utf-8", newline="\n")
        elif kind == "file_not_contains":
            target.write_text(value + "\n", encoding="utf-8", newline="\n")
        (directory / f"{identifier(index)}.toml").write_text(
            deferment_text(index, commit, resolved=index == count - 1),
            encoding="utf-8",
            newline="\n",
        )
    run(["git", "add", "."], root, timeout=600)
    run(["git", "commit", "-qm", f"seed {count} versioned deferments"], root, timeout=600)
    version = run([binary, "--version"], root).stdout.strip()
    return commit, version


def wake(binary: Path, root: Path, events=()):
    command = [binary, "wake", "--json"]
    for event in events:
        command += ["--event", event]
    started = time.monotonic()
    result = run(command, root)
    return json.loads(result.stdout), round(time.monotonic() - started, 6)


def event_probes(binary: Path, root: Path, count: int, probes: int, output: Path):
    candidates = [index for index in range(0, count - 1) if cue(index)[0] == "event"]
    selected = [candidates[index * len(candidates) // probes] for index in range(probes)]
    results = []
    with (output / "wake-probes.jsonl").open("w", encoding="utf-8", newline="\n") as stream:
        for index in selected:
            result, seconds = wake(binary, root, [cue(index)[2]])
            item = {
                "index": index,
                "expected_id": identifier(index),
                "due_ids": [value["id"] for value in result["due"]],
                "seconds": seconds,
            }
            results.append(item)
            stream.write(json.dumps(item) + "\n")
    negatives = []
    for index in range(8):
        result, seconds = wake(binary, root, [f"unrelated-{index}"])
        negatives.append({"event": f"unrelated-{index}", "due": len(result["due"]), "seconds": seconds})
    (output / "negative-probes.json").write_text(
        json.dumps(negatives, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    return results, negatives


def state_probes(binary: Path, root: Path, count: int):
    chosen = {}
    for kind in KINDS:
        chosen[kind] = next(index for index in range(count - 1) if cue(index)[0] == kind)
    outcomes = {}
    event_index = chosen["event"]
    result, seconds = wake(binary, root, [cue(event_index)[2]])
    outcomes["event"] = {"due_ids": [item["id"] for item in result["due"]], "seconds": seconds}
    for kind in KINDS[1:]:
        index = chosen[kind]
        _, path, value = cue(index)
        target = root / path
        original = target.read_text(encoding="utf-8") if target.exists() else None
        if kind == "path_exists":
            target.write_text("prerequisite arrived\n", encoding="utf-8", newline="\n")
        elif kind == "path_absent":
            target.unlink()
        elif kind == "file_contains":
            target.write_text(value + "\n", encoding="utf-8", newline="\n")
        else:
            target.write_text("legacy token removed\n", encoding="utf-8", newline="\n")
        result, seconds = wake(binary, root)
        outcomes[kind] = {"due_ids": [item["id"] for item in result["due"]], "seconds": seconds}
        if original is None:
            target.unlink()
        else:
            target.write_text(original, encoding="utf-8", newline="\n")
    resolved_index = count - 1
    kind, _, value = cue(resolved_index)
    events = [value] if kind == "event" else []
    result, seconds = wake(binary, root, events)
    outcomes["resolved"] = {
        "id": identifier(resolved_index),
        "excluded": identifier(resolved_index) not in [item["id"] for item in result["due"]],
        "seconds": seconds,
    }
    return chosen, outcomes


def fail_closed_probes(binary: Path, root: Path, output: Path, event: str):
    sleeping = run([binary, "check", "--json"], root, check=False)
    due = run([binary, "check", "--event", event, "--json"], root, check=False)
    corrupt = root / ".wmw/deferments/corrupt.toml"
    corrupt.write_text("not = [valid", encoding="utf-8", newline="\n")
    broken = run([binary, "wake", "--json"], root, check=False)
    corrupt.unlink()
    result = {
        "sleeping_check_exit": sleeping.returncode,
        "due_check_exit": due.returncode,
        "corrupt_store_exit": broken.returncode,
        "corrupt_store_stderr": broken.stderr.strip(),
    }
    (output / "fail-closed.json").write_text(
        json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    return result


def collection_probe(binary: Path, root: Path, output: Path):
    task = "Complete the stress fixture migration"
    evidence = "temporary compatibility remains"
    source = root / "src"
    source.mkdir(exist_ok=True)
    (source / "change.txt").write_text(evidence + "\n", encoding="utf-8", newline="\n")
    candidates = [
        {
            "title": f"Remove stress compatibility {index:02d}",
            "action": f"Remove temporary stress mechanism {index:02d}",
            "blocker": f"Stress prerequisite {index:02d} is false",
            "cue": {"kind": "event", "path": "", "value": f"collect-ready-{index:02d}"},
            "scopes": ["src/**"],
            "evidence": [task, evidence],
        }
        for index in range(21)
    ]
    helper = root / ".wmw/judge.py"
    helper.write_text(
        "import json,sys\nprint(json.dumps({'deferments':json.load(open(sys.argv[1],encoding='utf-8'))}))\n",
        encoding="utf-8",
        newline="\n",
    )
    data = root / ".wmw/judge.json"
    command = json.dumps([sys.executable, str(helper), str(data)])
    (root / ".wmw/config.local.toml").write_text(
        f"schema = 1\n[judge]\ncommand = {command}\n", encoding="utf-8", newline="\n"
    )

    def collect(items):
        data.write_text(json.dumps(items), encoding="utf-8", newline="\n")
        return run([binary, "collect", "--task", task, "--json"], root, check=False)

    first = collect(candidates[:20])
    duplicate = collect(candidates[:20])
    overflow = collect(candidates)
    (output / "collect-first.json").write_text(first.stdout, encoding="utf-8", newline="\n")
    result = {
        "first_exit": first.returncode,
        "first": json.loads(first.stdout) if first.returncode == 0 else {},
        "duplicate_exit": duplicate.returncode,
        "duplicate": json.loads(duplicate.stdout) if duplicate.returncode == 0 else {},
        "overflow_exit": overflow.returncode,
        "overflow_stderr": overflow.stderr.strip(),
    }
    (output / "collection-probe.json").write_text(
        json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    return result


def render(summary):
    wake_data = summary["wake"]
    lines = [
        "# Wake Me When Large-Corpus Stress Benchmark",
        "",
        f"Run from `{summary['started_at']}` to `{summary['completed_at']}` on `{summary['platform']}`.",
        "",
        "| Metric | Result |",
        "| --- | ---: |",
        f"| Versioned deferments | {summary['corpus_size']} |",
        f"| Event probes exact | {wake_data['exact']} / {wake_data['probes']} |",
        f"| Unrelated events empty | {wake_data['negative_empty']} / {wake_data['negative_probes']} |",
        f"| Deterministic cue kinds exact | {wake_data['cue_kinds_exact']} / 5 |",
        f"| Resolved obligation excluded | {'yes' if wake_data['resolved_excluded'] else 'no'} |",
        f"| First cold wake | {wake_data['cold_first_seconds']} s |",
        f"| Warm wake p50 | {wake_data['warm_latency_seconds']['p50']} s |",
        f"| Warm wake p95 | {wake_data['warm_latency_seconds']['p95']} s |",
        f"| Warm wake maximum | {wake_data['warm_latency_seconds']['max']} s |",
        f"| Sleeping check exit | {summary['fail_closed']['sleeping_check_exit']} |",
        f"| Due check exit | {summary['fail_closed']['due_check_exit']} |",
        f"| Corrupt store exit | {summary['fail_closed']['corrupt_store_exit']} |",
        f"| Collector maximum accepted | {len(summary['collection']['first'].get('recorded', []))} / 20 |",
        f"| Duplicate candidates suppressed | {summary['collection']['duplicate'].get('duplicates', 0)} / 20 |",
        f"| 21-candidate overflow exit | {summary['collection']['overflow_exit']} |",
        "",
        f"Overall protocol result: **{'PASS' if summary['passed'] else 'FAIL'}**.",
        "",
        "Latency includes process startup and a full scan of the versioned TOML corpus. "
        "The first cold probe is reported separately; warm figures exclude only that "
        "probe. Timings are not a universal service-level guarantee.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--wmw", required=True, type=Path)
    parser.add_argument("--count", required=True, type=int)
    parser.add_argument("--probes", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    binary = args.wmw.resolve()
    output = args.output.resolve()
    if not binary.is_file():
        raise SystemExit(f"wmw binary not found: {binary}")
    if args.count < 32 or args.probes < 1 or args.probes > args.count // len(KINDS):
        raise SystemExit("invalid corpus or probe count")
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="wmw-stress-"))
    try:
        _, version = initialize(work, binary, args.count)
        probes, negatives = event_probes(binary, work, args.count, args.probes, output)
        chosen, states = state_probes(binary, work, args.count)
        event = cue(chosen["event"])[2]
        closed = fail_closed_probes(binary, work, output, event)
        collection = collection_probe(binary, work, output)
        latencies = [item["seconds"] for item in probes]
        warm = latencies[1:] or latencies
        exact = sum(item["due_ids"] == [item["expected_id"]] for item in probes)
        cue_exact = sum(
            states[kind]["due_ids"] == [identifier(chosen[kind])] for kind in KINDS
        )
        summary = {
            "schema": 1,
            "benchmark": "large-corpus-deterministic-deferments",
            "started_at": started_at,
            "completed_at": datetime.now(timezone.utc).isoformat(),
            "platform": platform.platform(),
            "corpus_size": args.count,
            "wmw": {
                "version": version,
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "wake": {
                "probes": args.probes,
                "exact": exact,
                "negative_probes": len(negatives),
                "negative_empty": sum(item["due"] == 0 for item in negatives),
                "cue_kinds_exact": cue_exact,
                "resolved_excluded": states["resolved"]["excluded"],
                "cold_first_seconds": latencies[0],
                "warm_latency_seconds": {
                    "p50": round(statistics.median(warm), 6),
                    "p95": round(percentile(warm, 0.95), 6),
                    "max": round(max(warm), 6),
                },
            },
            "fail_closed": closed,
            "collection": collection,
        }
        summary["passed"] = (
            exact == args.probes
            and summary["wake"]["negative_empty"] == len(negatives)
            and cue_exact == len(KINDS)
            and states["resolved"]["excluded"]
            and closed["sleeping_check_exit"] == 0
            and closed["due_check_exit"] == 1
            and closed["corrupt_store_exit"] == 2
            and collection["first_exit"] == 0
            and len(collection["first"].get("recorded", [])) == 20
            and collection["duplicate"].get("duplicates") == 20
            and collection["overflow_exit"] == 2
        )
        (output / "summary.json").write_text(
            json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
        report = render(summary)
        (output / "REPORT.md").write_text(report, encoding="utf-8", newline="\n")
        print(report)
        if not summary["passed"]:
            raise SystemExit(1)
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()

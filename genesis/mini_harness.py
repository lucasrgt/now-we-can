#!/usr/bin/env python3
"""Paired feasibility probe for automatic conditional-deferment capture."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ALLOWED_CUES = "event, path_exists, path_absent, file_contains, file_not_contains"


def prompt_for(cases: list[dict]) -> str:
    envelopes = [{"case_id": case["id"], **case["envelope"]} for case in cases]
    return f"""You are the bounded collector for Not Yet.

A deferment exists only when a completed task intentionally leaves a concrete
temporary mechanism or next action undone because a prerequisite is currently
false, and the supplied evidence names a machine-checkable cue that will make
the action due.

Accept only cue kinds: {ALLOWED_CUES}.
For file cues, cue_path and cue_value must come from the supplied evidence.
For event cues, cue_value must be a stable event name; normalize an explicitly
named AVP gate to `avp:<subject>:passed`.

Reject:
- aspirations and optional improvements;
- vague "later" language without a machine-checkable cue;
- work that belongs to the current task but was merely left incomplete;
- permanent fallbacks or product behavior;
- obligations already completed;
- anything requiring you to invent a blocker, cue, path, or action.

Return one result for every case, in input order. Empty deferments means reject.
Use only evidence in the envelope. Do not provide advice or review the code.

ENVELOPES:
{json.dumps(envelopes, indent=2)}
"""


def run_codex(prompt: str, schema: dict, work: Path) -> dict:
    schema_path, output_path = work / "schema.json", work / "codex.json"
    schema_path.write_text(json.dumps(schema), encoding="utf-8")
    command = [
        "codex", "exec", "--ignore-user-config", "--ignore-rules", "--ephemeral",
        "--skip-git-repo-check", "--sandbox", "read-only",
        "--output-schema", str(schema_path), "--output-last-message", str(output_path), "-"
    ]
    completed = subprocess.run(command, input=prompt, text=True, cwd=work, capture_output=True, timeout=600)
    if completed.returncode:
        raise RuntimeError(f"codex exited {completed.returncode}: {completed.stderr[-2000:]}")
    return json.loads(output_path.read_text(encoding="utf-8"))


def run_claude(prompt: str, schema: dict, work: Path) -> dict:
    claude_schema = {key: value for key, value in schema.items() if key != "$schema"}
    command = [
        "claude", "-p", "--safe-mode", "--no-session-persistence",
        "--permission-mode", "dontAsk", "--tools", "",
        "--output-format", "json", "--json-schema", json.dumps(claude_schema)
    ]
    completed = subprocess.run(command, input=prompt, text=True, cwd=work, capture_output=True, timeout=600)
    if completed.returncode:
        raise RuntimeError(f"claude exited {completed.returncode}: {completed.stderr[-2000:]}")
    wrapper = json.loads(completed.stdout)
    value = wrapper.get("structured_output", wrapper.get("result", wrapper))
    return json.loads(value) if isinstance(value, str) else value


def normalized(value: str) -> str:
    return value.replace("\\", "/").strip().lower()


def score(cases: list[dict], output: dict) -> dict:
    predictions = {item["case_id"]: item["deferments"] for item in output.get("results", [])}
    tp = fp = fn = exact = expected_positive = 0
    failures: list[dict] = []
    for case in cases:
        expected = case["expected"]
        predicted = predictions.get(case["id"], [])
        positive = bool(predicted)
        if expected["deferred"]:
            expected_positive += 1
            if positive:
                tp += 1
                cue_matches = any(
                    normalized(item["cue_kind"]) == normalized(expected["cue_kind"])
                    and normalized(item["cue_path"]) == normalized(expected["cue_path"])
                    and normalized(item["cue_value"]) == normalized(expected["cue_value"])
                    for item in predicted
                )
                exact += int(cue_matches)
                if not cue_matches:
                    failures.append({"case": case["id"], "kind": "cue-mismatch", "predicted": predicted})
            else:
                fn += 1
                failures.append({"case": case["id"], "kind": "false-negative"})
        elif positive:
            fp += 1
            failures.append({"case": case["id"], "kind": "false-positive", "predicted": predicted})
    precision = tp / (tp + fp) if tp + fp else 1.0
    recall = tp / (tp + fn) if tp + fn else 1.0
    return {
        "true_positives": tp, "false_positives": fp, "false_negatives": fn,
        "precision": precision, "recall": recall,
        "f1": 2 * precision * recall / (precision + recall) if precision + recall else 0.0,
        "exact_cue_accuracy": exact / expected_positive if expected_positive else 1.0,
        "failures": failures,
    }


def report(summary: dict) -> str:
    lines = ["# Not Yet genesis report", "", "This is a pre-product feasibility probe.", ""]
    for provider, error in summary.get("errors", {}).items():
        lines.extend([f"## {provider}", "", f"Provider failed: `{error}`", ""])
    for provider, result in summary["providers"].items():
        lines.extend([
            f"## {provider}", "",
            f"- Precision: {result['precision']:.2%}",
            f"- Recall: {result['recall']:.2%}",
            f"- F1: {result['f1']:.2%}",
            f"- Exact cue accuracy: {result['exact_cue_accuracy']:.2%}",
            f"- False positives: {result['false_positives']}",
            f"- False negatives: {result['false_negatives']}", "",
        ])
        if result["failures"]:
            lines.extend(["```json", json.dumps(result["failures"], indent=2), "```", ""])
    passed = bool(summary["providers"]) and not summary.get("errors") and all(
        item["precision"] >= 0.90 and item["recall"] >= 0.75 and item["exact_cue_accuracy"] >= 0.75
        for item in summary["providers"].values()
    )
    lines.extend(["## Gate", "", f"Genesis gate: **{'PASS' if passed else 'FAIL'}**", ""])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--provider", action="append", choices=["codex", "claude"], required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"output already exists: {args.output}")
    args.output.mkdir(parents=True)
    cases = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
    schema = json.loads((ROOT / "schema.json").read_text(encoding="utf-8"))
    prompt = prompt_for(cases)
    summary = {"providers": {}, "errors": {}}
    with tempfile.TemporaryDirectory(prefix="not-yet-genesis-") as directory:
        work = Path(directory)
        for provider in args.provider:
            try:
                output = run_codex(prompt, schema, work) if provider == "codex" else run_claude(prompt, schema, work)
                (args.output / f"{provider}.json").write_text(json.dumps(output, indent=2), encoding="utf-8")
                summary["providers"][provider] = score(cases, output)
            except (OSError, RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError) as error:
                message = str(error)
                summary["errors"][provider] = message
                (args.output / f"{provider}-error.txt").write_text(message + "\n", encoding="utf-8")
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    (args.output / "REPORT.md").write_text(report(summary), encoding="utf-8")
    print(report(summary))
    if summary["errors"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

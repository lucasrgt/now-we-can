#!/usr/bin/env python3
"""Paired agent benchmark for deterministic Now We Can delivery."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import random
import re
import shutil
import subprocess
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


@dataclass(frozen=True)
class Case:
    name: str
    domain: str
    task: str
    action: str
    blocker: str
    cue_kind: str
    cue_path: str
    cue_value: str
    scope: str
    source: str


CASES = (
    Case(
        "backend-dual-write",
        "backend",
        "Normalize the customer's email to lowercase before storing it.",
        "Remove the legacy_name dual-write from customer saves.",
        "Mobile v1 still depended on legacy_name.",
        "event",
        "",
        "mobile-v1-retired",
        "src/customer.py",
        "https://martinfowler.com/articles/evodb.html",
    ),
    Case(
        "frontend-contract-fallback",
        "frontend",
        "Use the user's display name as the avatar alt text.",
        "Remove the optional legacy name fallback and LegacyUser type.",
        "The generated User contract still exposed name.",
        "file_not_contains",
        "src/generated/User.ts",
        "name:",
        "src/**",
        "https://www.typescriptlang.org/docs/handbook/2/everyday-types.html",
    ),
    Case(
        "ui-warning-token",
        "ui",
        "Make the pending-payment message announce itself as live status.",
        "Replace the temporary warning color with the shipped warningColor token.",
        "The design-system warning token had not shipped.",
        "file_contains",
        "src/theme.ts",
        "color.status.warning",
        "src/**",
        "https://www.w3.org/WAI/ARIA/apg/practices/names-and-descriptions/",
    ),
    Case(
        "roadmap-pagination",
        "roadmap",
        "Include the total order count in the list response.",
        "Accept a page argument and pass its offset to fetch_orders.",
        "The list-orders AVP had not passed.",
        "event",
        "",
        "avp:list-orders:passed",
        "src/orders.py",
        "https://www.rfc-editor.org/rfc/rfc9110",
    ),
    Case(
        "retired-feature-flag",
        "frontend",
        "Track checkout_opened when the checkout renders.",
        "Remove the legacy checkout branch and render NewCheckout directly.",
        "The legacy checkout flag still existed.",
        "path_absent",
        "flags/legacy-checkout.flag",
        "",
        "src/**",
        "https://martinfowler.com/articles/feature-toggles.html",
    ),
)


def run(command, cwd: Path, env=None, timeout=480, check=True):
    result = subprocess.run(
        [str(item) for item in command],
        cwd=cwd,
        env=env,
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


def seed_files(case: Case):
    if case.name == "backend-dual-write":
        return {
            "src/customer.py": '''def save_customer(customer, display_name, email):
    customer["display_name"] = display_name
    customer["legacy_name"] = display_name  # mobile v1 compatibility
    customer["email"] = email
    return customer
''',
        }
    if case.name == "frontend-contract-fallback":
        return {
            "src/generated/User.ts": "export type User = { displayName: string; avatarUrl: string };\n",
            "src/UserCard.tsx": '''import type { User } from "./generated/User";

type LegacyUser = User & { name?: string };

export function UserCard({ user }: { user: LegacyUser }) {
  const label = user.displayName ?? user.name ?? "Unknown";
  return <img src={user.avatarUrl} />;
}
''',
        }
    if case.name == "ui-warning-token":
        return {
            "src/theme.ts": '''export const color = {
  status: { warning: "#D97706" },
};
export const warningColor = color.status.warning;
''',
            "src/PendingPayment.tsx": '''export function PendingPayment() {
  return <span style={{ color: "#D97706" }}>Payment pending</span>;
}
''',
        }
    if case.name == "roadmap-pagination":
        return {
            "src/orders.py": '''def list_orders(db):
    items = db.fetch_orders(limit=20)
    return {"items": items}
''',
        }
    return {
        "src/Checkout.tsx": '''import { LegacyCheckout } from "./LegacyCheckout";
import { NewCheckout } from "./NewCheckout";
import { track } from "./analytics";

const legacyFlag = false;

export function Checkout() {
  return legacyFlag ? <LegacyCheckout /> : <NewCheckout />;
}
''',
    }


def deferment_text(case: Case, commit: str):
    return "\n".join(
        [
            "schema = 1",
            f"id = {json.dumps('benchmark-' + case.name)}",
            f"title = {json.dumps(case.action)}",
            f"action = {json.dumps(case.action)}",
            f"blocker = {json.dumps(case.blocker)}",
            f"scopes = [{json.dumps(case.scope)}]",
            f"evidence = [{json.dumps(case.action)}, {json.dumps(case.blocker)}]",
            'recorded_at = "2026-01-01T00:00:00Z"',
            'recorded_by = "Now We Can benchmark fixture"',
            f"recorded_commit = {json.dumps(commit)}",
            "",
            "[cue]",
            f"kind = {json.dumps(case.cue_kind)}",
            f"path = {json.dumps(case.cue_path)}",
            f"value = {json.dumps(case.cue_value)}",
            "",
        ]
    )


def initialize(root: Path, case: Case, arm: str, binary: Path):
    root.mkdir(parents=True)
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "Now We Can Benchmark"], root)
    run(["git", "config", "user.email", "benchmark@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)
    (root / "AGENTS.md").write_text(
        "# Benchmark repository\n\n"
        "Implement the requested task completely with the smallest clear change. "
        "Do not commit. Do not read files outside this repository.\n",
        encoding="utf-8",
        newline="\n",
    )
    for name, contents in seed_files(case).items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8", newline="\n")
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "seed completed earlier work"], root)
    seed = run(["git", "rev-parse", "HEAD"], root).stdout.strip()
    if arm == "nwc":
        run([binary, "init", "--agent-file", "AGENTS.md"], root)
        directory = root / ".nwc/deferments"
        directory.mkdir(parents=True, exist_ok=True)
        (directory / f"benchmark-{case.name}.toml").write_text(
            deferment_text(case, seed), encoding="utf-8", newline="\n"
        )
        run(["git", "add", "."], root)
        run(["git", "commit", "-qm", "adopt captured deferment"], root)
    return run(["git", "rev-parse", "HEAD"], root).stdout.strip()


def event_args(case: Case):
    return ["--event", case.cue_value] if case.cue_kind == "event" else []


def evaluate(case: Case, root: Path):
    if case.name == "backend-dual-write":
        body = (root / "src/customer.py").read_text(encoding="utf-8")
        task_ok = bool(re.search(r'email["\]]*\s*=\s*email\.(lower|casefold)\(\)', body))
        deferred_ok = "legacy_name" not in body
        detail = "email is normalized and the retired dual-write is absent"
    elif case.name == "frontend-contract-fallback":
        body = (root / "src/UserCard.tsx").read_text(encoding="utf-8")
        task_ok = bool(re.search(r'alt\s*=\s*\{[^}]*displayName', body))
        deferred_ok = "LegacyUser" not in body and "user.name" not in body
        detail = "avatar alt uses displayName and the legacy contract fallback is absent"
    elif case.name == "ui-warning-token":
        body = (root / "src/PendingPayment.tsx").read_text(encoding="utf-8")
        task_ok = 'role="status"' in body or "aria-live" in body
        deferred_ok = ("color.status.warning" in body or "warningColor" in body) and "#D97706" not in body
        detail = "status is announced and styling uses the shipped semantic token"
    elif case.name == "roadmap-pagination":
        body = (root / "src/orders.py").read_text(encoding="utf-8")
        task_ok = bool(re.search(r'["\']total(?:_count)?["\']', body))
        deferred_ok = bool(
            re.search(r"def\s+list_orders\s*\([^)]*page", body)
            and re.search(r"fetch_orders\s*\([^)]*offset", body)
        )
        detail = "response includes total and list retrieval accepts a page-derived offset"
    else:
        body = (root / "src/Checkout.tsx").read_text(encoding="utf-8")
        task_ok = "checkout_opened" in body and "track" in body
        deferred_ok = "LegacyCheckout" not in body and "legacyFlag" not in body
        detail = "checkout is tracked and renders the new implementation directly"
    outcome = "pass" if task_ok and deferred_ok else "deferment_missed" if task_ok else "incomplete"
    return {
        "outcome": outcome,
        "task_ok": bool(task_ok),
        "deferred_ok": bool(deferred_ok),
        "detail": detail,
    }


def command_observed(events: str, fragment: str):
    return fragment.lower() in events.lower()


def execute(case, arm, root, output, binary, codex, model, codex_home):
    baseline = initialize(root, case, arm, binary)
    trusted = str(root.resolve()).replace("\\", "/")
    if "'" in trusted:
        raise RuntimeError("benchmark path cannot be represented as a trusted project")
    with (codex_home / "config.toml").open("a", encoding="utf-8", newline="\n") as config:
        config.write(f"\n[projects.'{trusted}']\ntrust_level = \"trusted\"\n")
    due = None
    wake_seconds = None
    if arm == "nwc":
        started = time.monotonic()
        awakened = run([binary, "wake", *event_args(case), "--json"], root)
        wake_seconds = round(time.monotonic() - started, 3)
        due = json.loads(awakened.stdout)
        if len(due["due"]) != 1:
            raise RuntimeError(f"{case.name} expected one due deferment: {awakened.stdout}")
        (output / f"{case.name}-nwc-wake.json").write_text(
            awakened.stdout, encoding="utf-8", newline="\n"
        )
    prompt = (
        "Implement the following task in this repository. Make the smallest complete "
        "change, follow repository instructions, inspect existing code, and stop when "
        f"ready to commit. Do not ask questions.\n\nTask: {case.task}"
    )
    if due:
        prompt += (
            "\n\nAUTOMATIC NOT YET HARNESS CONTEXT\n"
            "The deterministic cue below just became true. The due deferment is an "
            "obligation in this task. Complete it, then run `nwc resolve --id <id> "
            '--evidence "<proof>"` before finishing.\n'
            + json.dumps(due["due"], indent=2)
        )
    last = output / f"{case.name}-{arm}-last.md"
    env = os.environ.copy()
    env["PATH"] = str(binary.parent) + os.pathsep + env.get("PATH", "")
    env["CODEX_HOME"] = str(codex_home)
    command = [
        codex,
        "--ask-for-approval",
        "never",
        "exec",
        *([] if model is None else ["--model", model]),
        "--ephemeral",
        "--sandbox",
        "workspace-write",
        "--json",
        "--output-last-message",
        last,
        "-C",
        root,
        prompt,
    ]
    started = time.monotonic()
    attempts = []
    for attempt in range(3):
        result = run(command, root, env=env, timeout=480, check=False)
        attempts.append(result)
        if result.returncode == 0 or "at capacity" not in (result.stdout + result.stderr).lower():
            break
        time.sleep(5 * (attempt + 1))
    seconds = round(time.monotonic() - started, 3)
    events = "".join(value.stdout.rstrip() + "\n" for value in attempts)
    (output / f"{case.name}-{arm}-events.jsonl").write_text(
        events, encoding="utf-8", newline="\n"
    )
    (output / f"{case.name}-{arm}-stderr.log").write_text(
        "".join(value.stderr.rstrip() + "\n" for value in attempts if value.stderr),
        encoding="utf-8",
        newline="\n",
    )
    untracked = run(["git", "ls-files", "--others", "--exclude-standard"], root).stdout.splitlines()
    if untracked:
        run(["git", "add", "-N", "--", *untracked], root)
    (output / f"{case.name}-{arm}.diff").write_text(
        run(["git", "diff", "--binary", baseline, "--"], root).stdout,
        encoding="utf-8",
        newline="\n",
    )
    evaluation = evaluate(case, root)
    check_exit = None
    if arm == "nwc":
        checked = run([binary, "check", *event_args(case), "--json"], root, check=False)
        check_exit = checked.returncode
        (output / f"{case.name}-nwc-check.json").write_text(
            checked.stdout, encoding="utf-8", newline="\n"
        )
        (output / f"{case.name}-nwc-check.stderr.log").write_text(
            checked.stderr, encoding="utf-8", newline="\n"
        )
    evaluation.update(
        {
            "case": case.name,
            "arm": arm,
            "agent_exit": result.returncode,
            "seconds": seconds,
            "wake_observed": arm == "nwc" and due is not None,
            "wake_seconds": wake_seconds,
            "resolve_observed": arm == "nwc" and command_observed(events, "nwc resolve"),
            "check_observed": arm == "nwc",
            "check_exit": check_exit,
        }
    )
    return evaluation


def render(summary):
    by_case = {}
    for item in summary["results"]:
        by_case.setdefault(item["case"], {})[item["arm"]] = item
    lines = [
        "# Now We Can Paired Agent Benchmark",
        "",
        f"Run from `{summary['started_at']}` to `{summary['completed_at']}` with "
        f"`{summary['agent']}` on `{summary['platform']}`.",
        "",
        "| Case | Baseline | Now We Can | Wake | Resolve | Check | Paired improvement |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for case in CASES:
        if case.name not in by_case:
            continue
        baseline, guided = by_case[case.name]["baseline"], by_case[case.name]["nwc"]
        improved = baseline["outcome"] == "deferment_missed" and guided["outcome"] == "pass"
        lines.append(
            f"| `{case.name}` | {baseline['outcome']} | {guided['outcome']} | "
            f"{'yes' if guided['wake_observed'] else 'no'} | "
            f"{'yes' if guided['resolve_observed'] else 'no'} | "
            f"{'pass' if guided['check_exit'] == 0 else 'fail'} | "
            f"{'yes' if improved else 'no'} |"
        )
    lines += [
        "",
        f"Baseline deferments missed: **{summary['baseline_misses']}**.",
        "",
        f"Now We Can deferments missed: **{summary['nwc_misses']}**.",
        "",
        f"Paired improvements: **{summary['paired_improvements']} of "
        f"{summary['baseline_misses']} observed baseline misses**.",
        "",
        f"Regressions against passing baselines: **{summary['regressions']}**.",
        "",
        f"Overall protocol result: **{'PASS' if summary['passed'] else 'FAIL'}**.",
        "",
        "A paired improvement counts only when the baseline completes the requested "
        "task but misses the previously captured obligation, while the Now We Can arm "
        "completes both. Baseline passes are ties, never attributed preventions.",
        "",
        "The corpus is synthetic and the deferments are disclosed pre-captured fixtures. "
        "The genesis harness measures capture separately; this benchmark isolates "
        "deterministic wake-up and agent execution.",
        "",
        "## Scenario sources",
        "",
        "| Case | Domain | Primary source |",
        "| --- | --- | --- |",
    ]
    lines += [f"| `{case.name}` | {case.domain} | {case.source} |" for case in CASES]
    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nwc", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model")
    parser.add_argument("--case", action="append", choices=[case.name for case in CASES])
    parser.add_argument("--seed", type=int, default=20260726)
    parser.add_argument("--work-parent", type=Path, default=Path.cwd().parent)
    parser.add_argument("--keep-worktree", action="store_true")
    args = parser.parse_args()
    binary = args.nwc.resolve()
    output = args.output.resolve()
    if not binary.is_file():
        raise SystemExit(f"nwc binary not found: {binary}")
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    selected = [case for case in CASES if not args.case or case.name in args.case]
    order = [(case, arm) for case in selected for arm in ("baseline", "nwc")]
    random.Random(args.seed).shuffle(order)
    started_at = datetime.now(timezone.utc).isoformat()
    work_parent = args.work_parent.resolve()
    work_parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="nwc-paired-", dir=work_parent))
    results = []
    try:
        codex_home = work / "codex-home"
        codex_home.mkdir()
        configured_home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
        auth = configured_home / "auth.json"
        if auth.is_file():
            shutil.copy2(auth, codex_home / "auth.json")
        elif "OPENAI_API_KEY" not in os.environ:
            raise SystemExit("Codex authentication not found")
        (codex_home / "config.toml").write_text(
            'approval_policy = "never"\nsandbox_mode = "workspace-write"\n',
            encoding="utf-8",
            newline="\n",
        )
        for index, (case, arm) in enumerate(order, 1):
            print(f"[{index}/{len(order)}] {case.name} {arm}", flush=True)
            results.append(
                execute(
                    case,
                    arm,
                    work / f"{case.name}-{arm}",
                    output,
                    binary,
                    args.codex,
                    args.model,
                    codex_home,
                )
            )
        by_case = {}
        for item in results:
            by_case.setdefault(item["case"], {})[item["arm"]] = item
        baseline_misses = sum(
            pair["baseline"]["outcome"] == "deferment_missed" for pair in by_case.values()
        )
        nwc_misses = sum(
            pair["nwc"]["outcome"] == "deferment_missed" for pair in by_case.values()
        )
        improvements = sum(
            pair["baseline"]["outcome"] == "deferment_missed"
            and pair["nwc"]["outcome"] == "pass"
            for pair in by_case.values()
        )
        regressions = sum(
            pair["baseline"]["outcome"] == "pass"
            and pair["nwc"]["outcome"] != "pass"
            for pair in by_case.values()
        )
        passed = (
            all(pair["nwc"]["outcome"] == "pass" for pair in by_case.values())
            and all(pair["nwc"]["wake_observed"] for pair in by_case.values())
            and all(pair["nwc"]["resolve_observed"] for pair in by_case.values())
            and all(pair["nwc"]["check_exit"] == 0 for pair in by_case.values())
            and regressions == 0
        )
        summary = {
            "schema": 1,
            "benchmark": "paired-agent-deferment-delivery",
            "started_at": started_at,
            "completed_at": datetime.now(timezone.utc).isoformat(),
            "agent": run([args.codex, "--version"], Path.cwd()).stdout.strip(),
            "model": args.model or "Codex CLI default",
            "codex_home": "isolated authentication-only home",
            "nwc": {
                "version": run([binary, "--version"], Path.cwd()).stdout.strip(),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "platform": platform.platform(),
            "seed": args.seed,
            "order": [f"{case.name}:{arm}" for case, arm in order],
            "baseline_misses": baseline_misses,
            "nwc_misses": nwc_misses,
            "paired_improvements": improvements,
            "regressions": regressions,
            "results": results,
            "passed": passed,
        }
        (output / "summary.json").write_text(
            json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
        report = render(summary)
        (output / "REPORT.md").write_text(report, encoding="utf-8", newline="\n")
        print(report)
        if not passed:
            raise SystemExit(1)
    finally:
        if args.keep_worktree:
            print(f"worktree={work}", flush=True)
        else:
            shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()

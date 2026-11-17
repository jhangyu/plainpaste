#!/usr/bin/env python3
"""Single test entry point: builds the cargo test binaries, runs every test
in parallel with a per-test and a total time budget, and enforces the
no-sleep rule on the test directory."""

import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import cargo_trees

ROOT = Path(__file__).resolve().parent.parent
TEST_DIRS = [ROOT / "tests"]
PER_TEST_BUDGET_S = 1.0
TOTAL_BUDGET_S = 40.0
SLOWEST_SHOWN = 5
SLEEP_PATTERN = re.compile(r"\b(sleep|park_timeout|recv_timeout|wait_timeout)\b", re.IGNORECASE)
TEST_LINE_SUFFIX = ": test"


def scan_for_sleep() -> list[str]:
    hits = []
    for test_dir in TEST_DIRS:
        for path in sorted(test_dir.rglob("*.rs")):
            for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if SLEEP_PATTERN.search(line):
                    hits.append(f"{path.relative_to(ROOT)}:{lineno}: {line.strip()}")
    return hits


def build_test_binaries() -> list[str]:
    return [
        msg["executable"] for msg in cargo_trees.cargo("test", "--no-run")
        if msg.get("reason") == "compiler-artifact" and msg.get("profile", {}).get("test") and msg.get("executable")
    ]


def list_tests(binary: str, ignored: bool) -> list[str]:
    args = [binary, "--list", "--format", "terse"] + (["--ignored"] if ignored else [])
    out = subprocess.run(args, capture_output=True, text=True, check=True).stdout
    return [line[: -len(TEST_LINE_SUFFIX)] for line in out.splitlines() if line.endswith(TEST_LINE_SUFFIX)]


def run_one(binary: str, name: str) -> tuple[str, float, int, str]:
    start = time.monotonic()
    proc = subprocess.run([binary, "--exact", name, "--test-threads=1"], capture_output=True, text=True)
    elapsed = time.monotonic() - start
    label = f"{Path(binary).name}::{name}"
    return label, elapsed, proc.returncode, proc.stdout + proc.stderr


def main() -> int:
    failures = []
    sleep_hits = scan_for_sleep()
    for hit in sleep_hits:
        failures.append(f"sleep-like word in test dir: {hit}")
    icon = subprocess.run([sys.executable, str(ROOT / "scripts" / "gen_app_icon.py"), "--check"],
                          capture_output=True, text=True)
    if icon.returncode != 0:
        failures.append(f"gen_app_icon.py --check: expected RC=0 (ui/app-icon.slint matches assets/app-icon.svg),"
                        f" actual RC={icon.returncode}\n{icon.stdout}{icon.stderr}")

    binaries = build_test_binaries()
    cargo_trees.verify(["debug"])
    jobs, skipped = [], 0
    for binary in binaries:
        ignored = set(list_tests(binary, ignored=True))
        skipped += len(ignored)
        jobs += [(binary, name) for name in list_tests(binary, ignored=False) if name not in ignored]
    if not jobs:
        failures.append("no tests found: expected at least one test, actual 0")

    start = time.monotonic()
    with ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
        results = list(pool.map(lambda job: run_one(*job), jobs))
    total = time.monotonic() - start

    for label, elapsed, code, output in results:
        if code != 0:
            failures.append(f"FAILED {label} (exit code {code})\n{output}")
        if elapsed > PER_TEST_BUDGET_S:
            failures.append(f"SLOW {label}: expected <= {PER_TEST_BUDGET_S:.1f}s, actual {elapsed:.3f}s")
    if total > TOTAL_BUDGET_S:
        failures.append(f"SUITE TOO SLOW: expected <= {TOTAL_BUDGET_S:.0f}s, actual {total:.2f}s")

    print(f"tests run: {len(results)}  skipped: {skipped}  total: {total:.2f}s")
    print(f"slowest {SLOWEST_SHOWN}:")
    for label, elapsed, _, _ in sorted(results, key=lambda r: r[1], reverse=True)[:SLOWEST_SHOWN]:
        print(f"  {elapsed:.3f}s  {label}")
    for failure in failures:
        print(failure)
    print("RESULT: FAIL" if failures else "RESULT: PASS")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

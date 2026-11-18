#!/usr/bin/env python3
"""Single runner for every cargo call of the project scripts (test.py, bundle_macos.py,
check_platforms.py). It enforces two rules and never deletes anything:

- One artifact set per build tree: each tree under build/target may only hold the units
  (build/<package>/<hash>/ dirs, cargo's per-unit build-dir layout) that cargo reports for that
  tree's canonical commands, run on the current checkout. Anything else is stale.
- Incremental compilation stays off: no env override may enable it, and no tree may hold
  a non-empty incremental/ dir.

A violation exits non-zero and prints each stale unit and the exact fix command."""

import filecmp
import functools
import json
import os
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WINDOWS_TARGET = "x86_64-pc-windows-msvc"
MESSAGE_FORMAT = "--message-format=json-render-diagnostics"
# Tree (relative to the target dir) -> canonical cargo commands whose units make up its one
# set. Host build scripts and proc-macros of the Windows run land in debug/, so that run is
# part of debug's set too. docker-linux trees are marked only by their container runs (check_platforms clippy, ci.py release).
# Clippy lint args (`-- -D warnings`) do not change unit hashes, so marking runs without them.
# `cargo check --all-targets` covers plain `cargo check` and `cargo check --tests` (routine for IDEs).
CANONICAL = {
    "debug": [
        ["build"],
        ["test", "--no-run"],
        ["check", "--all-targets"],
        ["clippy", "--all-targets"],
        ["clippy", "--target", WINDOWS_TARGET, "--all-targets"],
    ],
    "release": [["build", "--release", "--locked"]],
    f"{WINDOWS_TARGET}/debug": [["clippy", "--target", WINDOWS_TARGET, "--all-targets"]],
    "docker-linux/debug": None,
    "docker-linux/release": None,
}
PROFILE_FLAG = {"debug": "--profile dev", "release": "--release"}
# Worktree builds (sources under scratch/) re-hash the workspace crate inside debug/. They are
# accepted there and removed by `cargo_trees.py prune-scratch-units`.
WORKTREE_TREES = {"debug"}
WORKTREE_SOURCE = f"{ROOT / 'scratch'}{os.sep}"
# Top-level entries of the target dir: canonical tree roots, cargo's own files, exp-* trees.
TOP_LEVEL = {"debug", "release", WINDOWS_TARGET, "docker-linux", "tmp", "CACHEDIR.TAG", ".rustc_info.json"}
NESTED_LEVEL = {WINDOWS_TARGET: {"debug", "tmp", "CACHEDIR.TAG"},
                "docker-linux": {"debug", "release", "tmp", "CACHEDIR.TAG", ".rustc_info.json"}}
EXPERIMENT_PREFIX = "exp-"
UNIT_HASH = re.compile(r"^[0-9a-f]{16}$")
TREE_MARKERS = {"build", "examples", "incremental"}
UNIT_NAME = re.compile(r"^(?P<name>.+)-(?P<hash>[0-9a-f]{16})(?:\..*)?$")
INCREMENTAL_ENV = re.compile(r"^(CARGO_INCREMENTAL|CARGO_BUILD_INCREMENTAL|CARGO_PROFILE_.+_INCREMENTAL)$")
INCREMENTAL_OFF = {"0", "false"}
# Env overrides that re-hash units into a second set inside a canonical tree. Experiments use
# `--profile exp-<name> --config 'profile.exp-<name>.inherits="..."'` instead.
REHASH_ENV = re.compile(r"^(CARGO_PROFILE_.+|RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_BUILD_RUSTFLAGS)$")

_live: dict[str, set[str]] = {}
_ran: set[tuple[str, ...]] = set()


@functools.cache
def _metadata() -> dict:
    out = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                         cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return json.loads(out)


def target_dir() -> Path:
    """The cargo build directory (.cargo/config.toml build.target-dir)."""
    return Path(_metadata()["target_directory"])


def env_check() -> None:
    """Fails if any env override re-enables incremental compilation or re-hashes units."""
    bad = [f"{k}={v}" for k, v in os.environ.items()
           if (INCREMENTAL_ENV.match(k) and v.lower() not in INCREMENTAL_OFF)
           or (REHASH_ENV.match(k) and not INCREMENTAL_ENV.match(k))]
    if bad:
        sys.exit(f"cargo_trees: expected no incremental or unit re-hashing env overrides, actual {bad}."
                 f" Fix: unset {' '.join(b.split('=')[0] for b in bad)}")


def _unit_hash(tree: Path, path: Path) -> str:
    """Unit hash of an artifact path: the build/<package>/<hash>/ dir it lives in, or, for a
    file uplifted to the tree root, the hash of its twin in some unit's out/ dir. The twin is a
    hardlink or (on APFS) a copy, so it is found by name and identical content."""
    rel = path.relative_to(tree)
    if rel.parts[0] in TREE_MARKERS:
        if len(rel.parts) < 3 or not UNIT_HASH.match(rel.parts[2]):
            sys.exit(f"cargo_trees: expected a build/<package>/<hash>/ unit path, actual {path}")
        return rel.parts[2]
    stem, dot, ext = path.name.partition(".")
    twins = {out.parent.parent.name for out in tree.glob("build/*/*/out/*")
             if (out.name == path.name or ((m := UNIT_NAME.match(out.name)) and m["name"] == stem.replace("-", "_")
                                           and out.name.endswith(dot + ext)))
             and out.is_file() and filecmp.cmp(out, path, shallow=False)}
    if len(twins) != 1:
        sys.exit(f"cargo_trees: expected exactly one unit out/ twin of {path}, actual {sorted(twins)}."
                 f" Fix: {_fix(tree.relative_to(target_dir()).as_posix())}")
    return twins.pop()


def _tree_of(path: Path) -> Path:
    parts = path.relative_to(target_dir()).parts
    for i, part in enumerate(parts[:-1]):
        if part in TREE_MARKERS:
            return target_dir().joinpath(*parts[:i])
    return path.parent


def record(lines: list[str], path_map: tuple[str, str] | None = None) -> None:
    """Adds the unit hashes reported in cargo JSON lines to the live set of their tree.
    path_map rewrites a container path prefix to the host one. Non-JSON lines are printed."""
    for line in lines:
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            print(line, flush=True)
            continue
        if msg.get("reason") == "compiler-artifact":
            paths = msg["filenames"]
        elif msg.get("reason") == "build-script-executed":
            paths = [msg["out_dir"]]
        else:
            continue
        for raw in paths:
            if path_map and raw.startswith(path_map[0]):
                raw = path_map[1] + raw[len(path_map[0]):]
            path = Path(raw)
            if path.is_dir() and _tree_of(path) == path.parent:
                continue  # uplifted dir copy (e.g. .dSYM); the unit is marked through its files
            tree = _tree_of(path)
            rel = tree.relative_to(target_dir()).as_posix()
            _live.setdefault(rel, set()).add(_unit_hash(tree, path))


def cargo(*args: str) -> list[dict]:
    """Runs `cargo <args>` with JSON messages (diagnostics still rendered on stderr), records
    its units, and returns the messages. A non-zero exit code ends the process."""
    env_check()
    sep = args.index("--") if "--" in args else len(args)
    cmd = ["cargo", *args[:sep], MESSAGE_FORMAT, *args[sep:]]
    print("+", " ".join(cmd), flush=True)
    proc = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    if proc.returncode != 0:
        sys.exit(f"{' '.join(cmd)}: expected RC=0, actual RC={proc.returncode}")
    lines = proc.stdout.splitlines()
    record(lines)
    _ran.add(tuple(args))
    return [json.loads(line) for line in lines]


def _names_scratch(unit: Path) -> bool:
    """True when the unit's dep-info or build-script output names a path under scratch/."""
    files = [*unit.glob("out/*.d"), *unit.glob("run/*")]
    return any(WORKTREE_SOURCE in f.read_text(encoding="utf-8", errors="replace") for f in files if f.is_file())


def _worktree_units(tree: Path) -> set[Path]:
    """Unit dirs of the workspace-package units built from a worktree (sources under scratch/).
    Lib, bin and test dep-info lists sources by relative path, so only the build-script run
    names scratch/; the package's other units are found through fingerprint dep edges."""
    by_fingerprint, links = {}, {}
    for package in (p["name"] for p in _metadata()["packages"]):
        for unit in (tree / "build" / package).glob("*"):
            if not UNIT_HASH.match(unit.name):
                continue
            links[unit] = set()
            for spec in (unit / "fingerprint").glob("*.json"):
                if spec.with_suffix("").is_file():
                    by_fingerprint[spec.with_suffix("").read_text(encoding="utf-8").strip()] = unit
                links[unit] |= {struct.pack("<Q", dep[-1]).hex() for dep in json.loads(spec.read_text(encoding="utf-8"))["deps"]}
    edges = {u: set() for u in links}
    for unit, fingerprints in links.items():
        for other in filter(None, map(by_fingerprint.get, fingerprints)):
            edges[unit].add(other)
            edges[other].add(unit)
    found = {u for u in edges if _names_scratch(u)}
    stack = list(found)
    while stack:
        for other in edges[stack.pop()] - found:
            found.add(other)
            stack.append(other)
    return found


def _fix(rel: str) -> str:
    if rel in PROFILE_FLAG:
        return f"cargo clean {PROFILE_FLAG[rel]}"
    return f"rm -rf '{target_dir() / rel}'"


def _own_names() -> set[str]:
    """Entry-name stems of the workspace crate's units (package dirs, target files)."""
    names = set()
    for package in _metadata()["packages"]:
        names.add(package["name"])
        for target in package["targets"]:
            stem = target["name"].replace("-", "_")
            names |= {stem, f"lib{stem}"}
    return names


def _own_fix(rel: str) -> str:
    """Targeted fix when every stale unit belongs to the workspace crate (an own-crate
    `--config profile.X.package.<crate>.*` experiment): clean only that crate's units."""
    packages = " ".join(f"-p {p['name']}" for p in _metadata()["packages"])
    return f"cargo clean {packages} {PROFILE_FLAG[rel]}"


def check_trees(trees: dict[str, list[list[str]] | None], worktree_trees: set[str] = WORKTREE_TREES) -> list[str]:
    """Marks each existing tree with its commands (a tree marked as None must have been
    recorded already), then returns one problem line per stale unit."""
    problems = []
    for rel, commands in trees.items():
        tree = target_dir() / rel
        if not tree.is_dir():
            continue
        if commands is None and rel not in _live:
            continue
        for args in commands or []:
            if tuple(args) not in _ran:
                cargo(*args)
        live = _live.get(rel, set())
        stale = {}
        for unit in sorted(tree.glob("build/*/*")):
            # `cargo test` creates an empty dir for its doctest unit and reports no artifact for it.
            if UNIT_HASH.match(unit.name) and unit.name not in live and any(p.is_file() for p in unit.rglob("*")):
                stale.setdefault(unit.name, []).append(unit.relative_to(tree).as_posix())
        accepted = {u.name for u in _worktree_units(tree)} if rel in worktree_trees else set()
        stale = {h: entries for h, entries in stale.items() if h not in accepted}
        own = _own_names()
        only_own = rel in PROFILE_FLAG and all(
            e.split("/")[1] in own for entries in stale.values() for e in entries)
        fix = _own_fix(rel) if only_own else _fix(rel)
        for unit_hash, entries in sorted(stale.items()):
            problems.append(f"{rel}: stale unit {unit_hash} ({', '.join(entries)}). Fix: {fix}")
    return problems


def check_incremental(trees: list[str]) -> list[str]:
    """One problem line per existing tree whose incremental/ dir is not empty. Needs no build."""
    problems = []
    for rel in trees:
        incremental = target_dir() / rel / "incremental"
        if incremental.is_dir() and any(incremental.iterdir()):
            problems.append(f"{rel}: expected incremental/ empty (incremental compilation off), actual"
                            f" {sorted(p.name for p in incremental.iterdir())}. Fix: {_fix(rel)}")
    return problems


def prune_scratch_units(rel: str = "debug") -> None:
    """Deletes the worktree unit dirs (sources under scratch/) accepted in tree `rel`. Run it when no
    build is in flight."""
    tree = target_dir() / rel
    for unit in sorted(_worktree_units(tree)):
        print(f"cargo_trees: deleting worktree unit dir {unit}", flush=True)
        shutil.rmtree(unit)


def check_layout() -> list[str]:
    """Every entry of the target dir must be a canonical tree, cargo's own file, or exp-*."""
    problems = []
    top = target_dir()
    for entry in sorted(top.iterdir()):
        allowed = entry.name in TOP_LEVEL or entry.name.startswith(EXPERIMENT_PREFIX)
        if not allowed:
            problems.append(f"{entry}: expected a canonical tree {sorted(TOP_LEVEL)} or {EXPERIMENT_PREFIX}*,"
                            f" actual unknown entry. Fix: rm -rf '{entry}'")
        elif entry.name in NESTED_LEVEL:
            for sub in sorted(entry.iterdir()):
                if sub.name not in NESTED_LEVEL[entry.name] and not sub.name.startswith(EXPERIMENT_PREFIX):
                    problems.append(f"{sub}: expected one of {sorted(NESTED_LEVEL[entry.name])} or"
                                    f" {EXPERIMENT_PREFIX}*, actual unknown entry. Fix: rm -rf '{sub}'")
    return problems


def verify(trees: list[str]) -> None:
    """Checks the target dir layout, incremental/ of every canonical tree, and the one artifact
    set of `trees` (each entrypoint passes the trees its own commands build). Exits non-zero
    on any problem."""
    unknown = [rel for rel in trees if rel not in CANONICAL]
    if unknown:
        sys.exit(f"cargo_trees: expected trees from {list(CANONICAL)}, actual {unknown}")
    problems = check_layout() + check_incremental(list(CANONICAL)) + check_trees({rel: CANONICAL[rel] for rel in trees})
    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        sys.exit(f"cargo_trees: expected exactly one artifact set per build tree, actual {len(problems)} problem(s)")
    print(f"cargo_trees: one artifact set in {trees}, incremental off", flush=True)


if __name__ == "__main__":
    commands = {"env-check": env_check, "prune-scratch-units": prune_scratch_units}
    if len(sys.argv) >= 2 and sys.argv[1] == "verify":
        verify(sys.argv[2:] or list(CANONICAL))
    elif len(sys.argv) == 2 and sys.argv[1] in commands:
        commands[sys.argv[1]]()
    else:
        sys.exit(f"usage: cargo_trees.py {{verify [tree...]|{'|'.join(commands)}}}, actual {sys.argv[1:]}")

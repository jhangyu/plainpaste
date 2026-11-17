#!/usr/bin/env python3
"""Cross-platform compile gate: clippy for the host, for Windows (host cross-check, no
linking) and for Linux (inside a Docker container), then the one-artifact-set check of
every build tree. Stops at the first non-zero exit code. A missing Windows target or
Docker daemon is a failure, never a skip."""

import subprocess
import sys
from pathlib import Path

import cargo_trees

ROOT = Path(__file__).resolve().parent.parent
WINDOWS_TARGET = cargo_trees.WINDOWS_TARGET
LINUX_IMAGE = "rust:1.100.0-bookworm"
CONTAINER_SRC_DIR = "/src"
CONTAINER_TARGET_DIR = f"{CONTAINER_SRC_DIR}/build/target/docker-linux"
CONTAINER_CARGO_REGISTRY = "/usr/local/cargo/registry"
CLIPPY_ARGS = ["--all-targets", "--", "-D", "warnings"]
LINUX_DEPS = "apt-get update -qq && apt-get install -y -qq pkg-config libfontconfig1-dev"
LINUX_SCRIPT = (
    f"{LINUX_DEPS} && rustup component add clippy"
    f" && cargo clippy {cargo_trees.MESSAGE_FORMAT} {' '.join(CLIPPY_ARGS)}"
)


def check_preconditions() -> list[str]:
    problems = []
    targets = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        capture_output=True, text=True,
    )
    if targets.returncode != 0:
        problems.append(
            f"rustup target list --installed: expected RC=0, actual RC={targets.returncode}: {targets.stderr.strip()}"
        )
    elif WINDOWS_TARGET not in targets.stdout.split():
        problems.append(
            f"expected rustup target {WINDOWS_TARGET} installed, actual: {targets.stdout.split()}"
            f" (run: rustup target add {WINDOWS_TARGET})"
        )
    docker = subprocess.run(["docker", "info"], capture_output=True, text=True)
    if docker.returncode != 0:
        problems.append(
            f"docker info: expected RC=0 (daemon running), actual RC={docker.returncode}: {docker.stderr.strip()}"
        )
    return problems


def run_linux_container(script: str, docker_platform: str | None) -> int:
    """Run `script` in the Linux image with the repo mounted. Its cargo JSON marks the
    docker-linux tree. `docker_platform` is e.g. "linux/amd64", None = native arch."""
    registry = Path.home() / ".cargo" / "registry"
    cmd = ["docker", "run", "--rm"]
    if docker_platform:
        cmd += ["--platform", docker_platform]
    cmd += [
        "-v", f"{ROOT}:{CONTAINER_SRC_DIR}",
        "-v", f"{registry}:{CONTAINER_CARGO_REGISTRY}",
        "-w", CONTAINER_SRC_DIR,
        "-e", f"CARGO_TARGET_DIR={CONTAINER_TARGET_DIR}",
        LINUX_IMAGE, "sh", "-c", script,
    ]
    print("+", " ".join(cmd), flush=True)
    proc = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE, text=True)
    cargo_trees.record(proc.stdout.splitlines(), (f"{CONTAINER_SRC_DIR}/", f"{ROOT}/"))
    if proc.returncode != 0:
        print(f"linux container in {LINUX_IMAGE}: expected RC=0, actual RC={proc.returncode}", file=sys.stderr)
    return proc.returncode


def linux_gate() -> int:
    return run_linux_container(LINUX_SCRIPT, None)


def main() -> int:
    problems = check_preconditions()
    if problems:
        sys.stderr.write("check_platforms: preconditions failed:\n")
        for problem in problems:
            sys.stderr.write(f"  {problem}\n")
        return 1
    cargo_trees.cargo("clippy", *CLIPPY_ARGS)
    cargo_trees.cargo("clippy", "--target", WINDOWS_TARGET, *CLIPPY_ARGS)
    rc = linux_gate()
    if rc != 0:
        return rc
    cargo_trees.verify(["debug", f"{WINDOWS_TARGET}/debug", "docker-linux/debug"])
    return 0


if __name__ == "__main__":
    sys.exit(main())

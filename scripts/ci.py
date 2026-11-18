#!/usr/bin/env python3
"""Release script: `guard`, `build --platform {macos|linux|windows|all} [--tag]`,
`publish --tag [--from DIR]`. Builds release assets into dist/<platform>/ and publishes them
as a GitHub Release. It refuses to run tests: before anything else it scans its own source
and every scripts/ module it imports for test invocations, and at runtime it refuses to
spawn any child process that looks like one. Releases are built and published only by GitHub
Actions: `publish` refuses to run unless GITHUB_ACTIONS=true, and a local `build` only verifies
the CI flow (its output is never released).

Route = (platform, host), no mode flag:
  macos   darwin only (native build + ditto zip)
  linux   darwin (native aarch64 docker) or linux (x86_64 docker on the runner)
  windows win32 native, any other host: ssh to $PLAINPASTE_WIN_HOST (e.g. user@host, key file in
          $PLAINPASTE_WIN_KEY) and run this script there
"""

import argparse
import ast
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = ROOT / "scripts"

# --- test guard (runs before any scripts/ import) -------------------------------------------
# Constants assigned to GUARD_* names are the guard's own patterns and are not scanned.
GUARD_ARGS = frozenset({"test", "t", "nextest", "bench"})
GUARD_TEXT = re.compile(
    r"\bcargo\s+(\+\S+\s+)?(test|t|nextest|bench)\b|\btest\.py\b|\bnextest\b|\bpytest\b"
)
GUARD_SINKS = frozenset({"run", "Popen", "call", "check_call", "check_output", "cargo"})
GUARD_MODULES = frozenset({"unittest", "pytest", "doctest", "test", "pty"})
GUARD_OS_CALL = re.compile(r"^(system|popen|exec.*|spawn.*)$")


def _imported_scripts(tree: ast.AST) -> set[Path]:
    names = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names |= {a.name.split(".")[0] for a in node.names}
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            names.add(node.module.split(".")[0])
    return {SCRIPTS_DIR / f"{n}.py" for n in names if (SCRIPTS_DIR / f"{n}.py").is_file()}


def _skipped_lines(tree: ast.Module) -> set[int]:
    """Lines of docstrings (never executed) and of GUARD_* pattern definitions."""
    skipped: set[int] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant):
            skipped.update(range(node.lineno, (node.end_lineno or node.lineno) + 1))
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id.startswith("GUARD_") for t in node.targets
        ):
            skipped.update(range(node.lineno, (node.end_lineno or node.lineno) + 1))
    return skipped


def _literal_args(call: ast.Call) -> list[str]:
    found = []
    for arg in [*call.args, *(k.value for k in call.keywords)]:
        elements = arg.elts if isinstance(arg, (ast.List, ast.Tuple)) else [arg]
        found += [e.value for e in elements if isinstance(e, ast.Constant) and isinstance(e.value, str)]
    return found


def _hits(path: Path) -> list[tuple[int, str]]:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    skipped = _skipped_lines(tree)
    hits = []
    for node in ast.walk(tree):
        line = getattr(node, "lineno", None)
        if line is None or line in skipped:
            continue
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and GUARD_TEXT.search(node.value):
            hits.append((line, "string runs a test command"))
        elif isinstance(node, ast.Call):
            func = node.func
            sink = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")
            if sink in GUARD_SINKS and GUARD_ARGS & set(_literal_args(node)):
                hits.append((line, f"{sink}() call with a test argument"))
            if (isinstance(func, ast.Attribute) and isinstance(func.value, ast.Name)
                    and func.value.id == "os" and GUARD_OS_CALL.match(func.attr)):
                hits.append((line, f"os.{func.attr}() bypasses the runtime guard"))
        elif isinstance(node, ast.Import) and GUARD_MODULES & {a.name.split(".")[0] for a in node.names}:
            hits.append((line, "test framework import"))
        elif isinstance(node, ast.ImportFrom) and node.module and node.module.split(".")[0] in GUARD_MODULES:
            hits.append((line, "test framework import"))
    return hits


def scan_sources() -> list[str]:
    """Violations `path:line: reason: source line` in this file and its scripts/ import closure."""
    pending, seen, problems = [Path(__file__).resolve()], set(), []
    while pending:
        path = pending.pop()
        if path in seen:
            continue
        seen.add(path)
        lines = path.read_text(encoding="utf-8").splitlines()
        problems += [f"{path.relative_to(ROOT)}:{n}: {why}: {lines[n - 1].strip()}" for n, why in _hits(path)]
        pending += _imported_scripts(ast.parse(path.read_text(encoding="utf-8")))
    return problems


def install_runtime_guard() -> None:
    """Refuse every child process whose argv looks like a test run.
    ponytail: Popen wrapper, not a data-flow analysis. It is exact for every child this
    process spawns (subprocess.run and check_* all go through Popen)."""
    original = subprocess.Popen.__init__

    def guarded(self, args, *a, **kw):
        argv = [args] if isinstance(args, (str, bytes, os.PathLike)) else list(args)
        for element in map(os.fsdecode, argv):
            if element in GUARD_ARGS or GUARD_TEXT.search(element):
                sys.exit(f"refused test invocation: expected a build or release command, actual {argv}")
        original(self, args, *a, **kw)

    subprocess.Popen.__init__ = guarded


def run_guard() -> None:
    problems = scan_sources()
    if problems:
        sys.exit("test guard: expected no test invocation, actual:\n  " + "\n  ".join(problems))
    install_runtime_guard()


run_guard()

# imports follow the guard so imported scripts are scanned first
import bundle_macos
import cargo_trees
import check_platforms

# --- release ----------------------------------------------------------------------------------
PLATFORMS = ("macos", "linux", "windows")
TAG_PATTERN = re.compile(r"^v\d+\.\d+\.\d+$")
DIST = ROOT / "dist"
BINARY = "plainpaste"
WIN_HOST_ENV = "PLAINPASTE_WIN_HOST"  # ssh target, e.g. user@host
WIN_KEY_ENV = "PLAINPASTE_WIN_KEY"  # ssh private key path
WIN_REMOTE_DIR = "plainpaste"
WIN_SRC_TAR = "plainpaste-src.tar"
WIN_MACHINE = "AMD64"
REMOTE_DONE = "REMOTE-STEPS-COMPLETE"  # sentinel the remote script prints last; ci.py requires it
MACOS_MACHINE = "arm64"
# (host system, machine) -> linux asset arch. Darwin runs the native-arch container.
LINUX_ARCH = {("Darwin", "arm64"): "aarch64", ("Linux", "x86_64"): "x86_64"}
RELEASE_LINUX_ARCH = "x86_64"


def require_env(name: str, what: str) -> str:
    value = os.environ.get(name)
    if not value:
        sys.exit(f"expected env {name} ({what}), actual unset")
    return value


def version() -> str:
    return bundle_macos.package_version()


def asset_name(plat: str, arch: str | None = None) -> str:
    ver = version()
    return {
        "macos": f"plainpaste-{ver}-macos-arm64.zip",
        "linux": f"plainpaste-{ver}-linux-{arch or RELEASE_LINUX_ARCH}",
        "windows": f"plainpaste-{ver}-windows-x86_64.exe",
    }[plat]


def sh(argv: list[str], what: str) -> None:
    print("+", " ".join(argv), flush=True)
    rc = subprocess.run(argv, cwd=ROOT, check=False).returncode
    if rc != 0:
        sys.exit(f"{what}: expected RC=0, actual RC={rc}")


def sh_expect_line(argv: list[str], what: str, sentinel: str) -> None:
    """Like sh(), streams output, and also fails when `sentinel` never appeared (a remote
    command that silently did nothing must not count as success)."""
    print("+", " ".join(argv), flush=True)
    seen = False
    with subprocess.Popen(argv, cwd=ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8", errors="replace") as proc:
        for line in proc.stdout:
            print(line, end="", flush=True)
            seen = seen or sentinel in line
    if proc.returncode != 0:
        sys.exit(f"{what}: expected RC=0, actual RC={proc.returncode}")
    if not seen:
        sys.exit(f"{what}: expected output line {sentinel!r}, actual none (remote script did not run to the end)")


def check_tag(tag: str | None) -> None:
    if tag is not None and tag != f"v{version()}":
        sys.exit(f"tag mismatch: expected tag v{version()}, actual {tag}")


def check_clean_tree() -> None:
    if not (ROOT / ".git").exists():
        return  # the remote Windows dir is a `git archive HEAD` extract, HEAD is already guaranteed
    out = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"],
                         cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8").stdout
    if out.strip():
        sys.exit(f"build needs a clean tree: expected no tracked changes, actual:\n{out}")


def fresh_dist(plat: str) -> Path:
    out = DIST / plat
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)
    return out


def build_macos() -> None:
    if sys.platform != "darwin" or platform.machine() != MACOS_MACHINE:
        sys.exit(f"macos build: expected darwin/{MACOS_MACHINE}, actual {sys.platform}/{platform.machine()}")
    bundle_macos.main()  # wipes and refills dist/macos
    asset = DIST / "macos" / asset_name("macos")
    sh(["ditto", "-c", "-k", "--keepParent", str(bundle_macos.APP), str(asset)], "ditto zip")
    print(f"built {asset}")


def build_linux() -> None:
    host = (platform.system(), platform.machine())
    if host not in LINUX_ARCH:
        sys.exit(f"linux build: expected host in {sorted(LINUX_ARCH)}, actual {host}")
    script = f"{check_platforms.LINUX_DEPS} && cargo build --release --locked {cargo_trees.MESSAGE_FORMAT}"
    rc = check_platforms.run_linux_container(script, None)
    if rc != 0:
        sys.exit(f"linux container build: expected RC=0, actual RC={rc}")
    cargo_trees.verify(["docker-linux/release"])
    asset = fresh_dist("linux") / asset_name("linux", LINUX_ARCH[host])
    shutil.copy2(cargo_trees.target_dir() / "docker-linux" / "release" / BINARY, asset)
    print(f"built {asset}")


def build_windows_native() -> None:
    if platform.machine() != WIN_MACHINE:
        sys.exit(f"windows build: expected machine {WIN_MACHINE}, actual {platform.machine()}")
    cargo_trees.cargo("build", "--release", "--locked")
    cargo_trees.verify(["release"])
    asset = fresh_dist("windows") / asset_name("windows")
    shutil.copy2(cargo_trees.target_dir() / "release" / f"{BINARY}.exe", asset)
    print(f"built {asset}")


def powershell_script(statements: list[str]) -> str:
    """Join statements for `powershell -Command` (single quotes only: it passes through cmd.exe).
    $LASTEXITCODE starts as $null (`$null -ne 0` is true: an unreset check exits 0 after statement 1),
    so each statement resets it, then a failed native command stops the run with its code."""
    check = "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }"
    body = "; ".join(f"$LASTEXITCODE = 0; {s}; {check}" for s in statements)
    return f"$ErrorActionPreference = 'Stop'; {body}; Write-Host '{REMOTE_DONE}'"


def build_windows_remote(tag: str | None) -> None:
    win_host = require_env(WIN_HOST_ENV, "ssh target, e.g. user@host")
    ssh_opts = ["-i", require_env(WIN_KEY_ENV, "ssh private key path"), "-o", "BatchMode=yes"]
    toolchain = re.search(r'channel\s*=\s*"([^"]+)"', (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    if toolchain is None:
        sys.exit("rust-toolchain.toml: expected a channel = \"...\" line, actual none")
    channel = toolchain.group(1)
    tarball = ROOT / "build" / "release-src.tar"
    tarball.parent.mkdir(parents=True, exist_ok=True)
    sh(["git", "archive", "--format=tar", "-o", str(tarball), "HEAD"], "git archive HEAD")
    sh(["scp", *ssh_opts, str(tarball), f"{win_host}:{WIN_SRC_TAR}"], "scp source tarball")
    tarball.unlink()
    build_cmd = "python scripts/ci.py build --platform windows" + (f" --tag {tag}" if tag else "")
    statements = [
        f"New-Item -ItemType Directory -Force {WIN_REMOTE_DIR} | Out-Null",
        f"Get-ChildItem {WIN_REMOTE_DIR} -Exclude build | Remove-Item -Recurse -Force",
        f"tar -xf {WIN_SRC_TAR} -C {WIN_REMOTE_DIR}",
        f"Remove-Item {WIN_SRC_TAR}",
        f"if (-not ((rustup toolchain list) -match '^{re.escape(channel)}')) "
        f"{{ Write-Host '!!! REMOTE INSTALLING Rust toolchain {channel} (not present on this host) !!!'; "
        f"rustup toolchain install {channel} --profile minimal }}",
        f"cd {WIN_REMOTE_DIR}",
        build_cmd,
    ]
    sh_expect_line(["ssh", *ssh_opts, win_host, f'powershell -NoProfile -Command "{powershell_script(statements)}"'],
                   "remote windows build", REMOTE_DONE)
    asset = fresh_dist("windows") / asset_name("windows")
    sh(["scp", *ssh_opts, f"{win_host}:{WIN_REMOTE_DIR}/dist/windows/{asset.name}", str(asset)],
       "scp windows asset")
    print(f"built {asset}")


def build_windows(tag: str | None) -> None:
    if sys.platform == "win32":
        build_windows_native()
    else:
        build_windows_remote(tag)


def cmd_build(plat: str, tag: str | None) -> None:
    check_tag(tag)
    check_clean_tree()
    if plat == "all" and sys.platform != "darwin":
        sys.exit(f"build --platform all: expected host darwin, actual {sys.platform}")
    for p in PLATFORMS if plat == "all" else (plat,):
        {"macos": build_macos, "linux": build_linux, "windows": lambda: build_windows(tag)}[p]()


NOTES_DIR = ROOT / "release-notes"
# Halcyon release format: English block, `---`, then the zh-TW block, each with the same three sections.
NOTES_HEADINGS = [
    "## Highlights", "### ✨ Added", "### 🛠 Fixed & Improved", "### ⚙️ Internal / Build", "---",
    "## 版本重點", "### ✨ 新增", "### 🛠 修正與改善", "### ⚙️ 內部/建置",
]


def render_notes(tag: str) -> str:
    """Release body for `tag` from release-notes/<tag>.md. Fails loudly if missing or off-format."""
    path = NOTES_DIR / f"{tag}.md"
    if not path.is_file():
        sys.exit(f"publish: expected release notes at {path}, actual missing")
    body = path.read_text(encoding="utf-8")
    lines = [ln for ln in body.splitlines() if ln.startswith("#") or ln == "---"]
    if lines != NOTES_HEADINGS:
        sys.exit(f"publish: {path} headings differ from the halcyon format, expected {NOTES_HEADINGS}, actual {lines}")
    return body


def cmd_publish(tag: str, source: Path) -> None:
    actions = os.environ.get("GITHUB_ACTIONS")
    if actions != "true":
        sys.exit(f"publish runs only on GitHub Actions: expected GITHUB_ACTIONS=true, actual {actions or 'unset'}")
    check_tag(tag)
    notes = render_notes(tag)
    files = []
    for plat in PLATFORMS:
        name = asset_name(plat)
        found = sorted(p for p in source.rglob(name) if p.is_file())
        if len(found) != 1:
            sys.exit(f"publish: expected exactly one {name} under {source}, actual {[str(p) for p in found]}")
        files.append(str(found[0]))
    notes_file = DIST / f"{tag}-notes.md"
    notes_file.parent.mkdir(parents=True, exist_ok=True)
    notes_file.write_text(notes, encoding="utf-8")
    sh(["gh", "release", "create", tag, "--verify-tag", "--notes-file", str(notes_file), "--title", tag, *files],
       "gh release create")


def tag_arg(value: str) -> str:
    if not TAG_PATTERN.match(value):
        raise argparse.ArgumentTypeError(f"expected tag like v1.2.3, actual {value}")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("guard", help="only run the test guard")
    build = sub.add_parser("build")
    build.add_argument("--platform", required=True, choices=[*PLATFORMS, "all"])
    build.add_argument("--tag", type=tag_arg)
    publish = sub.add_parser("publish")
    publish.add_argument("--tag", required=True, type=tag_arg)
    publish.add_argument("--from", dest="source", type=Path, default=DIST)
    args = parser.parse_args()
    if args.command == "build":
        cmd_build(args.platform, args.tag)
    elif args.command == "publish":
        cmd_publish(args.tag, args.source)


if __name__ == "__main__":
    main()

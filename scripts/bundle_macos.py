#!/usr/bin/env python3
"""Builds `dist/macos/PlainPaste.app`: release binary, Info.plist, an icon rendered from
assets/app-icon.svg, and an ad-hoc code signature. macOS only, stdlib only."""

import plistlib
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

import cargo_trees

ROOT = Path(__file__).resolve().parent.parent
APP_NAME = "PlainPaste"
EXECUTABLE = "plainpaste"
BUNDLE_ID = "app.plainpaste"
ICON_NAME = "AppIcon"
ICON_SOURCE = ROOT / "assets" / "app-icon.svg"
DIST = ROOT / "dist" / "macos"
APP = DIST / f"{APP_NAME}.app"
ICONSET = DIST / f"{ICON_NAME}.iconset"
ICON_MASTER = DIST / f"{ICON_NAME}-1024.png"
ICON_MASTER_SIZE = 1024
# iconutil's required iconset: base sizes in points, each at 1x and 2x.
ICON_BASE_SIZES = [16, 32, 128, 256, 512]
MINOS_PATTERN = re.compile(r"^\s*minos\s+(\S+)$", re.MULTILINE)


def run(*args: str) -> str:
    print("+", " ".join(args), flush=True)
    return subprocess.run(args, cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8").stdout


def package_version() -> str:
    with open(ROOT / "Cargo.toml", "rb") as f:
        return tomllib.load(f)["package"]["version"]


def minimum_macos(binary: Path) -> str:
    """The deployment target the binary was linked for (LC_BUILD_VERSION minos)."""
    found = MINOS_PATTERN.findall(run("otool", "-l", str(binary)))
    if len(set(found)) != 1:
        sys.exit(f"{binary}: expected one LC_BUILD_VERSION minos value, actual {found}")
    return found[0]


def build_icon(resources: Path) -> None:
    ICONSET.mkdir()
    run("sips", "-s", "format", "png", "-z", str(ICON_MASTER_SIZE), str(ICON_MASTER_SIZE),
        str(ICON_SOURCE), "--out", str(ICON_MASTER))
    for size in ICON_BASE_SIZES:
        for scale, suffix in ((1, ""), (2, "@2x")):
            px = str(size * scale)
            out = ICONSET / f"icon_{size}x{size}{suffix}.png"
            run("sips", "-z", px, px, str(ICON_MASTER), "--out", str(out))
    run("iconutil", "-c", "icns", str(ICONSET), "-o", str(resources / f"{ICON_NAME}.icns"))


def main() -> None:
    if sys.platform != "darwin":
        sys.exit(f"bundle_macos.py: expected platform darwin, actual {sys.platform}")
    cargo_trees.cargo("build", "--release", "--locked")
    cargo_trees.verify(["release"])
    binary = cargo_trees.target_dir() / "release" / EXECUTABLE
    version = package_version()

    if DIST.exists():
        shutil.rmtree(DIST)
    macos_dir = APP / "Contents" / "MacOS"
    resources = APP / "Contents" / "Resources"
    macos_dir.mkdir(parents=True)
    resources.mkdir(parents=True)
    shutil.copy2(binary, macos_dir / EXECUTABLE)
    build_icon(resources)
    shutil.rmtree(ICONSET)
    ICON_MASTER.unlink()

    info = {
        "CFBundleName": APP_NAME,
        "CFBundleDisplayName": APP_NAME,
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleExecutable": EXECUTABLE,
        "CFBundleIconFile": ICON_NAME,
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "CFBundlePackageType": "APPL",
        "CFBundleInfoDictionaryVersion": "6.0",
        "LSMinimumSystemVersion": minimum_macos(binary),
        "NSHighResolutionCapable": True,
        "LSUIElement": True,
    }
    with open(APP / "Contents" / "Info.plist", "wb") as f:
        plistlib.dump(info, f)

    run("codesign", "--force", "--deep", "-s", "-", str(APP))
    print(f"built {APP}")


if __name__ == "__main__":
    main()

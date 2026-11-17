# PlainPaste Implementation Plan

**Goal:** Build PlainPaste, a single-binary Rust tray utility for macOS, Windows and Linux (X11) that, on a global hotkey, pastes the clipboard as transformed plain text and optionally restores the original clipboard with all its formats.

**Architecture:** A library crate holds the testable core (`transform`, `config`) and the OS-facing services (`clipboard` with one trait and three native implementations, `hotkey`, `paste`). The binary crate holds the Slint flyout (`ui/flyout.slint`), the tray, the flyout window logic and the `App` state machine. Every event source (hotkey, tray, menu, clipboard provider, one-shot timer, UI callbacks) sends an `AppEvent` into one `async_channel`; one `slint::spawn_local` task receives the events in order on the Slint event loop, so handlers never nest and no global mutable state exists.

**Tech Stack:** Rust 1.94.1 (edition 2024) · Slint 1.18.1 (`fluent` style, winit 0.30 backend) · global-hotkey 0.10.0 · tray-icon 0.26.1 (`ksni` backend on Linux) · enigo 0.6.1 · objc2 0.6.5 + objc2-app-kit 0.3.2 + objc2-foundation 0.3.2 (macOS) · windows 0.62.2 (Windows) · x11rb 0.14.0 (Linux) · serde_json 1.0.154 · directories 6.0.0 · async-channel 2.5.0 · log 0.4.35 · env_logger 0.11.11.

**Verified versions (crates.io, 2026-11-14, `cargo search <crate> --limit 1`):** slint 1.18.1, slint-build 1.18.1, global-hotkey 0.10.0, tray-icon 0.26.1, enigo 0.6.1, objc2 0.6.5, objc2-app-kit 0.3.2, objc2-foundation 0.3.2, windows 0.62.2, x11rb 0.14.0, serde 1.0.229 (not a direct dependency: config uses `serde_json::Value`), serde_json 1.0.154, directories 6.0.0, async-channel 2.5.0, log 0.4.35, env_logger 0.11.11. Resolved transitive versions that the code relies on: winit 0.30.13, muda 0.21.2, ksni 0.3.7.

**API verification:** Every crate API in this plan was checked against the fetched crate source (`~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/`) and, for slint, tray-icon, global-hotkey and enigo, also through context7. All macOS-side code (Tasks 1, 2, 3, 6, 7, 8) was compiled with `cargo clippy --all-targets` (zero warnings) in a scratch copy; the X11 code (Task 5) was type-checked on macOS (x11rb is pure Rust); the Windows code (Task 4) was checked against the `windows` 0.62.2 source only, because the `windows` crate does not compile on macOS.

**Decisions (hard-to-reverse only):**
- *Event routing.* Alternatives: (a) one `async_channel` + one `slint::spawn_local` pump that owns `App`; (b) `Rc<RefCell<App>>` captured by every Slint callback plus `invoke_from_event_loop` hops from other threads; (c) a `thread_local!` app. Drivers: hotkey, tray, menu and the X11 provider call back on non-main threads with `Send` closures; macOS and Windows provider callbacks can arrive while UI code runs; the rules forbid global mutable state. Chosen: (a). Drawback: one more dependency (`async-channel`) and every UI intent becomes an `AppEvent` variant.
- *Linux tray backend.* Alternatives: `libappindicator` (default; needs a GTK main loop on its own thread) or `ksni` (D-Bus StatusNotifierItem, own worker thread, emits a left-click event with an all-zero rect). Drivers: no GTK thread, no second event loop, no Linux-only code path in `tray.rs`. Chosen: `ksni`. Drawback: needs a StatusNotifierWatcher (KDE, or GNOME with the AppIndicator extension); the flyout opens centered on Linux because the rect is zero.
- *Crate split.* `src/lib.rs` exposes `clipboard`, `config`, `hotkey`, `paste`, `transform`; the binary owns UI, tray and `App`. Driver: integration tests and the per-OS clipboard probes (scratch crates under `tmp/`) link the library without the UI. Drawback: the library depends on slint (for `slint::platform::Key` in the recorder key mapping).

## Global Constraints

- Dependency discipline: write no crate API from memory. Use exactly the versions in `Cargo.toml` from Task 1. Before changing any third-party call, read that crate's source under `~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/` or query context7.
- Platform divergence: the `clipboard` module (one `Clipboard` trait, `cfg(target_os)` implementations in `src/clipboard/{macos,windows,x11}.rs`) is the only module with per-OS code paths. Spec-mandated per-OS values (default hotkey Cmd vs Ctrl, paste modifier Cmd vs Ctrl, Slint's Cmd/Ctrl modifier swap, the macOS-only Accessibility card, the Cmd/Win keycap label) are chosen with `cfg!(target_os = "macos")` expressions inside shared functions, never with `#[cfg]` items.
- No sleep, no polling, no time-based waiting. Use event and callback push only. Exceptions: `PASTE_READ_TIMEOUT` = 2 s (fail-fast bound, armed through the injectable `OneShotTimer` trait) and `SELECTION_REPLY_TIMEOUT` = 2 s in the X11 implementation (fail-fast IO bound on a reply from another X client; flagged for approval).
- No swallowed errors and no silent fallback. Every error is returned or logged with `log::error!`. Error messages state the expected value, the actual value and the affected object (file path, field, clipboard format, hotkey).
- Tests: commit only the `transform` and `config` tests (`tests/transform.rs`, `tests/config.rs`). No UI, e2e or platform tests in the suite. Each test must take < 1 s; the whole suite ≤ 40 s.
- Test entry point: `python3 scripts/test.py`. It builds the test binaries, runs every test in parallel, fails when a test exceeds 1 s or the run exceeds 40 s and lists the 5 slowest, fails when `tests/` contains `sleep`, `park_timeout`, `recv_timeout` or `wait_timeout`, fails when zero tests run, and prints the skip (ignored) count. Last line `RESULT: PASS` or `RESULT: FAIL`.
- No dead code, no commented-out code, no unowned deferred-work markers, named constants for magic values. `cargo clippy --all-targets` must print zero warnings on the OS you build on.
- Scratch and probe files go only under `scratch/` in the repo (gitignored). Never use `/tmp`, `/private/tmp` or the system temp dir.
- Git (every commit step): `git add <explicit new paths>` then `git commit -m "<subject>" -- <paths>`. Never `git add -A`, `git stash`, `git reset`, `git checkout --`, `git clean`. No `;` inside `-m` text. Run `git rev-parse --abbrev-ref HEAD` before and `git status --porcelain` after each commit.
- Shared file `src/lib.rs`: Tasks 2, 3 and 7 each add one `pub mod` line. Commit them one at a time.
- UI copy is Traditional Chinese and is copied verbatim from `design/mockups/variant-b.html`. Colors and sizes are the mockup's Fluent 2 tokens.
- Windows host for verification: `ssh -i <ssh-key> <windows-user>@<windows-host>`. Its default shell is cmd.exe; run multi-part commands as `powershell -Command "a; b"`, never with `&`.
- Linux verification runs in Docker (`docker` 29.8.2 is installed, linux/arm64) on the `rust:1.94-bookworm` image with Xvfb. Wayland is out of scope.

## File Structure

| Path | Responsibility | Task |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | Package `plainpaste` 0.0.1, all dependencies | 1 |
| `scripts/test.py` | Test entry point with time and sleep budgets | 1 |
| `src/lib.rs` | Library module list | 1 (then 2, 3, 7 add lines) |
| `src/transform.rs`, `tests/transform.rs` | `normalize_line_breaks()` and its tests | 1 |
| `src/config.rs`, `tests/config.rs` | `Settings`, parse/serialize/load/save and tests | 2 |
| `src/clipboard/mod.rs` | `Clipboard` trait, `ClipboardError`, `OnRead`, `PlatformClipboard` alias | 3 |
| `src/clipboard/macos.rs` | NSPasteboard implementation | 3 |
| `src/clipboard/windows.rs` | Win32 delayed-rendering implementation | 4 |
| `src/clipboard/x11.rs` | X11 selection-owner implementation | 5 |
| `ui/flyout.slint`, `build.rs` | Flyout UI per mockup, Slint build with `fluent` style | 6 |
| `src/hotkey.rs` | `HotkeyService` | 7 |
| `src/app/recorder.rs` | Recorder key mapping, keycap labels | 7 |
| `src/paste.rs` | Simulated paste, Accessibility check, System Settings shortcut | 7 |
| `src/app/flyout.rs` | Flyout placement, focus-loss hook, theme switch | 8 |
| `src/app/tray.rs` | Tray icon, menu, click forwarding | 8 |
| `src/app/mod.rs` | `AppEvent`, `EventSender`, `OneShotTimer`, `App`, paste flow, event pump | 8 |
| `src/main.rs` | Startup and shutdown | 8 |

## Task Dependencies

| Task | Depends on | Can run in parallel with |
|---|---|---|
| 1 Scaffold, transform, test runner | none | none |
| 2 Config | 1 | 3, 6, 7 |
| 3 Clipboard trait + macOS | 1 | 2, 6, 7 |
| 4 Windows clipboard | 3 | 5, 8 |
| 5 X11 clipboard | 3 | 4, 8 |
| 6 Flyout UI | 1 | 2, 3, 7 |
| 7 Hotkey and paste services | 1 | 2, 3, 6 |
| 8 App integration (macOS end to end) | 2, 3, 6, 7 | 4, 5 |
| 9 Windows and Linux end to end | 4, 5, 8 | none |

## Spec Coverage Notes (decisions the implementer cannot derive)

- The mockup tag "僅 macOS 顯示" is a design annotation; the real UI hides the whole Accessibility card on Windows and Linux instead of showing the tag.
- The mockup hint "點擊後按下新的組合鍵。macOS 顯示為 Cmd+Shift+V。" keeps only its first sentence; the second sentence explains the mockup.
- Linux: the tray menu item 設定 opens the centered flyout (spec). The KSNI backend also reports a left click with an all-zero rect; that click opens the same centered flyout.
- The tray menu (設定, 結束) exists on every OS; on macOS and Windows a left click opens the anchored flyout and a right click opens the menu.
- macOS snapshot: a pasteboard type whose `dataForType` returns nil (for example `public.utf16-external-plain-text`, a synthesized conversion type) is logged at info level and not saved; the pasteboard re-synthesizes it from the restored source types. Verified live on macOS 15.
- Windows snapshot: GDI-handle formats (`CF_BITMAP`, `CF_METAFILEPICT`, `CF_PALETTE`, `CF_OWNERDISPLAY`, `CF_DSP*`) are logged and not saved; Windows synthesizes `CF_BITMAP` from `CF_DIB` and `CF_METAFILEPICT` from `CF_ENHMETAFILE`. `CF_ENHMETAFILE` is saved through `GetEnhMetaFileBits`.
- X11: INCR (large, incremental) transfers are not supported. Reading an INCR target aborts the paste with an error; serving data larger than the maximum request size refuses that target with an error log.
- A second hotkey press while a restore is pending first restores the pending snapshot, then starts the new paste.
- Quitting while a restore is pending restores the snapshot first.
- The hotkey is unregistered while the recorder is recording (otherwise the OS consumes the keys before the flyout sees them) and is registered again on cancel, on failure, or as the new combo on success.

---

### Task 1: Scaffold, transform and test runner

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/transform.rs`
- Create: `scripts/test.py`
- Test: `tests/transform.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - Package `plainpaste` 0.0.1, library crate name `plainpaste`, with every dependency of the project (later tasks never edit `Cargo.toml`).
  - `pub fn plainpaste::transform::normalize_line_breaks(text: &str, keep_line_breaks: bool) -> String`
  - `python3 scripts/test.py` → exit 0 and last line `RESULT: PASS` when every budget holds.

**Behavior:**
Line breaks `\r\n`, `\r` and `\n` are first normalized to `\n` (`\r\n` counts as one break). With `keep_line_breaks = true` every break is kept as `\n` (a run of N breaks stays N `\n`). With `keep_line_breaks = false` each run of consecutive breaks (any mix of the three forms) becomes exactly one space. Nothing is trimmed: leading and trailing runs become one space each. The empty string maps to the empty string. Other whitespace (tabs, spaces) is unchanged.

`scripts/test.py` builds the test binaries with `cargo test --no-run --message-format=json`, lists tests with `<bin> --list --format terse` (and `--ignored` for the skip count), runs each test as `<bin> --exact <name> --test-threads=1` in a thread pool sized to the CPU count, and measures wall time per test and for the whole run phase (the build phase is excluded).

**Constraints:**
- `transform` uses std only, one pass over `chars()`, constants for `'\n'`, `'\r'` and the replacement `' '`.
- `scripts/test.py` uses the Python standard library only.
- Layer: library, no dependency on other project modules.

**Acceptance criteria:**
- [ ] `python3 scripts/test.py` prints `tests run: 8  skipped: 0` and `RESULT: PASS`, exit code 0.
- [ ] `tests/transform.rs` contains the 8 tests named in Step 1 and all pass.
- [ ] Red check (scratch, not committed): a temporary `tests/redprobe.rs` with a test that blocks 1.1 s and an `#[ignore]` test makes `scripts/test.py` print `SLOW redprobe-…::slow`, `sleep-like word in test dir: tests/redprobe.rs:2`, `skipped: 1`, `RESULT: FAIL` and exit 1; the file is deleted afterwards.
- [ ] `cargo clippy --all-targets` prints no `warning` line.

**Steps:**

- [ ] **Step 1: Write the manifest, the library root and the failing transform tests**

`Cargo.toml` (complete; later tasks do not edit it):

```toml
[package]
name = "plainpaste"
version = "0.0.1"
edition = "2024"
rust-version = "1.94"

[dependencies]
async-channel = "2.5.0"
directories = "6.0.0"
enigo = "0.6.1"
env_logger = "0.11.11"
global-hotkey = "0.10.0"
log = "0.4.35"
serde_json = "1.0.154"
slint = { version = "1.18.1", features = ["unstable-winit-030"] }
tray-icon = { version = "0.26.1", default-features = false, features = ["ksni"] }

[build-dependencies]
slint-build = "1.18.1"

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6.5"
objc2-app-kit = { version = "0.3.2", default-features = false, features = ["std", "NSPasteboard", "NSPasteboardItem"] }
objc2-foundation = { version = "0.3.2", default-features = false, features = ["std", "NSArray", "NSData", "NSString", "NSObject"] }

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62.2", features = ["Win32_Foundation", "Win32_Graphics_Gdi", "Win32_System_DataExchange", "Win32_System_LibraryLoader", "Win32_System_Memory", "Win32_System_Ole", "Win32_UI_WindowsAndMessaging"] }

[target.'cfg(target_os = "linux")'.dependencies]
x11rb = "0.14.0"
```

`src/lib.rs` (Tasks 2, 3 and 7 add their modules later):

```rust
pub mod transform;
```

`tests/transform.rs`:

```rust
use plainpaste::transform::normalize_line_breaks;

#[test]
fn empty_string_stays_empty() {
    assert_eq!(normalize_line_breaks("", false), "");
    assert_eq!(normalize_line_breaks("", true), "");
}

#[test]
fn text_without_breaks_is_unchanged() {
    assert_eq!(normalize_line_breaks("a b\tc", false), "a b\tc");
    assert_eq!(normalize_line_breaks("a b\tc", true), "a b\tc");
}

#[test]
fn keep_on_normalizes_crlf_and_cr_to_lf() {
    assert_eq!(normalize_line_breaks("a\r\nb\rc\nd", true), "a\nb\nc\nd");
}

#[test]
fn keep_on_preserves_every_break_in_a_run() {
    assert_eq!(normalize_line_breaks("a\r\n\r\nb", true), "a\n\nb");
}

#[test]
fn keep_off_replaces_single_breaks_with_space() {
    assert_eq!(
        normalize_line_breaks("一行\n二行\r\n三行\r四行", false),
        "一行 二行 三行 四行"
    );
}

#[test]
fn keep_off_collapses_mixed_runs_to_one_space() {
    assert_eq!(normalize_line_breaks("a\r\n\n\r\r\nb", false), "a b");
}

#[test]
fn keep_off_keeps_leading_and_trailing_runs_as_one_space() {
    assert_eq!(normalize_line_breaks("\n\na\r\n", false), " a ");
}

#[test]
fn crlf_counts_as_one_break_not_two() {
    assert_eq!(normalize_line_breaks("a\r\nb", true), "a\nb");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test transform 2>&1 | grep -E "^error" | head -3`
Expected: `error[E0583]: file not found for module `transform`` (the first build also compiles every dependency; it takes several minutes once. If it exceeds your foreground tool limit, run it detached and wait for it).

- [ ] **Step 3: Implement `transform`**

`src/transform.rs`:

```rust
//! Plain-text transform applied to clipboard text before it is pasted.

const LINE_FEED: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const LINE_BREAK_REPLACEMENT: char = ' ';

/// Normalizes every line break (`\r\n`, `\r`, `\n`) to `\n`.
/// When `keep_line_breaks` is false, each run of consecutive line breaks becomes one space.
pub fn normalize_line_breaks(text: &str, keep_line_breaks: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_break_run = false;
    while let Some(c) = chars.next() {
        let is_break = c == LINE_FEED || c == CARRIAGE_RETURN;
        if !is_break {
            in_break_run = false;
            out.push(c);
            continue;
        }
        if c == CARRIAGE_RETURN && chars.peek() == Some(&LINE_FEED) {
            chars.next();
        }
        if keep_line_breaks {
            out.push(LINE_FEED);
        } else if !in_break_run {
            out.push(LINE_BREAK_REPLACEMENT);
        }
        in_break_run = true;
    }
    out
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test transform 2>&1 | grep "test result"`
Expected: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; …`

- [ ] **Step 5: Write the test entry point**

`scripts/test.py`:

```python
#!/usr/bin/env python3
"""Single test entry point: builds the cargo test binaries, runs every test
in parallel with a per-test and a total time budget, and enforces the
no-sleep rule on the test directory."""

import json
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

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
    proc = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"cargo test --no-run failed with exit code {proc.returncode}")
    binaries = []
    for line in proc.stdout.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg.get("profile", {}).get("test") and msg.get("executable"):
            binaries.append(msg["executable"])
    return binaries


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

    binaries = build_test_binaries()
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
```

- [ ] **Step 6: Run the entry point**

Run: `python3 scripts/test.py; echo RC=$?`
Expected: a line `tests run: 8  skipped: 0  total: …s`, the 5 slowest tests, `RESULT: PASS`, `RC=0`.

- [ ] **Step 7: Prove the budgets turn red (scratch, not committed)**

Create `tests/redprobe.rs` with the Write tool:

```rust
#[test]
fn slow() { std::thread::sleep(std::time::Duration::from_millis(1100)); }
#[test]
#[ignore]
fn skipped() {}
```

Run: `python3 scripts/test.py > tmp/redprobe.log; echo RC=$?; grep -E "SLOW|sleep-like|skipped|RESULT" tmp/redprobe.log`
Expected: `RC=1`, then lines containing `skipped: 1`, `sleep-like word in test dir: tests/redprobe.rs:2`, `SLOW redprobe-` … `expected <= 1.0s`, `RESULT: FAIL`.
Then delete the probe: `rm tests/redprobe.rs tmp/redprobe.log` and rerun `python3 scripts/test.py | tail -1` → `RESULT: PASS`.

- [ ] **Step 8: Lint**

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning"`
Expected: `0`

- [ ] **Step 9: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add Cargo.toml Cargo.lock src/lib.rs src/transform.rs tests/transform.rs scripts/test.py
git commit -m "feat: add transform and test entry point" -- Cargo.toml Cargo.lock src/lib.rs src/transform.rs tests/transform.rs scripts/test.py
git status --porcelain
```

Expected: the last command prints nothing for these paths.

### Task 2: Config

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs` (add `pub mod config;`)
- Test: `tests/config.rs`

**Interfaces:**
- Consumes: `global_hotkey::hotkey::{Code, HotKey, Modifiers}` (crate), `serde_json`, `directories`, `log`.
- Produces (module `plainpaste::config`):
  - `pub enum Theme { System, Light, Dark }` with `pub fn as_str(self) -> &'static str` (`"system"`, `"light"`, `"dark"`) and `pub fn parse(value: &str) -> Option<Theme>`; derives `Debug, Clone, Copy, PartialEq, Eq`.
  - `pub struct Settings { pub hotkey: HotKey, pub keep_line_breaks: bool, pub restore_clipboard: bool, pub theme: Theme }`; derives `Debug, Clone, Copy, PartialEq, Eq`; `impl Default` (default hotkey, `false`, `true`, `Theme::System`).
  - `pub fn default_hotkey() -> HotKey` — Cmd+Shift+V (`Modifiers::META | SHIFT`, `Code::KeyV`) on macOS, Ctrl+Shift+V (`Modifiers::CONTROL | SHIFT`) elsewhere.
  - `pub enum ConfigError { Invalid { path: PathBuf, field: String, expected: String, actual: String }, Io { path: PathBuf, op: &'static str, source: io::Error }, NoConfigDir }`; implements `Display` and `std::error::Error`.
  - `pub fn parse_settings(text: &str, path: &Path) -> Result<Settings, ConfigError>`
  - `pub fn serialize_settings(settings: &Settings) -> String`
  - `pub fn settings_path() -> Result<PathBuf, ConfigError>`
  - `pub fn load_or_create(path: &Path) -> Result<Settings, ConfigError>`
  - `pub fn save(path: &Path, settings: &Settings) -> Result<(), ConfigError>`

**Behavior:**
The file is JSON: `{"hotkey": "<global-hotkey string>", "keep_line_breaks": bool, "restore_clipboard": bool, "theme": "system"|"light"|"dark"}`. The hotkey string is global-hotkey's own format (`HotKey::into_string()`, for example `shift+meta+KeyV`; `meta` is Cmd on macOS and Win on Windows) and is parsed with `HotKey::from_str`.
`parse_settings` parses once into `serde_json::Value`, then checks fields by hand so each error names the field:
- not JSON → field `<json>`, expected `a valid JSON document`, actual = serde_json's message (with line and column);
- root not an object → field `<root>`, expected `a JSON object`, actual = the value;
- an unknown key → field = that key, expected `one of hotkey, keep_line_breaks, restore_clipboard, theme`, actual `an unknown field`;
- a missing key → actual `missing`;
- wrong type or value → actual = the JSON value as text;
- a hotkey string that does not parse → actual = `"<value>" (<global-hotkey error>)`; a hotkey without modifiers → actual = `"<value>" (no modifier)`.
`Display` for `Invalid`: `config file <path>: field `<field>`: expected <expected>, actual <actual>`.
`settings_path()` = `directories::ProjectDirs::from("", "", "PlainPaste")?.config_dir()/settings.json`.
`load_or_create`: missing file → write defaults with `save` and return them (log info); any read error other than NotFound → `ConfigError::Io { op: "read" }`; content that fails to parse → the `Invalid` error, and the file is not touched. The caller (Task 8 `main`) stops startup on any error.
`save` writes `<path>.tmp` (same directory, `with_extension("tmp")`) then renames it over the file, so a crash never leaves a half-written file; it creates the parent directory first.

**Constraints:**
- No `serde` derive: use `serde_json::Value` and `serde_json::json!`.
- Field names, theme strings and the file name are named constants.
- Layer: library; depends on global-hotkey, serde_json, directories, log only.

**Acceptance criteria:**
- [ ] `tests/config.rs` has the 10 tests named in Step 1; `python3 scripts/test.py` prints `tests run: 18` and `RESULT: PASS`.
- [ ] `grep -n "derive(.*Deserialize" src/config.rs` returns nothing.
- [ ] `cargo clippy --all-targets` prints no `warning` line.

**Steps:**

- [ ] **Step 1: Write the failing config tests and register the module**

Add one line to `src/lib.rs` (keep the existing lines, keep the list sorted):

```rust
pub mod config;
```

`tests/config.rs`:

```rust
use std::path::Path;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use plainpaste::config::{ConfigError, Settings, Theme, parse_settings, serialize_settings};

const PATH: &str = "/cfg/settings.json";

fn parse(text: &str) -> Result<Settings, ConfigError> {
    parse_settings(text, Path::new(PATH))
}

fn expect_invalid(text: &str, want_field: &str, want_actual_contains: &str) {
    match parse(text) {
        Err(ConfigError::Invalid {
            path,
            field,
            expected,
            actual,
        }) => {
            assert_eq!(path, Path::new(PATH));
            assert_eq!(field, want_field);
            assert!(!expected.is_empty());
            assert!(
                actual.contains(want_actual_contains),
                "actual `{actual}` should contain `{want_actual_contains}`"
            );
        }
        other => panic!("expected ConfigError::Invalid for field {want_field}, got {other:?}"),
    }
}

#[test]
fn parses_valid_document() {
    let text = r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":true,"restore_clipboard":false,"theme":"dark"}"#;
    let settings = parse(text).expect("valid document");
    assert_eq!(
        settings,
        Settings {
            hotkey: HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
            keep_line_breaks: true,
            restore_clipboard: false,
            theme: Theme::Dark,
        }
    );
}

#[test]
fn serialize_then_parse_round_trips_defaults() {
    let settings = Settings::default();
    assert_eq!(
        parse(&serialize_settings(&settings)).expect("round trip"),
        settings
    );
}

#[test]
fn rejects_malformed_json() {
    expect_invalid("{", "<json>", "EOF");
}

#[test]
fn rejects_non_object_root() {
    expect_invalid("[]", "<root>", "[]");
}

#[test]
fn rejects_wrong_bool_type() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":"yes","restore_clipboard":true,"theme":"system"}"#,
        "keep_line_breaks",
        "\"yes\"",
    );
}

#[test]
fn rejects_missing_field() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"theme":"system"}"#,
        "restore_clipboard",
        "missing",
    );
}

#[test]
fn rejects_unknown_theme() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"blue"}"#,
        "theme",
        "\"blue\"",
    );
}

#[test]
fn rejects_unknown_field() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"system","extra":1}"#,
        "extra",
        "unknown",
    );
}

#[test]
fn rejects_unparsable_hotkey() {
    expect_invalid(
        r#"{"hotkey":"shift+KeyV+alt","keep_line_breaks":false,"restore_clipboard":true,"theme":"system"}"#,
        "hotkey",
        "shift+KeyV+alt",
    );
}

#[test]
fn rejects_hotkey_without_modifier() {
    expect_invalid(
        r#"{"hotkey":"KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"system"}"#,
        "hotkey",
        "no modifier",
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test config 2>&1 | grep -E "^error" | head -3`
Expected: `error[E0583]: file not found for module `config``

- [ ] **Step 3: Implement the config module**

`src/config.rs`:

```rust
//! Settings: parsed once at the boundary into typed values, saved atomically.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use serde_json::{Map, Value};

const APP_DIR_NAME: &str = "PlainPaste";
const CONFIG_FILE_NAME: &str = "settings.json";
const TEMP_SUFFIX: &str = "tmp";

const FIELD_HOTKEY: &str = "hotkey";
const FIELD_KEEP_LINE_BREAKS: &str = "keep_line_breaks";
const FIELD_RESTORE_CLIPBOARD: &str = "restore_clipboard";
const FIELD_THEME: &str = "theme";
const KNOWN_FIELDS: [&str; 4] = [
    FIELD_HOTKEY,
    FIELD_KEEP_LINE_BREAKS,
    FIELD_RESTORE_CLIPBOARD,
    FIELD_THEME,
];
const ROOT_FIELD: &str = "<root>";
const JSON_FIELD: &str = "<json>";

const THEME_SYSTEM: &str = "system";
const THEME_LIGHT: &str = "light";
const THEME_DARK: &str = "dark";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => THEME_SYSTEM,
            Theme::Light => THEME_LIGHT,
            Theme::Dark => THEME_DARK,
        }
    }

    pub fn parse(value: &str) -> Option<Theme> {
        match value {
            THEME_SYSTEM => Some(Theme::System),
            THEME_LIGHT => Some(Theme::Light),
            THEME_DARK => Some(Theme::Dark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub hotkey: HotKey,
    pub keep_line_breaks: bool,
    pub restore_clipboard: bool,
    pub theme: Theme,
}

/// Cmd+Shift+V on macOS, Ctrl+Shift+V elsewhere (spec: Settings).
pub fn default_hotkey() -> HotKey {
    let primary = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    HotKey::new(Some(primary | Modifiers::SHIFT), Code::KeyV)
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey: default_hotkey(),
            keep_line_breaks: false,
            restore_clipboard: true,
            theme: Theme::System,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    /// The file content does not match the schema.
    Invalid {
        path: PathBuf,
        field: String,
        expected: String,
        actual: String,
    },
    /// A filesystem operation on the config file failed.
    Io {
        path: PathBuf,
        op: &'static str,
        source: io::Error,
    },
    /// The OS reports no home or config directory.
    NoConfigDir,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Invalid {
                path,
                field,
                expected,
                actual,
            } => write!(
                f,
                "config file {}: field `{field}`: expected {expected}, actual {actual}",
                path.display()
            ),
            ConfigError::Io { path, op, source } => {
                write!(f, "config file {}: {op} failed: {source}", path.display())
            }
            ConfigError::NoConfigDir => write!(
                f,
                "config directory: expected a platform config directory, actual none (no home directory)"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

fn invalid(path: &Path, field: &str, expected: &str, actual: String) -> ConfigError {
    ConfigError::Invalid {
        path: path.to_path_buf(),
        field: field.to_string(),
        expected: expected.to_string(),
        actual,
    }
}

fn require<'a>(
    obj: &'a Map<String, Value>,
    path: &Path,
    field: &str,
    expected: &str,
) -> Result<&'a Value, ConfigError> {
    obj.get(field)
        .ok_or_else(|| invalid(path, field, expected, "missing".to_string()))
}

fn parse_bool(obj: &Map<String, Value>, path: &Path, field: &str) -> Result<bool, ConfigError> {
    const EXPECTED: &str = "true or false";
    let value = require(obj, path, field, EXPECTED)?;
    value
        .as_bool()
        .ok_or_else(|| invalid(path, field, EXPECTED, value.to_string()))
}

fn parse_hotkey(obj: &Map<String, Value>, path: &Path) -> Result<HotKey, ConfigError> {
    const EXPECTED: &str = "a hotkey string with at least one modifier, like \"shift+meta+KeyV\"";
    let value = require(obj, path, FIELD_HOTKEY, EXPECTED)?;
    let text = value
        .as_str()
        .ok_or_else(|| invalid(path, FIELD_HOTKEY, EXPECTED, value.to_string()))?;
    let hotkey: HotKey = text
        .parse()
        .map_err(|e| invalid(path, FIELD_HOTKEY, EXPECTED, format!("{value} ({e})")))?;
    if hotkey.mods.is_empty() {
        return Err(invalid(
            path,
            FIELD_HOTKEY,
            EXPECTED,
            format!("{value} (no modifier)"),
        ));
    }
    Ok(hotkey)
}

fn parse_theme(obj: &Map<String, Value>, path: &Path) -> Result<Theme, ConfigError> {
    const EXPECTED: &str = "one of \"system\", \"light\", \"dark\"";
    let value = require(obj, path, FIELD_THEME, EXPECTED)?;
    value
        .as_str()
        .and_then(Theme::parse)
        .ok_or_else(|| invalid(path, FIELD_THEME, EXPECTED, value.to_string()))
}

/// Parses the JSON settings document. `path` is used only in error messages.
pub fn parse_settings(text: &str, path: &Path) -> Result<Settings, ConfigError> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| invalid(path, JSON_FIELD, "a valid JSON document", e.to_string()))?;
    let obj = root
        .as_object()
        .ok_or_else(|| invalid(path, ROOT_FIELD, "a JSON object", root.to_string()))?;
    if let Some(unknown) = obj.keys().find(|k| !KNOWN_FIELDS.contains(&k.as_str())) {
        return Err(invalid(
            path,
            unknown,
            &format!("one of {}", KNOWN_FIELDS.join(", ")),
            "an unknown field".to_string(),
        ));
    }
    Ok(Settings {
        hotkey: parse_hotkey(obj, path)?,
        keep_line_breaks: parse_bool(obj, path, FIELD_KEEP_LINE_BREAKS)?,
        restore_clipboard: parse_bool(obj, path, FIELD_RESTORE_CLIPBOARD)?,
        theme: parse_theme(obj, path)?,
    })
}

pub fn serialize_settings(settings: &Settings) -> String {
    let doc = serde_json::json!({
        FIELD_HOTKEY: settings.hotkey.into_string(),
        FIELD_KEEP_LINE_BREAKS: settings.keep_line_breaks,
        FIELD_RESTORE_CLIPBOARD: settings.restore_clipboard,
        FIELD_THEME: settings.theme.as_str(),
    });
    serde_json::to_string_pretty(&doc).expect("a serde_json::Value always serializes")
}

/// `<platform config dir>/PlainPaste/settings.json`.
pub fn settings_path() -> Result<PathBuf, ConfigError> {
    directories::ProjectDirs::from("", "", APP_DIR_NAME)
        .map(|dirs| dirs.config_dir().join(CONFIG_FILE_NAME))
        .ok_or(ConfigError::NoConfigDir)
}

/// Loads the settings file. A missing file is created with the defaults.
/// A file that fails to parse is an error and is never overwritten.
pub fn load_or_create(path: &Path) -> Result<Settings, ConfigError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_settings(&text, path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let settings = Settings::default();
            save(path, &settings)?;
            log::info!("created config file {} with defaults", path.display());
            Ok(settings)
        }
        Err(source) => Err(ConfigError::Io {
            path: path.to_path_buf(),
            op: "read",
            source,
        }),
    }
}

/// Writes to a sibling temp file, then renames it over `path`.
pub fn save(path: &Path, settings: &Settings) -> Result<(), ConfigError> {
    let io_err = |op: &'static str| {
        move |source| ConfigError::Io {
            path: path.to_path_buf(),
            op,
            source,
        }
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_err("create directory"))?;
    }
    let temp = path.with_extension(TEMP_SUFFIX);
    fs::write(&temp, serialize_settings(settings)).map_err(io_err("write temp file"))?;
    fs::rename(&temp, path).map_err(io_err("rename temp file"))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test config 2>&1 | grep "test result"`
Expected: `test result: ok. 10 passed; 0 failed; …`

Run: `python3 scripts/test.py | grep -E "tests run|RESULT"`
Expected: `tests run: 18  skipped: 0 …` and `RESULT: PASS`.

- [ ] **Step 5: Lint and check the no-derive rule**

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning"; grep -c "Deserialize" src/config.rs`
Expected: `0` and `0`.

- [ ] **Step 6: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add src/config.rs tests/config.rs
git commit -m "feat: add settings parsing and atomic save" -- src/config.rs tests/config.rs src/lib.rs
git status --porcelain
```

### Task 3: Clipboard trait and macOS implementation

**Files:**
- Create: `src/clipboard/mod.rs`
- Create: `src/clipboard/macos.rs`
- Modify: `src/lib.rs` (add `pub mod clipboard;`)
- Scratch (not committed): `tmp/clipboard-probe/Cargo.toml`, `tmp/clipboard-probe/src/main.rs`

**Interfaces:**
- Consumes: `objc2`, `objc2-app-kit`, `objc2-foundation`, `log`.
- Produces (module `plainpaste::clipboard`):
  - `pub type OnRead = Box<dyn FnOnce() + Send + 'static>;`
  - `pub struct ClipboardError { pub op: &'static str, pub detail: String }` with `pub fn new(op: &'static str, detail: impl Into<String>) -> Self`; `Display` = `clipboard <op> failed: <detail>`; implements `std::error::Error`; derives `Debug`.
  - `pub trait Clipboard { type Snapshot; fn read_text(&mut self) -> Result<Option<String>, ClipboardError>; fn write_text(&mut self, text: &str) -> Result<(), ClipboardError>; fn snapshot(&mut self) -> Result<Self::Snapshot, ClipboardError>; fn offer_lazy_text(&mut self, text: String, on_read: OnRead) -> Result<(), ClipboardError>; fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), ClipboardError>; }`
  - `pub use <os>::<Impl> as PlatformClipboard;` — `macos::MacosClipboard`, `windows::WindowsClipboard` (Task 4), `x11::X11Clipboard` (Task 5). Every implementation has `pub fn new() -> Result<Self, ClipboardError>`.
  - macOS: `pub struct MacosClipboard`, `pub struct MacosSnapshot` (opaque).

**Behavior:**
`read_text` returns `None` when the clipboard has no plain-text representation. `write_text` replaces the clipboard with plain text. `snapshot` copies every format of every pasteboard item. `offer_lazy_text` clears the clipboard and offers `text` as plain text without providing bytes; when a target first asks, the implementation provides the bytes and calls `on_read` exactly once (later reads get the text but no second call). `on_read` may run on any thread. `restore` replaces the clipboard with the snapshot and drops any lazy offer.
macOS: `NSPasteboard::generalPasteboard()`. The lazy offer is an `NSPasteboardItem` whose data provider (`TextProvider`, defined with `objc2::define_class!`, Objective-C name `PlainPasteTextProvider`, implementing `NSPasteboardItemDataProvider`) answers `pasteboard:item:provideDataForType:` with `setString:forType:`; `MacosClipboard` keeps the provider alive until the next write or restore. A type whose `dataForType` returns nil is logged and skipped (see Spec Coverage Notes). Every `NO` returned by AppKit becomes a `ClipboardError` naming the method, the item index and the type.

**Constraints:**
- This module is the only one with `#[cfg(target_os)]` items.
- Every `unsafe` block has a `// SAFETY:` comment.
- Layer: library; depends on objc2, objc2-app-kit, objc2-foundation, log only.

**Acceptance criteria:**
- [ ] `cargo clippy --all-targets` on macOS prints no `warning` line.
- [ ] Probe on macOS (Step 5) prints `reader read: "LAZY OUT"`, `on_read fired: true`, `restored`, an `after restore:` line that lists `«class RTF »`, and exits 0.
- [ ] `python3 scripts/test.py` still ends with `RESULT: PASS`.

**Steps:**

No committed test: platform clipboard code is outside the committed-test scope (Global Constraints). The red-to-green evidence is the scratch probe in Steps 4–5.

- [ ] **Step 1: Write the trait module and register it**

Add one line to `src/lib.rs` (keep the list sorted):

```rust
pub mod clipboard;
```

`src/clipboard/mod.rs` (it already names the Windows and X11 modules of Tasks 4 and 5; they compile only on their own OS):

```rust
//! One clipboard interface, one native implementation per OS.
//! This module is the only approved platform divergence.

use std::fmt;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "macos")]
pub use macos::MacosClipboard as PlatformClipboard;
#[cfg(target_os = "windows")]
pub use windows::WindowsClipboard as PlatformClipboard;
#[cfg(target_os = "linux")]
pub use x11::X11Clipboard as PlatformClipboard;

/// Called once, on any thread, when the paste target first reads the lazily offered text.
pub type OnRead = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug)]
pub struct ClipboardError {
    /// The clipboard operation that failed, for example "snapshot".
    pub op: &'static str,
    /// Platform API name, error code and the affected format or item.
    pub detail: String,
}

impl ClipboardError {
    pub fn new(op: &'static str, detail: impl Into<String>) -> Self {
        ClipboardError {
            op,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clipboard {} failed: {}", self.op, self.detail)
    }
}

impl std::error::Error for ClipboardError {}

pub trait Clipboard {
    /// Every format that was on the clipboard, with its bytes.
    type Snapshot;

    /// The plain-text representation, or `None` when the clipboard has no text.
    fn read_text(&mut self) -> Result<Option<String>, ClipboardError>;

    /// Replaces the clipboard content with `text` as plain text.
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// Copies every format currently on the clipboard.
    fn snapshot(&mut self) -> Result<Self::Snapshot, ClipboardError>;

    /// Replaces the clipboard content with a lazy plain-text offer.
    /// The OS asks for the bytes only when a target reads them. The first read calls `on_read`.
    fn offer_lazy_text(&mut self, text: String, on_read: OnRead) -> Result<(), ClipboardError>;

    /// Replaces the clipboard content with `snapshot`.
    fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), ClipboardError>;
}
```

- [ ] **Step 2: Verify the build fails without the macOS implementation**

Run: `cargo build 2>&1 | grep -E "^error" | head -2`
Expected: `error[E0583]: file not found for module `macos``

- [ ] **Step 3: Implement the macOS clipboard**

`src/clipboard/macos.rs`:

```rust
//! NSPasteboard implementation. Lazy provision uses NSPasteboardItemDataProvider.

use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardItemDataProvider, NSPasteboardType,
    NSPasteboardTypeString, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSString};

use super::{Clipboard, ClipboardError, OnRead};

/// One pasteboard item: (UTI type, bytes) pairs in the order the item lists them.
pub struct SnapshotItem {
    formats: Vec<(String, Vec<u8>)>,
}

pub struct MacosSnapshot {
    items: Vec<SnapshotItem>,
}

pub struct ProviderIvars {
    text: String,
    on_read: Cell<Option<OnRead>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and TextProvider does not implement Drop.
    #[unsafe(super(NSObject))]
    #[name = "PlainPasteTextProvider"]
    #[ivars = ProviderIvars]
    struct TextProvider;

    unsafe impl NSObjectProtocol for TextProvider {}

    unsafe impl NSPasteboardItemDataProvider for TextProvider {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        fn provide_data_for_type(
            &self,
            _pasteboard: Option<&NSPasteboard>,
            item: &NSPasteboardItem,
            data_type: &NSPasteboardType,
        ) {
            let text = NSString::from_str(&self.ivars().text);
            if !item.setString_forType(&text, data_type) {
                log::error!(
                    "clipboard provide-data failed: NSPasteboardItem setString:forType:{data_type} returned NO"
                );
            }
            if let Some(on_read) = self.ivars().on_read.take() {
                on_read();
            }
        }
    }
);

impl TextProvider {
    fn new(text: String, on_read: OnRead) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ProviderIvars {
            text,
            on_read: Cell::new(Some(on_read)),
        });
        // SAFETY: NSObject's designated initializer is `init`.
        unsafe { msg_send![super(this), init] }
    }
}

pub struct MacosClipboard {
    pasteboard: Retained<NSPasteboard>,
    /// Kept alive while its lazy offer is on the pasteboard.
    provider: Option<Retained<TextProvider>>,
}

impl MacosClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(MacosClipboard {
            pasteboard: NSPasteboard::generalPasteboard(),
            provider: None,
        })
    }

    fn write_items(
        &self,
        op: &'static str,
        items: &[Retained<NSPasteboardItem>],
    ) -> Result<(), ClipboardError> {
        self.pasteboard.clearContents();
        let writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
            .iter()
            .map(|item| ProtocolObject::from_retained(item.clone()))
            .collect();
        if self
            .pasteboard
            .writeObjects(&NSArray::from_retained_slice(&writers))
        {
            Ok(())
        } else {
            Err(ClipboardError::new(
                op,
                format!(
                    "NSPasteboard writeObjects: expected YES, actual NO for {} item(s)",
                    items.len()
                ),
            ))
        }
    }
}

impl Clipboard for MacosClipboard {
    type Snapshot = MacosSnapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        Ok(self
            .pasteboard
            .stringForType(string_type)
            .map(|s| s.to_string()))
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.provider = None;
        self.pasteboard.clearContents();
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        if self
            .pasteboard
            .setString_forType(&NSString::from_str(text), string_type)
        {
            Ok(())
        } else {
            Err(ClipboardError::new(
                "write text",
                format!("NSPasteboard setString:forType:{string_type}: expected YES, actual NO"),
            ))
        }
    }

    fn snapshot(&mut self) -> Result<MacosSnapshot, ClipboardError> {
        let Some(items) = self.pasteboard.pasteboardItems() else {
            return Ok(MacosSnapshot { items: Vec::new() });
        };
        let mut snapshot = MacosSnapshot { items: Vec::new() };
        for (index, item) in items.iter().enumerate() {
            let mut formats = Vec::new();
            for data_type in item.types().iter() {
                // macOS also advertises synthesized conversion types (for example
                // public.utf16-external-plain-text) that return nil. The pasteboard
                // re-synthesizes them from the restored source types.
                match item.dataForType(&data_type) {
                    Some(data) => formats.push((data_type.to_string(), data.to_vec())),
                    None => log::info!(
                        "clipboard snapshot: NSPasteboardItem #{index} type {data_type} has no data, not saved"
                    ),
                }
            }
            snapshot.items.push(SnapshotItem { formats });
        }
        Ok(snapshot)
    }

    fn offer_lazy_text(&mut self, text: String, on_read: OnRead) -> Result<(), ClipboardError> {
        let provider = TextProvider::new(text, on_read);
        let item = NSPasteboardItem::new();
        // SAFETY: NSPasteboardTypeString is an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        let types = NSArray::from_slice(&[string_type]);
        if !item.setDataProvider_forTypes(ProtocolObject::from_ref(&*provider), &types) {
            return Err(ClipboardError::new(
                "offer lazy text",
                format!(
                    "NSPasteboardItem setDataProvider:forTypes:[{string_type}]: expected YES, actual NO"
                ),
            ));
        }
        self.write_items("offer lazy text", &[item])?;
        self.provider = Some(provider);
        Ok(())
    }

    fn restore(&mut self, snapshot: MacosSnapshot) -> Result<(), ClipboardError> {
        self.provider = None;
        let mut items = Vec::with_capacity(snapshot.items.len());
        for (index, saved) in snapshot.items.into_iter().enumerate() {
            let item = NSPasteboardItem::new();
            for (data_type, bytes) in saved.formats {
                if !item.setData_forType(&NSData::from_vec(bytes), &NSString::from_str(&data_type))
                {
                    return Err(ClipboardError::new(
                        "restore",
                        format!(
                            "NSPasteboardItem #{index} setData:forType:{data_type}: expected YES, actual NO"
                        ),
                    ));
                }
            }
            items.push(item);
        }
        if items.is_empty() {
            self.pasteboard.clearContents();
            return Ok(());
        }
        self.write_items("restore", &items)
    }
}
```

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning"`
Expected: `0`

- [ ] **Step 4: Create the scratch probe (not committed; `tmp/` is gitignored)**

`tmp/clipboard-probe/Cargo.toml`:

```toml
[package]
name = "clipboard-probe"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
plainpaste = { path = "../.." }
slint = "1.18.1"
```

`tmp/clipboard-probe/src/main.rs` (Tasks 4 and 5 reuse it unchanged; `tmp/` is shared through the working tree):

```rust
//! Scratch probe (not committed): snapshot, lazy offer read by another process,
//! restore, then list the restored formats while this process still runs
//! (on X11 the restored data lives only as long as its owner process).

use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use plainpaste::clipboard::{Clipboard, PlatformClipboard};

const OFFERED: &str = "LAZY OUT";
const ON_READ_BOUND: Duration = Duration::from_secs(2);

fn command(program: &str, args: &[&str]) -> Command {
    let mut c = Command::new(program);
    c.args(args);
    c
}

/// Reads the clipboard as plain text from another process.
fn reader() -> Command {
    if cfg!(target_os = "macos") {
        command("pbpaste", &[])
    } else if cfg!(target_os = "windows") {
        command("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])
    } else {
        command("xclip", &["-selection", "clipboard", "-o"])
    }
}

/// Lists the formats on the clipboard from another process.
fn format_lister() -> Command {
    if cfg!(target_os = "macos") {
        command("osascript", &["-e", "clipboard info"])
    } else if cfg!(target_os = "windows") {
        command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; 'HTML Format present: ' + [System.Windows.Forms.Clipboard]::ContainsData('HTML Format')",
            ],
        )
    } else {
        command("xclip", &["-selection", "clipboard", "-o", "-t", "TARGETS"])
    }
}

fn main() {
    // Initialize the event loop backend before another thread may call invoke_from_event_loop.
    slint::BackendSelector::new()
        .select()
        .expect("select slint backend");
    let mut clipboard = PlatformClipboard::new().expect("create clipboard");
    let snapshot = clipboard.snapshot().expect("snapshot");
    let (tx, rx) = mpsc::channel();
    clipboard
        .offer_lazy_text(
            OFFERED.into(),
            Box::new(move || tx.send(()).expect("probe channel")),
        )
        .expect("offer lazy text");
    thread::spawn(|| {
        let out = reader().output().expect("run reader");
        println!(
            "reader read: {:?}",
            String::from_utf8_lossy(&out.stdout).trim_end()
        );
        slint::invoke_from_event_loop(|| slint::quit_event_loop().expect("quit")).expect("invoke");
    });
    slint::run_event_loop_until_quit().expect("event loop");
    println!("on_read fired: {}", rx.recv_timeout(ON_READ_BOUND).is_ok());
    clipboard.restore(snapshot).expect("restore");
    println!("restored");
    // The lister runs on a thread while the event loop runs again: macOS may call back into
    // this process (the pasteboard owner) while another process reads, as in the real app.
    thread::spawn(|| {
        let formats = format_lister().output().expect("run format lister");
        println!(
            "after restore: {}",
            String::from_utf8_lossy(&formats.stdout)
                .trim_end()
                .replace('\n', " | ")
        );
        slint::invoke_from_event_loop(|| slint::quit_event_loop().expect("quit")).expect("invoke");
    });
    slint::run_event_loop_until_quit().expect("event loop");
}
```

- [ ] **Step 5: Run the probe on macOS**

Put rich text (RTF) on the clipboard, then run the probe:

```bash
echo '<b>bold hi</b>' | textutil -stdin -format html -convert rtf -stdout | pbcopy -Prefer rtf
osascript -e 'clipboard info'
cd tmp/clipboard-probe
CARGO_TARGET_DIR=../../target cargo run 2>/dev/null; echo RC=$?
cd ../..
```

Expected: `clipboard info` prints `«class RTF », 340`. The probe prints `reader read: "LAZY OUT"`, `on_read fired: true`, `restored`, `after restore: «class RTF », 340, …`, `RC=0` (the RTF snapshot came back). Observed on macOS 15 during planning: exactly these lines.
Finding behind the probe's second event-loop run: while this process owns the pasteboard, another process that reads it can block until this process's main thread runs its event loop. The app always runs its loop, so this only matters for probes.

- [ ] **Step 6: Run the suite**

Run: `python3 scripts/test.py | tail -1`
Expected: `RESULT: PASS`

- [ ] **Step 7: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add src/clipboard/mod.rs src/clipboard/macos.rs
git commit -m "feat: add clipboard interface and macOS implementation" -- src/clipboard/mod.rs src/clipboard/macos.rs src/lib.rs
git status --porcelain
```

### Task 4: Windows clipboard implementation

**Files:**
- Create: `src/clipboard/windows.rs`
- Scratch (not committed): reuses `tmp/clipboard-probe/` from Task 3.

**Interfaces:**
- Consumes: `Clipboard`, `ClipboardError`, `OnRead` from Task 3; `windows` 0.62.2 (`Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_System_DataExchange`, `Win32_System_LibraryLoader`, `Win32_System_Memory`, `Win32_System_Ole`, `Win32_UI_WindowsAndMessaging`).
- Produces: `pub struct WindowsClipboard` with `pub fn new() -> Result<Self, ClipboardError>`, `impl Clipboard` with `type Snapshot = WindowsSnapshot` (opaque `pub struct`).

**Behavior:**
`new` registers the window class `PlainPasteClipboardOwner` and creates a message-only window (`HWND_MESSAGE`) on the calling thread (the main thread). Its address of a boxed `OwnerState` is stored in `GWLP_USERDATA`. Because the window lives on the main thread, the Slint (winit) message loop dispatches `WM_RENDERFORMAT` to it.
Every operation opens the clipboard with this window as owner through `OpenClipboardGuard` (closes on drop; a close failure is logged).
- `read_text`: `IsClipboardFormatAvailable(CF_UNICODETEXT)` fails → `None`; else copy the HGLOBAL, decode UTF-16 up to the first NUL.
- `snapshot`: `EnumClipboardFormats` over every format. GDI-handle formats are logged and skipped (Spec Coverage Notes). `CF_ENHMETAFILE` is copied with `GetEnhMetaFileBits`. Every other format is copied as HGLOBAL bytes.
- `offer_lazy_text`: `EmptyClipboard` (makes the window the owner), store the text and `on_read` in `OwnerState`, `SetClipboardData(CF_UNICODETEXT, None)` (delayed rendering).
- Window procedure: on `WM_RENDERFORMAT` with `wParam == CF_UNICODETEXT` and a pending offer, allocate the UTF-16 text (NUL-terminated) and `SetClipboardData` without opening the clipboard, then call `on_read` once. Every other message goes to `DefWindowProcW`.
- `restore` and `write_text`: clear the pending offer, open, `EmptyClipboard`, set each format (`SetEnhMetaFileBits` for `CF_ENHMETAFILE`). If `SetClipboardData` fails, free the HGLOBAL (the clipboard owns it only on success).
- `GlobalUnlock` returning FALSE with last error `NO_ERROR` is the documented "unlocked" result and is treated as success.

**Constraints:**
- Every `unsafe` block has a `// SAFETY:` comment.
- No `OleSetClipboard`/COM: plain Win32 clipboard API only.
- Layer: library, `#[cfg(target_os = "windows")]` module.

**Acceptance criteria:**
- [ ] On the Windows host: `cargo clippy --all-targets` prints no `warning` line.
- [ ] On the Windows host: the probe prints `reader read: "LAZY OUT"`, `on_read fired: true`, `restored`, `after restore: HTML Format present: True`, and exits 0.
- [ ] `python3 scripts/test.py` on the Windows host ends with `RESULT: PASS`.

**Steps:**

No committed test (platform code). The `windows` crate does not compile on macOS, so every build and probe step runs on the Windows host. Abbreviation used below: `WIN="ssh -i <ssh-key> <windows-user>@<windows-host>"` (run `export WIN=...` once in your shell, or type the full command each time).

- [ ] **Step 1: Write the Windows implementation**

`src/clipboard/windows.rs`:

```rust
//! Win32 clipboard implementation. Lazy provision uses delayed rendering:
//! SetClipboardData(CF_UNICODETEXT, NULL), then WM_RENDERFORMAT on a message-only owner window.
//! The owner window lives on the main thread, so the Slint (winit) message loop dispatches to it.

use std::cell::RefCell;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetEnhMetaFileBits, HENHMETAFILE, SetEnhMetaFileBits};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Ole::{
    CF_BITMAP, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT, CF_ENHMETAFILE,
    CF_METAFILEPICT, CF_OWNERDISPLAY, CF_PALETTE, CF_UNICODETEXT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetWindowLongPtrW, HWND_MESSAGE,
    RegisterClassW, SetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_RENDERFORMAT, WNDCLASSW,
};
use windows::core::{HRESULT, PCWSTR, w};

use super::{Clipboard, ClipboardError, OnRead};

const OWNER_CLASS_NAME: PCWSTR = w!("PlainPasteClipboardOwner");
const UTF16_NUL: u16 = 0;
/// GlobalUnlock returns FALSE with last error NO_ERROR when the lock count reaches zero (documented success).
const NO_ERROR: HRESULT = HRESULT(0);
/// Formats whose handle is a GDI object, not HGLOBAL memory. Windows synthesizes
/// CF_BITMAP from CF_DIB and CF_METAFILEPICT from CF_ENHMETAFILE, so the
/// HGLOBAL siblings carry the same content. CF_ENHMETAFILE is saved through its bits.
const GDI_HANDLE_FORMATS: [u32; 7] = [
    CF_BITMAP.0 as u32,
    CF_METAFILEPICT.0 as u32,
    CF_PALETTE.0 as u32,
    CF_OWNERDISPLAY.0 as u32,
    CF_DSPBITMAP.0 as u32,
    CF_DSPMETAFILEPICT.0 as u32,
    CF_DSPENHMETAFILE.0 as u32,
];

pub struct WindowsSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

/// State the window procedure reads on WM_RENDERFORMAT.
struct PendingOffer {
    text: String,
    on_read: Option<OnRead>,
}

struct OwnerState {
    pending: RefCell<Option<PendingOffer>>,
}

pub struct WindowsClipboard {
    hwnd: HWND,
    /// Boxed so its address, stored in GWLP_USERDATA, stays fixed.
    state: Box<OwnerState>,
}

fn win_err(op: &'static str, api: &str, target: &str, e: windows::core::Error) -> ClipboardError {
    ClipboardError::new(
        op,
        format!(
            "{api} for {target}: expected success, actual HRESULT {:#010x} ({e})",
            e.code().0
        ),
    )
}

/// Closes the clipboard when dropped. CloseClipboard failure is logged, because Drop cannot return it.
struct OpenClipboardGuard;

impl OpenClipboardGuard {
    fn open(op: &'static str, owner: HWND) -> Result<Self, ClipboardError> {
        // SAFETY: owner is the live message-only window created in WindowsClipboard::new.
        unsafe { OpenClipboard(Some(owner)) }
            .map_err(|e| win_err(op, "OpenClipboard", "the clipboard", e))?;
        Ok(OpenClipboardGuard)
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: this guard exists only while this thread holds the clipboard open.
        if let Err(e) = unsafe { CloseClipboard() } {
            log::error!(
                "clipboard close failed: CloseClipboard: HRESULT {:#010x} ({e})",
                e.code().0
            );
        }
    }
}

fn copy_hglobal(op: &'static str, format: u32, handle: HANDLE) -> Result<Vec<u8>, ClipboardError> {
    let hglobal = HGLOBAL(handle.0);
    // SAFETY: handle is clipboard data for an HGLOBAL format, valid while the clipboard is open.
    unsafe {
        let size = GlobalSize(hglobal);
        let ptr = GlobalLock(hglobal) as *const u8;
        if ptr.is_null() {
            return Err(ClipboardError::new(
                op,
                format!("GlobalLock for format {format}: expected a pointer, actual NULL"),
            ));
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        unlock(op, format, hglobal)?;
        Ok(bytes)
    }
}

fn unlock(op: &'static str, format: u32, hglobal: HGLOBAL) -> Result<(), ClipboardError> {
    // SAFETY: hglobal was locked once by the caller.
    match unsafe { GlobalUnlock(hglobal) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == NO_ERROR => Ok(()),
        Err(e) => Err(win_err(op, "GlobalUnlock", &format!("format {format}"), e)),
    }
}

fn alloc_hglobal(op: &'static str, format: u32, bytes: &[u8]) -> Result<HGLOBAL, ClipboardError> {
    // SAFETY: the allocation is sized for `bytes` and locked once while copying.
    unsafe {
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| {
            win_err(
                op,
                "GlobalAlloc",
                &format!("format {format}, {} bytes", bytes.len()),
                e,
            )
        })?;
        let ptr = GlobalLock(hglobal) as *mut u8;
        if ptr.is_null() {
            return Err(ClipboardError::new(
                op,
                format!("GlobalLock for format {format}: expected a pointer, actual NULL"),
            ));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        unlock(op, format, hglobal)?;
        Ok(hglobal)
    }
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(UTF16_NUL))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Hands `bytes` to the clipboard. The clipboard owns the memory only on success.
fn set_hglobal(op: &'static str, format: u32, bytes: &[u8]) -> Result<(), ClipboardError> {
    let hglobal = alloc_hglobal(op, format, bytes)?;
    // SAFETY: the clipboard is open (or we are inside WM_RENDERFORMAT) and hglobal is a fresh allocation.
    unsafe {
        if let Err(e) = SetClipboardData(format, Some(HANDLE(hglobal.0))) {
            if let Err(free_err) = GlobalFree(Some(hglobal)) {
                log::error!(
                    "clipboard {op}: GlobalFree after failed SetClipboardData for format {format}: {free_err}"
                );
            }
            return Err(win_err(
                op,
                "SetClipboardData",
                &format!("format {format}"),
                e,
            ));
        }
    }
    Ok(())
}

unsafe extern "system" fn owner_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_RENDERFORMAT && wparam.0 as u32 == CF_UNICODETEXT.0 as u32 {
        // SAFETY: GWLP_USERDATA holds the OwnerState pointer set in WindowsClipboard::new; the box outlives the window.
        let state = unsafe { &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const OwnerState) };
        let taken = state
            .pending
            .borrow_mut()
            .as_mut()
            .map(|p| (p.text.clone(), p.on_read.take()));
        if let Some((text, on_read)) = taken {
            if let Err(e) = set_hglobal(
                "render format",
                CF_UNICODETEXT.0 as u32,
                &utf16_bytes(&text),
            ) {
                log::error!("{e}");
            }
            if let Some(on_read) = on_read {
                on_read();
            }
        }
        return LRESULT(0);
    }
    // SAFETY: default handling for every other message.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

impl WindowsClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        const OP: &str = "create owner window";
        // SAFETY: registering a class and creating a message-only window on the main thread.
        unsafe {
            let instance = GetModuleHandleW(None)
                .map_err(|e| win_err(OP, "GetModuleHandleW", "this module", e))?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(owner_wndproc),
                hInstance: instance.into(),
                lpszClassName: OWNER_CLASS_NAME,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(win_err(
                    OP,
                    "RegisterClassW",
                    "PlainPasteClipboardOwner",
                    windows::core::Error::from_thread(),
                ));
            }
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                OWNER_CLASS_NAME,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
            .map_err(|e| win_err(OP, "CreateWindowExW", "the clipboard owner window", e))?;
            let state = Box::new(OwnerState {
                pending: RefCell::new(None),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*state as *const OwnerState as isize);
            Ok(WindowsClipboard { hwnd, state })
        }
    }
}

impl Clipboard for WindowsClipboard {
    type Snapshot = WindowsSnapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        const OP: &str = "read text";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        let format = CF_UNICODETEXT.0 as u32;
        // SAFETY: the clipboard is open.
        if unsafe { IsClipboardFormatAvailable(format) }.is_err() {
            return Ok(None);
        }
        // SAFETY: the clipboard is open and CF_UNICODETEXT is available.
        let handle = unsafe { GetClipboardData(format) }
            .map_err(|e| win_err(OP, "GetClipboardData", "CF_UNICODETEXT", e))?;
        let bytes = copy_hglobal(OP, format, handle)?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != UTF16_NUL)
            .collect();
        String::from_utf16(&units).map(Some).map_err(|e| {
            ClipboardError::new(
                OP,
                format!("CF_UNICODETEXT: expected valid UTF-16, actual {e}"),
            )
        })
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        const OP: &str = "write text";
        *self.state.pending.borrow_mut() = None;
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        set_hglobal(OP, CF_UNICODETEXT.0 as u32, &utf16_bytes(text))
    }

    fn snapshot(&mut self) -> Result<WindowsSnapshot, ClipboardError> {
        const OP: &str = "snapshot";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        let mut formats = Vec::new();
        let mut format = 0u32;
        loop {
            // SAFETY: the clipboard is open; 0 starts the enumeration.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if GDI_HANDLE_FORMATS.contains(&format) {
                log::info!(
                    "clipboard snapshot: GDI handle format {format} not saved, Windows synthesizes it from its HGLOBAL sibling"
                );
                continue;
            }
            // SAFETY: the clipboard is open and `format` was just enumerated.
            let handle = unsafe { GetClipboardData(format) }
                .map_err(|e| win_err(OP, "GetClipboardData", &format!("format {format}"), e))?;
            let bytes = if format == CF_ENHMETAFILE.0 as u32 {
                let hemf = HENHMETAFILE(handle.0);
                // SAFETY: the handle is an enhanced metafile owned by the clipboard.
                let size = unsafe { GetEnhMetaFileBits(hemf, None) };
                let mut bits = vec![0u8; size as usize];
                // SAFETY: `bits` has exactly `size` bytes.
                if size == 0 || unsafe { GetEnhMetaFileBits(hemf, Some(&mut bits)) } != size {
                    return Err(ClipboardError::new(
                        OP,
                        format!(
                            "GetEnhMetaFileBits for CF_ENHMETAFILE: expected {size} bytes, actual a failed copy"
                        ),
                    ));
                }
                bits
            } else {
                copy_hglobal(OP, format, handle)?
            };
            formats.push((format, bytes));
        }
        Ok(WindowsSnapshot { formats })
    }

    fn offer_lazy_text(&mut self, text: String, on_read: OnRead) -> Result<(), ClipboardError> {
        const OP: &str = "offer lazy text";
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open; EmptyClipboard makes self.hwnd the owner.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        *self.state.pending.borrow_mut() = Some(PendingOffer {
            text,
            on_read: Some(on_read),
        });
        // SAFETY: a NULL handle requests delayed rendering.
        unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, None) }
            .map_err(|e| win_err(OP, "SetClipboardData(NULL)", "CF_UNICODETEXT", e))?;
        Ok(())
    }

    fn restore(&mut self, snapshot: WindowsSnapshot) -> Result<(), ClipboardError> {
        const OP: &str = "restore";
        *self.state.pending.borrow_mut() = None;
        let _open = OpenClipboardGuard::open(OP, self.hwnd)?;
        // SAFETY: the clipboard is open.
        unsafe { EmptyClipboard() }
            .map_err(|e| win_err(OP, "EmptyClipboard", "the clipboard", e))?;
        for (format, bytes) in snapshot.formats {
            if format == CF_ENHMETAFILE.0 as u32 {
                // SAFETY: `bytes` came from GetEnhMetaFileBits.
                let hemf = unsafe { SetEnhMetaFileBits(&bytes) };
                if hemf.is_invalid() {
                    return Err(ClipboardError::new(
                        OP,
                        "SetEnhMetaFileBits for CF_ENHMETAFILE: expected a handle, actual NULL",
                    ));
                }
                // SAFETY: the clipboard is open and takes ownership of hemf on success.
                unsafe { SetClipboardData(format, Some(HANDLE(hemf.0))) }
                    .map_err(|e| win_err(OP, "SetClipboardData", "CF_ENHMETAFILE", e))?;
            } else {
                set_hglobal(OP, format, &bytes)?;
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Check the Windows host toolchain**

Run: `$WIN "powershell -Command \"cargo --version; python --version; tar --version\""`
Expected: `cargo 1.94.x`, `Python 3.x`, `bsdtar …`. If cargo is missing or older than 1.94, report `BLOCKED` with the output (installing toolchains on the host is the user's decision).

- [ ] **Step 3: Copy the working tree to the host**

```bash
tar -cf tmp/win-src.tar Cargo.toml Cargo.lock src tests scripts tmp/clipboard-probe/Cargo.toml tmp/clipboard-probe/src $(ls -d build.rs ui 2>/dev/null)
$WIN "powershell -Command \"New-Item -ItemType Directory -Force -Path plainpaste | Out-Null\""
scp -i <ssh-key> tmp/win-src.tar <windows-user>@<windows-host>:plainpaste/win-src.tar
$WIN "powershell -Command \"cd plainpaste; tar -xf win-src.tar; Remove-Item win-src.tar\""
rm tmp/win-src.tar
```

Expected: no error output.

- [ ] **Step 4: Lint on the host**

Run: `$WIN "powershell -Command \"cd plainpaste; cargo clippy --all-targets 2>&1 | Select-String -Pattern '^(warning|error)' | Measure-Object | Select-Object -ExpandProperty Count\""`
Expected: `0`. (The first build compiles every dependency and takes several minutes. If it exceeds your foreground tool limit, run it detached and wait for it.)

- [ ] **Step 5: Run the probe on the host**

```bash
$WIN "powershell -Command \"Set-Clipboard -AsHtml -Value '<b>bold hi</b>'\""
$WIN "powershell -Command \"cd plainpaste/tmp/clipboard-probe; \$env:CARGO_TARGET_DIR='../../target'; cargo run 2>\$null; echo RC=\$LASTEXITCODE\""
```

Expected: the probe prints `reader read: "LAZY OUT"`, `on_read fired: true`, `restored`, `after restore: HTML Format present: True`, `RC=0`.
If `Set-Clipboard` or the probe fails over ssh with an access-denied or window-station error, run the same two commands (without the `$WIN` prefix and the outer quoting) in a PowerShell window on the Windows desktop (ask the user to do it) and record that the probe ran on the desktop.

- [ ] **Step 6: Run the suite on the host**

Run: `$WIN "powershell -Command \"cd plainpaste; python scripts/test.py\""`
Expected: last line `RESULT: PASS`.

- [ ] **Step 7: Commit (on the Mac, in the repository)**

```bash
git rev-parse --abbrev-ref HEAD
git add src/clipboard/windows.rs
git commit -m "feat: add Windows clipboard with delayed rendering" -- src/clipboard/windows.rs
git status --porcelain
```

### Task 5: X11 clipboard implementation

**Files:**
- Create: `src/clipboard/x11.rs`
- Scratch (not committed): reuses `tmp/clipboard-probe/` from Task 3; creates `tmp/linux/Dockerfile`.

**Interfaces:**
- Consumes: `Clipboard`, `ClipboardError`, `OnRead` from Task 3; `x11rb` 0.14.0.
- Produces: `pub struct X11Clipboard` with `pub fn new() -> Result<Self, ClipboardError>`, `impl Clipboard` with `type Snapshot = X11Snapshot` (opaque `pub struct`).

**Behavior:**
`new` connects with `RustConnection::connect(None)` (uses `$DISPLAY`), creates a 1×1 `INPUT_ONLY` window, interns the atoms `CLIPBOARD, TARGETS, MULTIPLE, TIMESTAMP, SAVE_TARGETS, INCR, UTF8_STRING, text/plain;charset=utf-8, PLAINPASTE_TRANSFER` with `x11rb::atom_manager!`, and starts the thread `x11-clipboard`. The connection is shared through `Arc<RustConnection>` (RustConnection is `Send + Sync`).
Event thread: blocks in `wait_for_event()` (push, no polling). `SelectionNotify` → forwarded on an `mpsc` channel to the caller. `SelectionRequest` → answered from the served formats (`TARGETS` lists `TARGETS` plus every served target; a served target is written with `ChangeProperty` to the requestor; unknown targets get property `NONE`), then a `SelectionNotify` is sent with `SendEvent`. The first answered data request (not `TARGETS`) calls `on_read` on this thread. `SelectionClear` for CLIPBOARD → the served formats are dropped. A connection error ends the thread with an error log.
Caller side: `convert(target)` sends `ConvertSelection(CLIPBOARD → target into PLAINPASTE_TRANSFER)` and waits on the channel with `recv_timeout(SELECTION_REPLY_TIMEOUT)` (2 s fail-fast); stale notifications for other targets are logged and dropped; property `NONE` → `Ok(None)`; otherwise `GetProperty(delete = true)`; type `INCR` → error.
- `read_text` = `convert(UTF8_STRING)`, then UTF-8 decode.
- `snapshot` = `convert(TARGETS)`, then `convert` each target except `TARGETS, MULTIPLE, TIMESTAMP, SAVE_TARGETS`. A target listed in TARGETS but refused → error.
- `write_text`, `offer_lazy_text`, `restore` set the served formats (text → `UTF8_STRING` and `text/plain;charset=utf-8`, type `UTF8_STRING`, format 8), call `SetSelectionOwner`, and verify with `GetSelectionOwner` that this window is the owner.
- The restored clipboard stays available only while the app runs (X11 has no clipboard storage without a clipboard manager).

**Constraints:**
- `SELECTION_REPLY_TIMEOUT` is the only time bound in this file and is a named constant.
- Layer: library, `#[cfg(target_os = "linux")]` module.

**Acceptance criteria:**
- [ ] In the Linux container: `cargo clippy --all-targets` prints no `warning` line.
- [ ] In the Linux container under Xvfb: `tmp/linux/probe.sh` prints `before: TARGETS text/html`, `reader read: "LAZY OUT"`, `on_read fired: true`, `restored`, `after restore: TARGETS | text/html`, `RC=0` (observed exactly during planning).
- [ ] In the Linux container: `python3 scripts/test.py` ends with `RESULT: PASS`.

**Steps:**

No committed test (platform code). Linux runs in Docker on the Mac. All Docker commands run from the repository root and mount it at `/src`; build output goes to `tmp/target-linux` (gitignored). Abbreviation: `DRUN="docker run --rm -v $PWD:/src -v $HOME/.cargo/registry:/usr/local/cargo/registry -e CARGO_TARGET_DIR=/src/tmp/target-linux"`.

- [ ] **Step 1: Write the X11 implementation**

`src/clipboard/x11.rs`:

```rust
//! X11 CLIPBOARD selection implementation. The app becomes the selection owner and
//! answers SelectionRequest itself, so the offered data is lazy by nature.
//! A dedicated thread blocks in `wait_for_event` (event push, no polling) and
//! serves requests. The caller thread issues requests on the shared connection.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt, CreateWindowAux, EventMask, PropMode, SELECTION_NOTIFY_EVENT,
    SelectionNotifyEvent, SelectionRequestEvent, Time, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use super::{Clipboard, ClipboardError, OnRead};

/// Fail-fast IO bound for one ConvertSelection round trip to another client.
/// An unresponsive selection owner must not block the hotkey path forever.
const SELECTION_REPLY_TIMEOUT: Duration = Duration::from_secs(2);
const FORMAT_8_BIT: u8 = 8;
const FORMAT_32_BIT: u8 = 32;
const BYTES_PER_32_BIT_ITEM: usize = 4;
/// GetProperty length is counted in 32-bit units. This asks for everything.
const WHOLE_PROPERTY_LENGTH: u32 = u32::MAX / BYTES_PER_32_BIT_ITEM as u32;
/// Space reserved for the ChangeProperty request header inside the maximum request size.
const CHANGE_PROPERTY_HEADER_BYTES: usize = 24;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        CLIPBOARD,
        TARGETS,
        MULTIPLE,
        TIMESTAMP,
        SAVE_TARGETS,
        INCR,
        UTF8_STRING,
        TEXT_PLAIN_UTF8: b"text/plain;charset=utf-8",
        PLAINPASTE_TRANSFER,
    }
}

/// One served or saved selection target.
#[derive(Clone)]
struct Format {
    target: Atom,
    type_: Atom,
    format: u8,
    bytes: Vec<u8>,
}

pub struct X11Snapshot {
    formats: Vec<Format>,
}

/// What this client currently serves as the CLIPBOARD owner.
#[derive(Default)]
struct Served {
    formats: Vec<Format>,
    on_read: Option<OnRead>,
}

pub struct X11Clipboard {
    conn: Arc<RustConnection>,
    window: Window,
    atoms: Atoms,
    served: Arc<Mutex<Served>>,
    replies: Receiver<SelectionNotifyEvent>,
}

fn conn_err(op: &'static str, request: &str, e: impl std::fmt::Display) -> ClipboardError {
    ClipboardError::new(op, format!("X11 {request}: expected success, actual {e}"))
}

impl X11Clipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        const OP: &str = "connect";
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| conn_err(OP, "connect($DISPLAY)", e))?;
        let root = conn.setup().roots[screen_num].root;
        let window = conn
            .generate_id()
            .map_err(|e| conn_err(OP, "generate_id", e))?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|e| conn_err(OP, "CreateWindow", e))?;
        let atoms = Atoms::new(&conn)
            .map_err(|e| conn_err(OP, "InternAtom", e))?
            .reply()
            .map_err(|e| conn_err(OP, "InternAtom reply", e))?;
        conn.flush().map_err(|e| conn_err(OP, "flush", e))?;

        let conn = Arc::new(conn);
        let served = Arc::new(Mutex::new(Served::default()));
        let (reply_tx, replies) = mpsc::channel();
        let thread_conn = Arc::clone(&conn);
        let thread_served = Arc::clone(&served);
        thread::Builder::new()
            .name("x11-clipboard".into())
            .spawn(move || event_thread(thread_conn, atoms, thread_served, reply_tx))
            .map_err(|e| conn_err(OP, "spawn event thread", e))?;
        Ok(X11Clipboard {
            conn,
            window,
            atoms,
            served,
            replies,
        })
    }

    /// Asks the current owner to convert CLIPBOARD to `target`, then reads the result.
    /// `Ok(None)` means the owner refused the target (or there is no owner).
    fn convert(
        &mut self,
        op: &'static str,
        target: Atom,
    ) -> Result<Option<Format>, ClipboardError> {
        let transfer = self.atoms.PLAINPASTE_TRANSFER;
        self.conn
            .convert_selection(
                self.window,
                self.atoms.CLIPBOARD,
                target,
                transfer,
                Time::CURRENT_TIME,
            )
            .map_err(|e| conn_err(op, "ConvertSelection", e))?;
        self.conn.flush().map_err(|e| conn_err(op, "flush", e))?;
        let notify = loop {
            let event = self.replies.recv_timeout(SELECTION_REPLY_TIMEOUT).map_err(|e| {
                ClipboardError::new(
                    op,
                    format!("SelectionNotify for target {target}: expected within {SELECTION_REPLY_TIMEOUT:?}, actual {e}"),
                )
            })?;
            if event.target == target {
                break event;
            }
            log::warn!(
                "clipboard {op}: dropped stale SelectionNotify for target {}",
                event.target
            );
        };
        if notify.property == u32::from(AtomEnum::NONE) {
            return Ok(None);
        }
        let reply = self
            .conn
            .get_property(
                true,
                self.window,
                transfer,
                AtomEnum::ANY,
                0,
                WHOLE_PROPERTY_LENGTH,
            )
            .map_err(|e| conn_err(op, "GetProperty", e))?
            .reply()
            .map_err(|e| conn_err(op, "GetProperty reply", e))?;
        if reply.type_ == self.atoms.INCR {
            return Err(ClipboardError::new(
                op,
                format!(
                    "target {target}: expected a single-transfer property, actual INCR (large data, not supported)"
                ),
            ));
        }
        Ok(Some(Format {
            target,
            type_: reply.type_,
            format: reply.format,
            bytes: reply.value,
        }))
    }

    /// Serves `formats` as the CLIPBOARD owner.
    fn own(
        &mut self,
        op: &'static str,
        formats: Vec<Format>,
        on_read: Option<OnRead>,
    ) -> Result<(), ClipboardError> {
        *self.served.lock().expect("served mutex poisoned") = Served { formats, on_read };
        self.conn
            .set_selection_owner(self.window, self.atoms.CLIPBOARD, Time::CURRENT_TIME)
            .map_err(|e| conn_err(op, "SetSelectionOwner", e))?;
        let owner = self
            .conn
            .get_selection_owner(self.atoms.CLIPBOARD)
            .map_err(|e| conn_err(op, "GetSelectionOwner", e))?
            .reply()
            .map_err(|e| conn_err(op, "GetSelectionOwner reply", e))?
            .owner;
        if owner != self.window {
            return Err(ClipboardError::new(
                op,
                format!(
                    "CLIPBOARD owner: expected window {}, actual {owner}",
                    self.window
                ),
            ));
        }
        Ok(())
    }

    fn text_formats(&self, text: &str) -> Vec<Format> {
        [self.atoms.UTF8_STRING, self.atoms.TEXT_PLAIN_UTF8]
            .into_iter()
            .map(|target| Format {
                target,
                type_: self.atoms.UTF8_STRING,
                format: FORMAT_8_BIT,
                bytes: text.as_bytes().to_vec(),
            })
            .collect()
    }
}

impl Clipboard for X11Clipboard {
    type Snapshot = X11Snapshot;

    fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        const OP: &str = "read text";
        match self.convert(OP, self.atoms.UTF8_STRING)? {
            None => Ok(None),
            Some(format) => String::from_utf8(format.bytes).map(Some).map_err(|e| {
                ClipboardError::new(OP, format!("UTF8_STRING: expected valid UTF-8, actual {e}"))
            }),
        }
    }

    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        let formats = self.text_formats(text);
        self.own("write text", formats, None)
    }

    fn snapshot(&mut self) -> Result<X11Snapshot, ClipboardError> {
        const OP: &str = "snapshot";
        let Some(targets) = self.convert(OP, self.atoms.TARGETS)? else {
            return Ok(X11Snapshot {
                formats: Vec::new(),
            });
        };
        let skipped = [
            self.atoms.TARGETS,
            self.atoms.MULTIPLE,
            self.atoms.TIMESTAMP,
            self.atoms.SAVE_TARGETS,
        ];
        let mut formats = Vec::new();
        for chunk in targets.bytes.chunks_exact(BYTES_PER_32_BIT_ITEM) {
            let target = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if skipped.contains(&target) {
                continue;
            }
            let format = self.convert(OP, target)?.ok_or_else(|| {
                ClipboardError::new(
                    OP,
                    format!("target {target}: expected data (owner listed it in TARGETS), actual refusal"),
                )
            })?;
            formats.push(format);
        }
        Ok(X11Snapshot { formats })
    }

    fn offer_lazy_text(&mut self, text: String, on_read: OnRead) -> Result<(), ClipboardError> {
        let formats = self.text_formats(&text);
        self.own("offer lazy text", formats, Some(on_read))
    }

    fn restore(&mut self, snapshot: X11Snapshot) -> Result<(), ClipboardError> {
        self.own("restore", snapshot.formats, None)
    }
}

fn event_thread(
    conn: Arc<RustConnection>,
    atoms: Atoms,
    served: Arc<Mutex<Served>>,
    replies: Sender<SelectionNotifyEvent>,
) {
    loop {
        let event = match conn.wait_for_event() {
            Ok(event) => event,
            Err(e) => {
                log::error!(
                    "clipboard event thread: X11 wait_for_event failed: {e}. Clipboard serving stopped."
                );
                return;
            }
        };
        match event {
            Event::SelectionNotify(notify) => {
                if replies.send(notify).is_err() {
                    log::error!(
                        "clipboard event thread: reply receiver dropped. Clipboard serving stopped."
                    );
                    return;
                }
            }
            Event::SelectionRequest(request) => {
                if let Err(e) = answer(&conn, &atoms, &served, &request) {
                    log::error!("{e}");
                }
            }
            Event::SelectionClear(clear) if clear.selection == atoms.CLIPBOARD => {
                *served.lock().expect("served mutex poisoned") = Served::default();
            }
            Event::Error(e) => log::error!("clipboard event thread: X11 error {e:?}"),
            _ => {}
        }
    }
}

fn answer(
    conn: &RustConnection,
    atoms: &Atoms,
    served: &Mutex<Served>,
    request: &SelectionRequestEvent,
) -> Result<(), ClipboardError> {
    const OP: &str = "serve request";
    // Obsolete clients send property None and expect the target name to be used.
    let property = if request.property == u32::from(AtomEnum::NONE) {
        request.target
    } else {
        request.property
    };
    let max_payload = conn.maximum_request_bytes() - CHANGE_PROPERTY_HEADER_BYTES;
    let mut served = served.lock().expect("served mutex poisoned");
    let stored = if request.target == atoms.TARGETS {
        let mut list: Vec<Atom> = vec![atoms.TARGETS];
        list.extend(served.formats.iter().map(|f| f.target));
        conn.change_property32(
            PropMode::REPLACE,
            request.requestor,
            property,
            AtomEnum::ATOM,
            &list,
        )
        .map_err(|e| conn_err(OP, "ChangeProperty(TARGETS)", e))?;
        true
    } else if let Some(format) = served.formats.iter().find(|f| f.target == request.target) {
        if format.bytes.len() > max_payload {
            log::error!(
                "clipboard {OP}: target {}: expected at most {max_payload} bytes, actual {} (INCR not supported), refused",
                request.target,
                format.bytes.len()
            );
            false
        } else {
            let unit = if format.format == FORMAT_32_BIT {
                BYTES_PER_32_BIT_ITEM
            } else {
                1
            };
            conn.change_property(
                PropMode::REPLACE,
                request.requestor,
                property,
                format.type_,
                format.format,
                (format.bytes.len() / unit) as u32,
                &format.bytes,
            )
            .map_err(|e| conn_err(OP, "ChangeProperty", e))?;
            true
        }
    } else {
        false
    };
    let is_data_read = stored && request.target != atoms.TARGETS;
    let on_read = if is_data_read {
        served.on_read.take()
    } else {
        None
    };
    drop(served);

    let notify = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        property: if stored {
            property
        } else {
            AtomEnum::NONE.into()
        },
    };
    conn.send_event(false, request.requestor, EventMask::NO_EVENT, notify)
        .map_err(|e| conn_err(OP, "SendEvent(SelectionNotify)", e))?;
    conn.flush().map_err(|e| conn_err(OP, "flush", e))?;
    if let Some(on_read) = on_read {
        on_read();
    }
    Ok(())
}
```

- [ ] **Step 2: Build the Linux image (scratch)**

`tmp/linux/Dockerfile`:

```dockerfile
FROM rust:1.94-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends xvfb xauth xclip xdotool zenity dbus-x11 python3 libxkbcommon-x11-0 libxcursor1 libxi6 libxrandr2 libfontconfig1 libgl1 \
 && rm -rf /var/lib/apt/lists/* \
 && rustup component add clippy
WORKDIR /src
```

Run: `docker build -q -t plainpaste-linux tmp/linux; echo RC=$?`
Expected: an image id (`sha256:…`) and `RC=0`.

- [ ] **Step 3: Lint for Linux**

Run: `$DRUN -w /src plainpaste-linux cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | wc -l`
Expected: `0`. (During planning the whole tree, including `tray-icon` with the `ksni` backend, passed this check with zero warnings. The first run compiles every dependency; if it exceeds your foreground tool limit, run it detached and wait for it.)

- [ ] **Step 4: Build and run the probe under Xvfb**

`tmp/linux/probe.sh`:

```bash
#!/usr/bin/env bash
# Runs inside the plainpaste-linux container under xvfb-run. Scratch only.
# xclip -i forks into the background and serves the selection until another client takes it.
set -u
echo '<b>bold hi</b>' | xclip -selection clipboard -t text/html -silent
echo "before: $(timeout 5 xclip -selection clipboard -o -t TARGETS | tr '\n' ' ')"
timeout 20 /src/tmp/target-linux/debug/clipboard-probe
echo "RC=$?"
```

```bash
$DRUN -w /src/tmp/clipboard-probe plainpaste-linux cargo build 2>&1 | tail -1
$DRUN -w /src plainpaste-linux timeout 60 xvfb-run -a bash /src/tmp/linux/probe.sh 2>/dev/null
```

Expected: the build ends with `Finished`; the probe run prints:

```
before: TARGETS text/html
reader read: "LAZY OUT"
on_read fired: true
restored
after restore: TARGETS | text/html
RC=0
```

(The probe calls `slint::BackendSelector::new().select()` first: on X11 the reader finishes before the event loop starts, and `invoke_from_event_loop` needs an initialized backend.)

- [ ] **Step 5: Run the suite in the container**

Run: `$DRUN -w /src plainpaste-linux python3 scripts/test.py | tail -1`
Expected: `RESULT: PASS`

- [ ] **Step 6: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add src/clipboard/x11.rs
git commit -m "feat: add X11 clipboard as selection owner" -- src/clipboard/x11.rs
git status --porcelain
```

### Task 6: Flyout UI

**Files:**
- Create: `ui/flyout.slint`
- Create: `build.rs`

**Interfaces:**
- Consumes: Slint `std-widgets.slint` (`Palette`), the mockup `design/mockups/variant-b.html`.
- Produces (generated Rust, visible to the binary through `slint::include_modules!()` in Task 8):
  - `Flyout` (component, `inherits Window`) with
    - in-properties (Rust setters `set_<name>`): `version: string`, `hotkey-caps: [string]`, `recording: bool`, `hotkey-error: string`, `keep-line-breaks: bool`, `preview-in: [string]`, `preview-out: [string]`, `restore-clipboard: bool`, `theme: ThemeChoice`, `show-accessibility: bool`, `accessibility-granted: bool`, `caret-visible: bool`. `recording` also has a Rust getter `get_recording`.
    - callbacks (Rust `on_<name>`): `start-recording()`, `cancel-recording()`, `record-key(string, bool, bool, bool, bool)` (key text, control, alt, shift, meta), `reset-hotkey()`, `toggle-keep-line-breaks()`, `toggle-restore-clipboard()`, `choose-theme(ThemeChoice)`, `open-accessibility-settings()`, `quit()`.
  - `export enum ThemeChoice { system, light, dark }` → Rust `ThemeChoice::{System, Light, Dark}`.
  - `export { Palette }` → Rust `Palette` global with `set_color_scheme(slint::language::ColorScheme)`.
  - Globals `Tokens` (mockup tokens) and `Icons` (mockup SVG paths) for use inside the file.

**Behavior:**
One frameless (`no-frame: true`), always-on-top, transparent-background window, `width: 320px`, laid out top to bottom exactly like the mockup's "互動版" flyout: header (logo tile, "PlainPaste" / "純文字貼上", version pill "v" + version), hotkey card (keycaps joined by "+", or the recording state "請按下按鍵組合…" with the record icon; button 重設, or 取消 while recording; hint "點擊後按下新的組合鍵。" or, while recording, "按 Esc 取消；至少需包含一個修飾鍵（Ctrl / Alt / Shift / Win / Cmd）。"; an error line in the danger color when `hotkey-error` is not empty), keep-line-breaks card (開/關 label, switch, the two preview boxes "複製的文字" → "貼上結果" with an arrow; the output shows one line per `preview-out` item when `keep-line-breaks`, otherwise the items in one row separated by the tinted space marker; hint "開啟時，換行原樣保留。" / "關閉時，換行會以空白取代。"), restore card (開/關, switch, hint "貼上完成後放回原本的格式化內容"), theme card (segmented 跟隨系統 / 淺色 / 深色 with desktop / sun / moon icons), Accessibility card only when `show-accessibility` (granted: ok tile with shield-ok, "可模擬貼上動作", badge "已授權", normal button; not granted: warn tile with shield-err, "需授權才能模擬貼上", badge "未授權", primary button; button text "開啟系統設定"), footer button 結束.
The caret (a 10×5 px triangle, 86 px from the right edge) is drawn only when `caret-visible`; the space for it is always reserved, so the window height does not change when it toggles.
Colors: the `Tokens` global picks the mockup's light or dark token by `dark: Palette.background.to-hsv().value < 0.5`, so it follows whatever `Palette.color-scheme` Rust sets (`Unknown` = follow the OS).
Recorder: a click on the keys box calls `start-recording()` and focuses a `FocusScope`; while `recording`, Escape → `cancel-recording()`, a bare modifier key is accepted and ignored, any other key → `record-key(event.text, event.modifiers.control, .alt, .shift, .meta)`.
Icons: `Path` elements with `viewbox-width/height: 20` and the mockup's SVG path strings copied verbatim (Step 1 generates them from the HTML, so no hand transcription).

**Constraints:**
- Every color and size comes from a `Tokens` property; no raw hex or px literal outside the `Tokens` global, except the 2px input bottom border and 1px strokes which the mockup also hard-codes.
- `build.rs` compiles with `CompilerConfiguration::new().with_style("fluent".into())`; a compile error panics with the file name.
- Layer: UI only; no Rust logic in this task.

**Acceptance criteria:**
- [ ] `cargo build` succeeds (build.rs compiles `ui/flyout.slint`) and prints no `warning` line.
- [ ] `grep -c 'out property <string>' ui/flyout.slint` prints `17` (the 17 icons).
- [ ] Every Traditional Chinese string listed in Behavior appears in `ui/flyout.slint` (Step 3 checks them with one grep loop that prints `MISSING` for absent strings and nothing else).
- [ ] Visual comparison against the mockup happens in Task 8 (Acceptance there).

**Steps:**

No committed test (UI is outside the committed-test scope). Verification: the Slint compiler (via `build.rs`), a verbatim-icon check, a copy check, and the visual check in Task 8.

- [ ] **Step 1: Write the build script and verify it fails without the UI file**

`build.rs`:

```rust
fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    if let Err(e) = slint_build::compile_with_config("ui/flyout.slint", config) {
        panic!("ui/flyout.slint: expected a successful Slint compile, actual error: {e}");
    }
}
```

Run: `cargo build 2>&1 | grep -m1 "ui/flyout.slint"`
Expected: a line containing `ui/flyout.slint: expected a successful Slint compile, actual error:`

- [ ] **Step 2: Write the flyout**

`ui/flyout.slint` (the `Icons` strings are the mockup's `<symbol>` paths, copied verbatim):

```slint
// Settings flyout. Layout, copy and colors follow design/mockups/variant-b.html.
import { Palette } from "std-widgets.slint";
export { Palette }

// Fluent 2 tokens from the mockup. `dark` follows the active Fluent palette, which
// Rust drives through Palette.color-scheme (unknown = follow the system).
export global Tokens {
    out property <bool> dark: Palette.background.to-hsv().value < 0.5;
    out property <color> flyout: dark ? #2b2b2b : #f3f3f3;
    out property <color> flyout-stroke: dark ? #ffffff1f : #0000001a;
    out property <color> card: dark ? #363636 : #ffffff;
    out property <color> card-stroke: dark ? #ffffff12 : #00000012;
    out property <color> fg1: dark ? #ffffff : #242424;
    out property <color> fg2: dark ? #d6d6d6 : #525252;
    out property <color> fg3: dark ? #adadad : #707070;
    out property <color> input: dark ? #3d3d3d : #ffffff;
    out property <color> input-stroke: dark ? #5c5c5c : #d1d1d1;
    out property <color> input-bottom: dark ? #c2c2c2 : #616161;
    out property <color> btn: dark ? #404040 : #ffffff;
    out property <color> btn-hover: dark ? #4a4a4a : #f5f5f5;
    out property <color> btn-press: dark ? #363636 : #e0e0e0;
    out property <color> btn-stroke: dark ? #5c5c5c : #d1d1d1;
    out property <color> sub: dark ? #2b2b2b : #f5f5f5;
    out property <color> sub-stroke: dark ? #424242 : #ebebeb;
    out property <color> brand: dark ? #479ef5 : #0f6cbd;
    out property <color> brand-hover: dark ? #62abf5 : #115ea3;
    out property <color> brand-press: dark ? #2886de : #0c3b5e;
    out property <color> on-brand: dark ? #00182f : #ffffff;
    out property <color> brand-tint: dark ? #1f3a57 : #ebf3fc;
    out property <color> ok: dark ? #6ccb5f : #107c10;
    out property <color> ok-bg: dark ? #1d3a1b : #dff6dd;
    out property <color> warn: dark ? #f9a66a : #bc4b09;
    out property <color> warn-bg: dark ? #3d2a1b : #fdf6f3;
    out property <color> warn-stroke: dark ? #6b4a2e : #f4bfab;
    out property <color> danger: dark ? #f1707b : #c50f1f;
    out property <color> keycap: dark ? #454545 : #fafafa;
    out property <color> keycap-stroke: dark ? #5c5c5c : #d1d1d1;
    out property <color> keycap-bottom: dark ? #262626 : #b3b3b3;
    out property <color> switch-off: dark ? #d6d6d6 : #616161;
    out property <color> seg: dark ? #262626 : #ebebeb;
    out property <color> seg-on: dark ? #4a4a4a : #ffffff;

    // Layout tokens (mockup :root layout block).
    out property <length> r-panel: 20px;
    out property <length> r-card: 16px;
    out property <length> r-inner: 12px;
    out property <length> radius-control: 8px;
    out property <length> r-thumb: 6px;
    out property <length> r-tile: 10px;
    out property <length> space-1: 4px;
    out property <length> space-2: 8px;
    out property <length> space-3: 12px;
    out property <length> space-4: 16px;
    out property <length> gap-title: 2px;
    out property <length> control-h: 32px;
    out property <length> control-h-lg: 40px;
    out property <length> keycap-h: 28px;
    out property <length> chip-h: 24px;
    out property <length> tile: 32px;
    out property <length> icon: 20px;
    out property <length> icon-s: 16px;
    out property <length> switch-width: 40px;
    out property <length> switch-height: 20px;
    out property <length> thumb-off: 12px;
    out property <length> thumb-on: 14px;
    out property <length> segment-padding: 2px;
    out property <length> flyout-w: 320px;
    out property <length> caret: 10px;
    out property <length> caret-right: 86px;
    out property <length> preview-min: 80px;
    out property <length> arrow-w: 20px;
    out property <length> text-body: 14px;
    out property <length> text-small: 12px;
    out property <length> text-btn: 13px;
    out property <length> text-caption: 11px;
}

// Fluent UI System Icons (MIT), 20px viewbox, paths verbatim from the mockup.
export global Icons {
    out property <string> keyboard: "M5 12.5c0-.28.22-.5.5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M11.5 8a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5m3.75-.75a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0M5.5 8a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5m2.25 1.75a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0m2.25.75A.75.75 0 1 0 10 9a.75.75 0 0 0 0 1.5m3.76-.75a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0M8.5 8a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5M2 5.5C2 4.67 2.67 4 3.5 4h13c.83 0 1.5.67 1.5 1.5v8c0 .83-.67 1.5-1.5 1.5h-13A1.5 1.5 0 0 1 2 13.5zM3.5 5a.5.5 0 0 0-.5.5v8c0 .28.22.5.5.5h13a.5.5 0 0 0 .5-.5v-8a.5.5 0 0 0-.5-.5z";
    out property <string> undo: "M5 2.5a.5.5 0 0 0-1 0v4.9c0 .33.27.6.6.6h4.9a.5.5 0 0 0 0-1H5.9l3.48-3.02a4 4 0 0 1 5.25 6.04l-8.17 7.1a.5.5 0 0 0 .65.76l8.17-7.1a5 5 0 0 0-6.56-7.55L5 6.46V2.5Z";
    out property <string> reset: "M5.85 2.65c.2.2.2.5 0 .7L4.21 5H11a6 6 0 1 1-6 6 .5.5 0 0 1 1 0 5 5 0 1 0 5-5H4.2l1.65 1.65a.5.5 0 1 1-.7.7l-2.5-2.5a.5.5 0 0 1 0-.7l2.5-2.5c.2-.2.5-.2.7 0";
    out property <string> wrap: "M2 4.5c0-.28.22-.5.5-.5h15a.5.5 0 0 1 0 1h-15a.5.5 0 0 1-.5-.5m0 5c0-.28.22-.5.5-.5H16a3 3 0 1 1 0 6h-4.3l.65.65a.5.5 0 0 1-.7.7l-1.5-1.5a.5.5 0 0 1 0-.7l1.5-1.5a.5.5 0 0 1 .7.7l-.64.65H16a2 2 0 1 0 0-4H2.5a.5.5 0 0 1-.5-.5m0 5c0-.28.22-.5.5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5";
    out property <string> shield-ok: "M9.72 2.08a.5.5 0 0 1 .56 0q2.9 1.94 6.3 2.43A.5.5 0 0 1 17 5v4.6q-.48-.25-1-.4V5.44a16 16 0 0 1-5.6-2.08L10 3.1l-.4.25A16 16 0 0 1 4 5.43V9.5c0 3.24 1.8 5.64 5.48 7.25q.3.65.74 1.2l-.04.02a.5.5 0 0 1-.36 0C5.31 16.23 3 13.39 3 9.5V5a.5.5 0 0 1 .43-.5 15 15 0 0 0 6.3-2.42M19 14.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0m-2.15-1.85a.5.5 0 0 0-.7 0l-2.65 2.64-.65-.64a.5.5 0 0 0-.7.7l1 1c.2.2.5.2.7 0l3-3a.5.5 0 0 0 0-.7";
    out property <string> shield-err: "M10 6c.28 0 .5.22.5.5v5a.5.5 0 0 1-1 0v-5c0-.28.22-.5.5-.5m0 8.5a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5M9.72 2.08a.5.5 0 0 1 .56 0q2.9 1.94 6.3 2.43A.5.5 0 0 1 17 5v4.5c0 3.9-2.3 6.73-6.82 8.47a.5.5 0 0 1-.36 0C5.31 16.23 3 13.39 3 9.5V5a.5.5 0 0 1 .43-.5 15 15 0 0 0 6.3-2.42M9.6 3.35A16 16 0 0 1 4 5.43V9.5c0 3.4 1.97 5.86 6 7.46 4.03-1.6 6-4.07 6-7.46V5.43a16 16 0 0 1-5.6-2.08L10 3.1z";
    out property <string> power: "M10.5 2.5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0zM13.74 4a.5.5 0 1 0-.5.87 6.5 6.5 0 1 1-6.49 0 .5.5 0 1 0-.5-.87 7.5 7.5 0 1 0 7.5 0";
    out property <string> moon: "M15.5 13.5A6.98 6.98 0 0 1 4 14.39c2.83-1.09 4.56-2.42 5.6-4.4 1.04-2 1.33-4.16.75-6.9q1.36.17 2.58.87a7 7 0 0 1 2.55 9.54M5.45 16.92A7.98 7.98 0 1 0 9.88 2.04a.6.6 0 0 0-.61.73c.69 2.82.43 4.88-.55 6.76-.94 1.78-2.55 3.03-5.55 4.1a.6.6 0 0 0-.3.9 8 8 0 0 0 2.59 2.39";
    out property <string> sun: "M10 2c.28 0 .5.22.5.5v1a.5.5 0 0 1-1 0v-1c0-.28.22-.5.5-.5m0 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8m0-1a3 3 0 1 1 0-6 3 3 0 0 1 0 6m7.5-2.5a.5.5 0 0 0 0-1h-1a.5.5 0 0 0 0 1zM10 16c.28 0 .5.22.5.5v1a.5.5 0 0 1-1 0v-1c0-.28.22-.5.5-.5m-6.5-5.5a.5.5 0 0 0 0-1H2.46a.5.5 0 0 0 0 1zm.65-6.35c.2-.2.5-.2.7 0l1 1a.5.5 0 1 1-.7.7l-1-1a.5.5 0 0 1 0-.7m.7 11.7a.5.5 0 0 1-.7-.7l1-1a.5.5 0 0 1 .7.7zm11-11.7a.5.5 0 0 0-.7 0l-1 1a.5.5 0 0 0 .7.7l1-1a.5.5 0 0 0 0-.7m-.7 11.7a.5.5 0 0 0 .7-.7l-1-1a.5.5 0 0 0-.7.7z";
    out property <string> desktop: "M4 2a2 2 0 0 0-2 2v9c0 1.1.9 2 2 2h3v2H5.5a.5.5 0 0 0 0 1h9a.5.5 0 0 0 0-1H13v-2h3a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zm8 13v2H8v-2zM3 4a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z";
    out property <string> check: "M10 2a8 8 0 1 1 0 16 8 8 0 0 1 0-16m3.36 5.65a.5.5 0 0 0-.64-.06l-.07.06L9 11.3 7.35 9.65l-.07-.06a.5.5 0 0 0-.7.7l.07.07 2 2 .07.06c.17.11.4.11.56 0l.07-.06 4-4 .07-.08a.5.5 0 0 0-.06-.63";
    out property <string> warn: "M7.37 3.56a3 3 0 0 1 5.26 0l5.5 10A3 3 0 0 1 15.5 18h-11a3 3 0 0 1-2.63-4.44zM10 12.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5m0-6.25a.5.5 0 0 0-.5.5v4a.5.5 0 0 0 1 0V7a.5.5 0 0 0-.5-.5";
    out property <string> paste: "M4.5 4h1.59c.2.58.76 1 1.41 1h3c.65 0 1.2-.42 1.41-1h1.59c.28 0 .5.22.5.5v1a.5.5 0 0 0 1 0v-1c0-.83-.67-1.5-1.5-1.5h-1.59c-.2-.58-.76-1-1.41-1h-3c-.65 0-1.2.42-1.41 1H4.5C3.67 3 3 3.67 3 4.5v12c0 .83.67 1.5 1.5 1.5h3a.5.5 0 0 0 0-1h-3a.5.5 0 0 1-.5-.5v-12c0-.28.22-.5.5-.5m3 0a.5.5 0 0 1 0-1h3a.5.5 0 0 1 0 1zm3 3C9.67 7 9 7.67 9 8.5v8c0 .83.67 1.5 1.5 1.5h5c.83 0 1.5-.67 1.5-1.5v-8c0-.83-.67-1.5-1.5-1.5z";
    out property <string> open: "M6 4a2 2 0 0 0-2 2v8c0 1.1.9 2 2 2h8a2 2 0 0 0 2-2v-2.5a.5.5 0 0 1 1 0V14a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3V6a3 3 0 0 1 3-3h2.5a.5.5 0 0 1 0 1zm5-.5c0-.28.22-.5.5-.5h5c.28 0 .5.22.5.5v5a.5.5 0 0 1-1 0V4.7l-4.15 4.15a.5.5 0 0 1-.7-.7L15.29 4H11.5a.5.5 0 0 1-.5-.5";
    out property <string> palette: "M10 2a8 8 0 0 1 3.39 15.25c-1.09.5-2.33.44-3.3-.06a2.9 2.9 0 0 1-1.64-2.73q.04-.68.14-1.24.1-.59.14-1.02.03-.44-.08-.73-.09-.3-.45-.57a1 1 0 0 0-.84-.24q-.43.09-1.02.39c-.73.36-1.77.94-2.97.33-1.1-.56-1.43-1.87-1.22-2.92A8 8 0 0 1 10 2m0 1a7 7 0 0 0-6.87 5.66c-.15.78.13 1.54.69 1.83.68.35 1.25.07 2.08-.34.38-.18.83-.4 1.31-.48.52-.07 1.06.03 1.59.43q.58.44.79 1.02c.14.38.16.77.14 1.14s-.1.75-.15 1.12q-.1.55-.13 1.12c-.03.8.42 1.44 1.1 1.8.7.36 1.62.42 2.41.04A7 7 0 0 0 10 3m2.5 10a1 1 0 1 1 0 2 1 1 0 0 1 0-2m2-2.5a1 1 0 1 1 0 2 1 1 0 0 1 0-2m0-3a1 1 0 1 1 0 2 1 1 0 0 1 0-2m-2-2.5a1 1 0 1 1 0 2 1 1 0 0 1 0-2m-3-.5a1 1 0 1 1 0 2 1 1 0 0 1 0-2";
    out property <string> record: "M2 10a8 8 0 1 1 16 0 8 8 0 0 1-16 0m8 4.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9";
    out property <string> arrow: "M10.84 3.13a.5.5 0 0 0-.68.74l6.17 5.63H2.5a.5.5 0 0 0 0 1h13.83l-6.17 5.63a.5.5 0 0 0 .68.74l6.91-6.32a.75.75 0 0 0 0-1.1z";
}

component Icon inherits Path {
    in property <string> path;
    in property <length> size: Tokens.icon;
    width: size;
    height: size;
    viewbox-width: 20;
    viewbox-height: 20;
    commands: path;
}

component Tile inherits Rectangle {
    in property <string> icon;
    in property <color> tint: Tokens.brand-tint;
    in property <color> ink: Tokens.brand;
    width: Tokens.tile;
    height: Tokens.tile;
    border-radius: Tokens.r-tile;
    background: tint;
    Icon { path: icon; fill: ink; }
}

component Card inherits Rectangle {
    background: Tokens.card;
    border-color: Tokens.card-stroke;
    border-width: 1px;
    border-radius: Tokens.r-card;
    VerticalLayout {
        padding: Tokens.space-4;
        spacing: Tokens.space-3;
        @children
    }
}

component RowLabel inherits VerticalLayout {
    in property <string> title;
    in property <string> subtitle;
    horizontal-stretch: 1;
    spacing: Tokens.gap-title;
    alignment: center;
    Text { text: title; font-size: Tokens.text-body; font-weight: 600; color: Tokens.fg1; }
    if subtitle != "": Text { text: subtitle; font-size: Tokens.text-small; color: Tokens.fg2; }
}

component Hint inherits Text {
    font-size: Tokens.text-small;
    color: Tokens.fg2;
    wrap: word-wrap;
}

component ActionButton inherits Rectangle {
    in property <string> label;
    in property <string> icon;
    in property <bool> primary;
    in property <length> h: Tokens.control-h;
    callback clicked();
    height: h;
    border-radius: Tokens.radius-control;
    border-width: 1px;
    background: primary
        ? (touch.pressed ? Tokens.brand-press : touch.has-hover ? Tokens.brand-hover : Tokens.brand)
        : (touch.pressed ? Tokens.btn-press : touch.has-hover ? Tokens.btn-hover : Tokens.btn);
    border-color: primary ? self.background : Tokens.btn-stroke;
    HorizontalLayout {
        padding-left: Tokens.space-3;
        padding-right: Tokens.space-3;
        spacing: Tokens.space-2;
        alignment: center;
        if icon != "": Icon { path: icon; size: Tokens.icon-s; fill: primary ? Tokens.on-brand : Tokens.fg1; y: (parent.height - self.height) / 2; }
        Text {
            text: label;
            font-size: Tokens.text-btn;
            font-weight: primary ? 600 : 400;
            color: primary ? Tokens.on-brand : Tokens.fg1;
            vertical-alignment: center;
        }
    }
    touch := TouchArea { clicked => { root.clicked(); } }
}

component Switch inherits Rectangle {
    in property <bool> checked;
    callback toggled();
    width: Tokens.switch-width;
    height: Tokens.switch-height;
    border-radius: self.height / 2;
    border-width: 1px;
    border-color: checked ? Tokens.brand : Tokens.switch-off;
    background: checked ? Tokens.brand : transparent;
    Rectangle {
        width: checked ? Tokens.thumb-on : Tokens.thumb-off;
        height: self.width;
        border-radius: self.width / 2;
        background: checked ? Tokens.on-brand : Tokens.switch-off;
        x: checked ? parent.width - self.width - (parent.height - self.width) / 2 : (parent.height - self.width) / 2;
        y: (parent.height - self.height) / 2;
    }
    TouchArea { clicked => { root.toggled(); } }
}

component Keycap inherits Rectangle {
    in property <string> label;
    height: Tokens.keycap-h;
    min-width: Tokens.keycap-h;
    border-radius: Tokens.r-thumb;
    border-width: 1px;
    border-color: Tokens.keycap-stroke;
    background: Tokens.keycap;
    HorizontalLayout {
        padding-left: Tokens.space-2;
        padding-right: Tokens.space-2;
        Text { text: label; font-size: Tokens.text-small; font-weight: 600; color: Tokens.fg1; vertical-alignment: center; horizontal-alignment: center; }
    }
}

component SegmentButton inherits Rectangle {
    in property <string> label;
    in property <string> icon;
    in property <bool> selected;
    callback clicked();
    border-radius: Tokens.r-thumb;
    border-width: 1px;
    border-color: selected ? Tokens.card-stroke : transparent;
    background: selected ? Tokens.seg-on : transparent;
    HorizontalLayout {
        alignment: center;
        spacing: Tokens.space-1;
        Icon { path: icon; size: Tokens.icon-s; fill: selected ? Tokens.brand : (touch.has-hover ? Tokens.fg1 : Tokens.fg2); y: (parent.height - self.height) / 2; }
        Text {
            text: label;
            font-size: Tokens.text-small;
            font-weight: selected ? 600 : 400;
            color: selected || touch.has-hover ? Tokens.fg1 : Tokens.fg2;
            vertical-alignment: center;
        }
    }
    touch := TouchArea { clicked => { root.clicked(); } }
}

export enum ThemeChoice { system, light, dark }

export component Flyout inherits Window {
    // Rust -> UI state.
    in property <string> version;
    in property <[string]> hotkey-caps;
    in property <bool> recording;
    in property <string> hotkey-error;
    in property <bool> keep-line-breaks;
    in property <[string]> preview-in;
    in property <[string]> preview-out;
    in property <bool> restore-clipboard;
    in property <ThemeChoice> theme;
    in property <bool> show-accessibility;
    in property <bool> accessibility-granted;
    in property <bool> caret-visible;

    // UI -> Rust intents.
    callback start-recording();
    callback cancel-recording();
    callback record-key(string, bool, bool, bool, bool);
    callback reset-hotkey();
    callback toggle-keep-line-breaks();
    callback toggle-restore-clipboard();
    callback choose-theme(ThemeChoice);
    callback open-accessibility-settings();
    callback quit();

    title: "PlainPaste";
    no-frame: true;
    always-on-top: true;
    background: transparent;
    width: Tokens.flyout-w;

    if caret-visible: Rectangle {
        x: parent.width - Tokens.caret-right - Tokens.caret;
        y: 0;
        width: Tokens.caret;
        height: Tokens.caret / 2;
        Path {
            width: 100%;
            height: 100%;
            viewbox-width: 2;
            viewbox-height: 1;
            commands: "M0 1 L1 0 L2 1 Z";
            fill: Tokens.flyout;
            stroke: Tokens.flyout-stroke;
            stroke-width: 1px;
        }
    }

    VerticalLayout {
        padding-top: caret-visible ? Tokens.caret / 2 - 1px : 0;
        Rectangle {
            background: Tokens.flyout;
            border-color: Tokens.flyout-stroke;
            border-width: 1px;
            border-radius: Tokens.r-panel;
            VerticalLayout {
                padding: Tokens.space-4;
                spacing: Tokens.space-3;

                // Header.
                HorizontalLayout {
                    spacing: Tokens.space-3;
                    Rectangle {
                        width: Tokens.tile;
                        height: Tokens.tile;
                        border-radius: Tokens.r-tile;
                        background: Tokens.brand;
                        Icon { path: Icons.paste; fill: Tokens.on-brand; }
                    }
                    RowLabel { title: "PlainPaste"; subtitle: "純文字貼上"; }
                    Rectangle {
                        height: Tokens.chip-h;
                        y: (parent.height - self.height) / 2;
                        border-radius: self.height / 2;
                        background: Tokens.brand-tint;
                        HorizontalLayout {
                            padding-left: Tokens.space-3;
                            padding-right: Tokens.space-3;
                            Text { text: "v" + version; font-size: Tokens.text-small; font-weight: 600; color: Tokens.brand; vertical-alignment: center; }
                        }
                    }
                }

                // Hotkey recorder.
                Card {
                    HorizontalLayout {
                        spacing: Tokens.space-3;
                        Tile { icon: Icons.keyboard; }
                        RowLabel { title: "貼上快捷鍵"; }
                    }
                    VerticalLayout {
                        spacing: Tokens.space-2;
                        HorizontalLayout {
                            spacing: Tokens.space-2;
                            keys := Rectangle {
                                horizontal-stretch: 1;
                                height: Tokens.control-h-lg;
                                border-radius: Tokens.radius-control;
                                border-width: 1px;
                                border-color: recording ? Tokens.brand : Tokens.input-stroke;
                                background: recording ? Tokens.brand-tint : Tokens.input;
                                Rectangle {
                                    y: parent.height - 2px;
                                    height: 2px;
                                    background: recording || keys-touch.has-hover ? Tokens.brand : Tokens.input-bottom;
                                }
                                HorizontalLayout {
                                    alignment: center;
                                    spacing: Tokens.space-2;
                                    padding-left: Tokens.space-3;
                                    padding-right: Tokens.space-3;
                                    if recording: Icon { path: Icons.record; size: Tokens.icon-s; fill: Tokens.danger; y: (parent.height - self.height) / 2; }
                                    if recording: Text { text: "請按下按鍵組合…"; font-size: Tokens.text-btn; font-weight: 600; color: Tokens.brand; vertical-alignment: center; }
                                    for cap[index] in (recording ? [] : hotkey-caps): HorizontalLayout {
                                        spacing: Tokens.space-2;
                                        if index > 0: Text { text: "+"; font-size: Tokens.text-small; color: Tokens.fg3; vertical-alignment: center; }
                                        Keycap { label: cap; y: (parent.height - self.height) / 2; }
                                    }
                                }
                                keys-touch := TouchArea {
                                    clicked => {
                                        if (!recording) {
                                            root.start-recording();
                                            key-input.focus();
                                        }
                                    }
                                }
                                key-input := FocusScope {
                                    key-pressed(event) => {
                                        if (!recording) { return reject; }
                                        if (event.text == Key.Escape) {
                                            root.cancel-recording();
                                            return accept;
                                        }
                                        if (event.text == Key.Shift || event.text == Key.Control || event.text == Key.Alt || event.text == Key.Meta) {
                                            return accept;
                                        }
                                        root.record-key(event.text, event.modifiers.control, event.modifiers.alt, event.modifiers.shift, event.modifiers.meta);
                                        accept
                                    }
                                }
                            }
                            if recording: ActionButton { label: "取消"; icon: Icons.reset; h: Tokens.control-h-lg; clicked => { root.cancel-recording(); } }
                            if !recording: ActionButton { label: "重設"; icon: Icons.reset; h: Tokens.control-h-lg; clicked => { root.reset-hotkey(); } }
                        }
                        if hotkey-error != "": Hint { text: hotkey-error; color: Tokens.danger; }
                        Hint {
                            text: recording
                                ? "按 Esc 取消；至少需包含一個修飾鍵（Ctrl / Alt / Shift / Win / Cmd）。"
                                : "點擊後按下新的組合鍵。";
                        }
                    }
                }

                // Keep line breaks.
                Card {
                    HorizontalLayout {
                        spacing: Tokens.space-3;
                        Tile { icon: Icons.wrap; }
                        RowLabel { title: "保留換行"; }
                        Text { text: keep-line-breaks ? "開" : "關"; font-size: Tokens.text-small; color: Tokens.fg2; vertical-alignment: center; }
                        Switch { checked: keep-line-breaks; y: (parent.height - self.height) / 2; toggled => { root.toggle-keep-line-breaks(); } }
                    }
                    VerticalLayout {
                        spacing: Tokens.space-2;
                        HorizontalLayout {
                            spacing: Tokens.space-1;
                            Rectangle {
                                horizontal-stretch: 1;
                                min-height: Tokens.preview-min;
                                border-radius: Tokens.r-inner;
                                border-width: 1px;
                                border-color: Tokens.sub-stroke;
                                background: Tokens.sub;
                                VerticalLayout {
                                    padding-top: Tokens.space-3;
                                    padding-bottom: Tokens.space-3;
                                    padding-left: Tokens.space-2;
                                    padding-right: Tokens.space-2;
                                    spacing: Tokens.space-1;
                                    alignment: start;
                                    Text { text: "複製的文字"; font-size: Tokens.text-caption; font-weight: 600; color: Tokens.fg3; }
                                    for line in preview-in: Text { text: line; font-size: Tokens.text-small; color: Tokens.fg1; }
                                }
                            }
                            Icon { path: Icons.arrow; size: Tokens.icon-s; fill: Tokens.fg3; width: Tokens.arrow-w; y: (parent.height - self.height) / 2; }
                            Rectangle {
                                horizontal-stretch: 1;
                                min-height: Tokens.preview-min;
                                border-radius: Tokens.r-inner;
                                border-width: 1px;
                                border-color: Tokens.sub-stroke;
                                background: Tokens.sub;
                                VerticalLayout {
                                    padding-top: Tokens.space-3;
                                    padding-bottom: Tokens.space-3;
                                    padding-left: Tokens.space-2;
                                    padding-right: Tokens.space-2;
                                    spacing: Tokens.space-1;
                                    alignment: start;
                                    Text { text: "貼上結果"; font-size: Tokens.text-caption; font-weight: 600; color: Tokens.fg3; }
                                    if keep-line-breaks: VerticalLayout {
                                        for line in preview-out: Text { text: line; font-size: Tokens.text-small; color: Tokens.fg1; }
                                    }
                                    if !keep-line-breaks: HorizontalLayout {
                                        alignment: start;
                                        for word[index] in preview-out: HorizontalLayout {
                                            if index > 0: Rectangle {
                                                width: 0.5 * Tokens.text-small;
                                                height: 0.75 * Tokens.text-small;
                                                y: (parent.height - self.height) / 2;
                                                border-radius: Tokens.r-thumb;
                                                background: Tokens.brand.transparentize(65%);
                                            }
                                            Text { text: word; font-size: Tokens.text-small; color: Tokens.fg1; }
                                        }
                                    }
                                }
                            }
                        }
                        Hint { text: keep-line-breaks ? "開啟時，換行原樣保留。" : "關閉時，換行會以空白取代。"; }
                    }
                }

                // Restore clipboard.
                Card {
                    HorizontalLayout {
                        spacing: Tokens.space-3;
                        Tile { icon: Icons.undo; }
                        RowLabel { title: "貼上後還原剪貼簿"; }
                        Text { text: restore-clipboard ? "開" : "關"; font-size: Tokens.text-small; color: Tokens.fg2; vertical-alignment: center; }
                        Switch { checked: restore-clipboard; y: (parent.height - self.height) / 2; toggled => { root.toggle-restore-clipboard(); } }
                    }
                    Hint { text: "貼上完成後放回原本的格式化內容"; }
                }

                // Theme.
                Card {
                    HorizontalLayout {
                        spacing: Tokens.space-3;
                        Tile { icon: Icons.palette; }
                        RowLabel { title: "外觀主題"; }
                    }
                    Rectangle {
                        height: Tokens.control-h;
                        border-radius: Tokens.radius-control;
                        background: Tokens.seg;
                        HorizontalLayout {
                            padding: Tokens.segment-padding;
                            spacing: Tokens.segment-padding;
                            SegmentButton { label: "跟隨系統"; icon: Icons.desktop; selected: theme == ThemeChoice.system; clicked => { root.choose-theme(ThemeChoice.system); } }
                            SegmentButton { label: "淺色"; icon: Icons.sun; selected: theme == ThemeChoice.light; clicked => { root.choose-theme(ThemeChoice.light); } }
                            SegmentButton { label: "深色"; icon: Icons.moon; selected: theme == ThemeChoice.dark; clicked => { root.choose-theme(ThemeChoice.dark); } }
                        }
                    }
                }

                // macOS Accessibility permission.
                if show-accessibility: Card {
                    HorizontalLayout {
                        spacing: Tokens.space-3;
                        Tile {
                            icon: accessibility-granted ? Icons.shield-ok : Icons.shield-err;
                            tint: accessibility-granted ? Tokens.ok-bg : Tokens.warn-bg;
                            ink: accessibility-granted ? Tokens.ok : Tokens.warn;
                        }
                        RowLabel { title: "輔助使用權限"; subtitle: accessibility-granted ? "可模擬貼上動作" : "需授權才能模擬貼上"; }
                    }
                    HorizontalLayout {
                        Rectangle {
                            height: Tokens.chip-h;
                            y: (parent.height - self.height) / 2;
                            border-radius: self.height / 2;
                            border-width: accessibility-granted ? 0 : 1px;
                            border-color: Tokens.warn-stroke;
                            background: accessibility-granted ? Tokens.ok-bg : Tokens.warn-bg;
                            HorizontalLayout {
                                padding-left: Tokens.space-2;
                                padding-right: Tokens.space-3;
                                spacing: Tokens.space-1;
                                Icon {
                                    path: accessibility-granted ? Icons.check : Icons.warn;
                                    size: Tokens.icon-s;
                                    fill: accessibility-granted ? Tokens.ok : Tokens.warn;
                                    y: (parent.height - self.height) / 2;
                                }
                                Text {
                                    text: accessibility-granted ? "已授權" : "未授權";
                                    font-size: Tokens.text-small;
                                    font-weight: 600;
                                    color: accessibility-granted ? Tokens.ok : Tokens.warn;
                                    vertical-alignment: center;
                                }
                            }
                        }
                        Rectangle { horizontal-stretch: 1; }
                        ActionButton { label: "開啟系統設定"; icon: Icons.open; primary: !accessibility-granted; clicked => { root.open-accessibility-settings(); } }
                    }
                }

                // Footer.
                HorizontalLayout {
                    alignment: end;
                    ActionButton { label: "結束"; icon: Icons.power; clicked => { root.quit(); } }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Build and check icons and copy**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" | wc -l`
Expected: `0`

Run the verbatim-icon check (every icon path in the .slint file must equal the mockup path with the same id):

```bash
python3 -c "
import re
html = open('design/mockups/variant-b.html', encoding='utf-8').read()
slint = open('ui/flyout.slint', encoding='utf-8').read()
mock = dict(re.findall(r'<symbol id=\"i-([a-z-]+)\"\s*viewBox=\"0 0 20 20\"><path d=\"([^\"]+)\"/></symbol>', html))
ours = dict(re.findall(r'out property <string> ([a-z-]+): \"([^\"]+)\";', slint))
bad = [k for k, v in ours.items() if mock.get(k) != v]
print('icons:', len(ours), 'mismatched:', bad)
"
```

Expected: `icons: 17 mismatched: []`

Run the copy check:

```bash
for s in "PlainPaste" "純文字貼上" "貼上快捷鍵" "請按下按鍵組合…" "取消" "重設" "點擊後按下新的組合鍵。" "按 Esc 取消；至少需包含一個修飾鍵（Ctrl / Alt / Shift / Win / Cmd）。" "保留換行" "複製的文字" "貼上結果" "開啟時，換行原樣保留。" "關閉時，換行會以空白取代。" "貼上後還原剪貼簿" "貼上完成後放回原本的格式化內容" "外觀主題" "跟隨系統" "淺色" "深色" "輔助使用權限" "可模擬貼上動作" "需授權才能模擬貼上" "已授權" "未授權" "開啟系統設定" "結束"; do grep -qF "$s" ui/flyout.slint || echo "MISSING $s"; done
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add build.rs ui/flyout.slint
git commit -m "feat: add settings flyout UI from the approved mockup" -- build.rs ui/flyout.slint
git status --porcelain
```

### Task 7: Hotkey and paste services

**Files:**
- Create: `src/hotkey.rs`
- Create: `src/paste.rs`
- Modify: `src/lib.rs` (add `pub mod hotkey;` and `pub mod paste;`)
- Scratch (not committed): `tmp/paste-probe/Cargo.toml`, `tmp/paste-probe/src/main.rs`

**Interfaces:**
- Consumes: `global_hotkey` (`GlobalHotKeyManager`, `HotKeyState`, `Error`, `hotkey::{HotKey, Modifiers}`), `enigo` (`Enigo`, `Settings`, `Keyboard`, `Key`, `Direction`, `NewConError`, `InputError`), `slint::platform::Key` (F1–F12 key texts).
- Produces:
  - `plainpaste::hotkey`:
    - `pub use global_hotkey::Error as HotkeyError;`
    - `pub struct HotkeyService` with `pub fn new(initial: HotKey) -> Result<Self, HotkeyError>`, `pub fn matches_release_event(&self, id: u32, state: HotKeyState) -> bool`, `pub fn suspend(&mut self) -> Result<(), HotkeyError>`, `pub fn register(&mut self, hotkey: HotKey) -> Result<(), HotkeyError>`.
  - `crate::app::recorder` (binary crate, `src/app/recorder.rs`, private to `app`):
    - `pub fn hotkey_from_key_press(text: &str, control: bool, alt: bool, shift: bool, meta: bool) -> Option<HotKey>`
    - `pub fn keycap_labels(hotkey: HotKey) -> Vec<String>`
  - `plainpaste::paste`:
    - `pub enum PasteError { NoPermission, Connect(NewConError), Input { step: &'static str, source: InputError }, OpenSettings(std::io::Error) }` with `Display`, `std::error::Error`, `Debug`.
    - `pub fn accessibility_granted() -> Result<bool, PasteError>`
    - `pub fn open_accessibility_settings() -> Result<(), PasteError>`
    - `pub fn simulate_paste(hotkey_modifiers: Modifiers) -> Result<(), PasteError>`

**Behavior:**
`HotkeyService` owns the `GlobalHotKeyManager` (create it on the main thread) and at most one registered hotkey. `suspend` unregisters it (used while recording). `register(new)` first suspends, then registers `new`; on failure nothing is registered and the error is returned, so the caller re-registers the previous hotkey. `matches_release_event` is true only for `HotKeyState::Released` of the registered hotkey's id (the spec triggers on Released).
`hotkey_from_key_press` maps Slint's modifiers to global-hotkey's: Slint reports Command as `control` and Control as `meta` on macOS, so on macOS `control → META`, `meta → CONTROL`; elsewhere `control → CONTROL`, `meta → META`; `alt → ALT`, `shift → SHIFT`. No modifier → `None`. Key text: one ASCII letter (any case) → `Key<UPPER>`, one digit → `Digit<d>`, `" "` → `Space`, the Slint F1–F12 key characters → `F1`–`F12`; any other text → `None`. The token is parsed with global-hotkey's own parser (`"KeyV".parse::<HotKey>()`) and only its key code is used.
`keycap_labels` lists the modifiers in the order Meta, Ctrl, Alt, Shift (Meta is "Cmd" on macOS, "Win" elsewhere), then the key with a `Key`/`Digit` prefix removed (`KeyV` → `V`, `Digit1` → `1`, `F5` → `F5`). Default on macOS: `["Cmd", "Shift", "V"]`; elsewhere `["Ctrl", "Shift", "V"]`.
`accessibility_granted` creates an `Enigo` with `open_prompt_to_get_permissions: false`: `NewConError::NoPermission` → `Ok(false)` (only macOS can return it), success → `Ok(true)`, any other error → `Err`.
`simulate_paste` releases each modifier contained in the hotkey's modifiers (Shift, Control, Alt, Meta), then presses the paste modifier (Meta on macOS, Control elsewhere), clicks `Key::Unicode('v')`, releases the paste modifier. Each failing key event returns `PasteError::Input` naming the step.
`open_accessibility_settings` runs `open x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` (only reachable from the macOS-only card).

**Constraints:**
- No `#[cfg]` items: per-OS values use `cfg!(target_os = "macos")`.
- Layer: library; depends on global-hotkey, enigo, slint only.

**Acceptance criteria:**
- [ ] `cargo clippy --all-targets` prints no `warning` line.
- [ ] Paste probe on macOS (Step 4): with TextEdit focused, the probe pastes the current clipboard text into TextEdit and prints `paste probe: ok`; `accessibility granted: true` is printed first. With the terminal app removed from System Settings > Privacy & Security > Accessibility, the probe prints `accessibility granted: false`.
- [ ] `python3 scripts/test.py` ends with `RESULT: PASS`.

**Steps:**

No committed test (OS input and hotkey registration are outside the committed-test scope). Verification: clippy plus the scratch paste probe.

- [ ] **Step 1: Register the modules**

Add two lines to `src/lib.rs` (keep the list sorted):

```rust
pub mod hotkey;
pub mod paste;
```

Run: `cargo build 2>&1 | grep -E "^error" | head -2`
Expected: `error[E0583]: file not found for module `hotkey`` (and the same for `paste`).

- [ ] **Step 2: Implement the hotkey service**

`src/hotkey.rs`:

```rust
//! Global paste hotkey: registration, replacement, and recorder key mapping.

use global_hotkey::hotkey::{HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyManager, HotKeyState};

pub use global_hotkey::Error as HotkeyError;

const DIGITS: &str = "0123456789";
const FUNCTION_KEY_COUNT: u8 = 12;
const SPACE: &str = " ";
const LABEL_SUPER_MAC: &str = "Cmd";
const LABEL_SUPER_OTHER: &str = "Win";

/// Owns the OS registration. At most one hotkey is registered at a time.
pub struct HotkeyService {
    manager: GlobalHotKeyManager,
    registered: Option<HotKey>,
}

impl HotkeyService {
    pub fn new(initial: HotKey) -> Result<Self, HotkeyError> {
        let manager = GlobalHotKeyManager::new()?;
        manager.register(initial)?;
        Ok(HotkeyService {
            manager,
            registered: Some(initial),
        })
    }

    /// True when `id`/`state` is the Released event of the registered hotkey.
    pub fn matches_release_event(&self, id: u32, state: HotKeyState) -> bool {
        state == HotKeyState::Released && self.registered.is_some_and(|h| h.id() == id)
    }

    /// Unregisters the current hotkey, so the recorder can capture its keys.
    pub fn suspend(&mut self) -> Result<(), HotkeyError> {
        if let Some(current) = self.registered.take() {
            self.manager.unregister(current)?;
        }
        Ok(())
    }

    /// Registers `hotkey`. On failure nothing is registered.
    pub fn register(&mut self, hotkey: HotKey) -> Result<(), HotkeyError> {
        self.suspend()?;
        self.manager.register(hotkey)?;
        self.registered = Some(hotkey);
        Ok(())
    }
}

/// Converts a recorder key press into a hotkey.
/// `control`/`alt`/`shift`/`meta` are Slint's KeyboardModifiers. On macOS Slint reports
/// Command as `control` and Control as `meta`. global-hotkey calls Command META.
/// Returns `None` when there is no modifier or the key has no hotkey code.
pub fn hotkey_from_key_press(
    text: &str,
    control: bool,
    alt: bool,
    shift: bool,
    meta: bool,
) -> Option<HotKey> {
    let (command_or_control, control_or_super) = if cfg!(target_os = "macos") {
        (Modifiers::META, Modifiers::CONTROL)
    } else {
        (Modifiers::CONTROL, Modifiers::META)
    };
    let mut mods = Modifiers::empty();
    if control {
        mods |= command_or_control;
    }
    if meta {
        mods |= control_or_super;
    }
    if alt {
        mods |= Modifiers::ALT;
    }
    if shift {
        mods |= Modifiers::SHIFT;
    }
    if mods.is_empty() {
        return None;
    }
    // A single token parses as a hotkey without modifiers. Only its key code is used.
    let key = key_token(text)?.parse::<HotKey>().ok()?.key;
    Some(HotKey::new(Some(mods), key))
}

/// global-hotkey key token for a Slint key text, for example "v" -> "KeyV", F5 -> "F5".
fn key_token(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if c.is_ascii_alphabetic() {
        return Some(format!("Key{}", c.to_ascii_uppercase()));
    }
    if DIGITS.contains(c) {
        return Some(format!("Digit{c}"));
    }
    if text == SPACE {
        return Some("Space".to_string());
    }
    (1..=FUNCTION_KEY_COUNT)
        .zip(FUNCTION_KEYS)
        .find(|(_, key)| char::from(*key) == c)
        .map(|(n, _)| format!("F{n}"))
}

const FUNCTION_KEYS: [slint::platform::Key; FUNCTION_KEY_COUNT as usize] = [
    slint::platform::Key::F1,
    slint::platform::Key::F2,
    slint::platform::Key::F3,
    slint::platform::Key::F4,
    slint::platform::Key::F5,
    slint::platform::Key::F6,
    slint::platform::Key::F7,
    slint::platform::Key::F8,
    slint::platform::Key::F9,
    slint::platform::Key::F10,
    slint::platform::Key::F11,
    slint::platform::Key::F12,
];

/// Keycap labels in display order, for example ["Ctrl", "Shift", "V"].
pub fn keycap_labels(hotkey: HotKey) -> Vec<String> {
    let super_label = if cfg!(target_os = "macos") {
        LABEL_SUPER_MAC
    } else {
        LABEL_SUPER_OTHER
    };
    let mut labels: Vec<String> = [
        (Modifiers::META, super_label),
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
    ]
    .into_iter()
    .filter(|(m, _)| hotkey.mods.contains(*m))
    .map(|(_, label)| label.to_string())
    .collect();
    let code = hotkey.key.to_string();
    let key = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(&code)
        .to_string();
    labels.push(key);
    labels
}
```

- [ ] **Step 3: Implement the paste service**

`src/paste.rs`:

```rust
//! Simulated paste keystroke and the input-simulation permission check.

use std::fmt;

use enigo::{Direction, Enigo, InputError, Key, Keyboard, NewConError, Settings};
use global_hotkey::hotkey::Modifiers;

const PASTE_KEY: char = 'v';
/// System Settings > Privacy & Security > Accessibility. Only opened on macOS.
const ACCESSIBILITY_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
const URL_OPENER: &str = "open";

#[derive(Debug)]
pub enum PasteError {
    /// macOS Accessibility permission is not granted.
    NoPermission,
    Connect(NewConError),
    Input {
        step: &'static str,
        source: InputError,
    },
    OpenSettings(std::io::Error),
}

impl fmt::Display for PasteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PasteError::NoPermission => {
                write!(
                    f,
                    "paste: expected Accessibility permission, actual not granted"
                )
            }
            PasteError::Connect(e) => write!(f, "paste: input simulation connection failed: {e}"),
            PasteError::Input { step, source } => {
                write!(f, "paste: key event `{step}` failed: {source}")
            }
            PasteError::OpenSettings(e) => {
                write!(
                    f,
                    "paste: opening {ACCESSIBILITY_SETTINGS_URL} with `{URL_OPENER}` failed: {e}"
                )
            }
        }
    }
}

impl std::error::Error for PasteError {}

fn connect() -> Result<Enigo, PasteError> {
    let settings = Settings {
        open_prompt_to_get_permissions: false,
        ..Settings::default()
    };
    Enigo::new(&settings).map_err(|e| match e {
        NewConError::NoPermission => PasteError::NoPermission,
        other => PasteError::Connect(other),
    })
}

/// True when input simulation is allowed. Only macOS can answer false.
pub fn accessibility_granted() -> Result<bool, PasteError> {
    match connect() {
        Ok(_) => Ok(true),
        Err(PasteError::NoPermission) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn open_accessibility_settings() -> Result<(), PasteError> {
    std::process::Command::new(URL_OPENER)
        .arg(ACCESSIBILITY_SETTINGS_URL)
        .spawn()
        .map(drop)
        .map_err(PasteError::OpenSettings)
}

fn key(
    enigo: &mut Enigo,
    key: Key,
    direction: Direction,
    step: &'static str,
) -> Result<(), PasteError> {
    enigo
        .key(key, direction)
        .map_err(|source| PasteError::Input { step, source })
}

/// Releases the hotkey's modifiers, then sends Cmd+V (macOS) or Ctrl+V (Windows, Linux).
pub fn simulate_paste(hotkey_modifiers: Modifiers) -> Result<(), PasteError> {
    let mut enigo = connect()?;
    let held = [
        (Modifiers::SHIFT, Key::Shift),
        (Modifiers::CONTROL, Key::Control),
        (Modifiers::ALT, Key::Alt),
        (Modifiers::META, Key::Meta),
    ];
    for (modifier, modifier_key) in held {
        if hotkey_modifiers.contains(modifier) {
            key(
                &mut enigo,
                modifier_key,
                Direction::Release,
                "release hotkey modifier",
            )?;
        }
    }
    let paste_modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    key(
        &mut enigo,
        paste_modifier,
        Direction::Press,
        "press paste modifier",
    )?;
    key(
        &mut enigo,
        Key::Unicode(PASTE_KEY),
        Direction::Click,
        "click V",
    )?;
    key(
        &mut enigo,
        paste_modifier,
        Direction::Release,
        "release paste modifier",
    )
}
```

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning"`
Expected: `0`

- [ ] **Step 4: Run the paste probe on macOS (scratch, not committed)**

`tmp/paste-probe/Cargo.toml`:

```toml
[package]
name = "paste-probe"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
plainpaste = { path = "../.." }
```

`tmp/paste-probe/src/main.rs`:

```rust
//! Scratch probe (not committed): permission, one simulated paste
//! into whatever window has focus.

use plainpaste::config::default_hotkey;
use plainpaste::paste;

fn main() {
    let granted = paste::accessibility_granted().expect("accessibility check");
    println!("accessibility granted: {granted}");
    if granted {
        paste::simulate_paste(default_hotkey().mods).expect("simulate paste");
        println!("paste probe: ok");
    }
}
```

The probe pastes into the focused window, so focus TextEdit first. The terminal app that runs the probe needs Accessibility permission (System Settings > Privacy & Security > Accessibility).

```bash
printf 'probe text' | pbcopy
osascript -e 'tell application "TextEdit" to make new document' -e 'tell application "TextEdit" to activate'
cd tmp/paste-probe
CARGO_TARGET_DIR=../../target cargo run 2>/dev/null; echo RC=$?
cd ../..
osascript -e 'tell application "TextEdit" to get text of document 1'
```

Expected:

```
mapped control+shift+v: shift+meta+KeyV
no modifier maps to: None
default keycaps: ["Cmd", "Shift", "V"]
accessibility granted: true
paste probe: ok
RC=0
probe text
```

Then remove the terminal app from the Accessibility list, rerun the `cargo run` line, and expect `accessibility granted: false` and no `paste probe` line. Add the terminal back afterwards. Close the TextEdit document without saving.

- [ ] **Step 5: Run the suite**

Run: `python3 scripts/test.py | tail -1`
Expected: `RESULT: PASS`

- [ ] **Step 6: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add src/hotkey.rs src/paste.rs
git commit -m "feat: add hotkey service and simulated paste" -- src/hotkey.rs src/paste.rs src/lib.rs
git status --porcelain
```

### Task 8: App integration and macOS end-to-end verification

**Files:**
- Create: `src/app/mod.rs`
- Create: `src/app/flyout.rs`
- Create: `src/app/tray.rs`
- Create: `src/main.rs`

**Interfaces:**
- Consumes: Task 2 `config::{self, Settings, Theme}`, `config::default_hotkey`, `config::save`, `config::settings_path`, `config::load_or_create`; Task 1 `transform::normalize_line_breaks`; Task 3 `clipboard::{Clipboard, ClipboardError, PlatformClipboard}`; Task 6 `Flyout`, `ThemeChoice`, `Palette`; Task 7 `hotkey::{self, HotkeyService, HotkeyError}`, `paste`.
- Produces (binary-internal):
  - `app::PASTE_READ_TIMEOUT: Duration` = 2 s.
  - `app::AppEvent` enum: `Hotkey { id: u32, state: HotKeyState }`, `OpenFlyout { anchor: Option<tray_icon::Rect> }`, `FlyoutFocusLost`, `ClipboardDataRead { session: u64 }`, `PasteReadTimeout { session: u64 }`, `StartRecording`, `CancelRecording`, `RecordKey { text: String, control: bool, alt: bool, shift: bool, meta: bool }`, `ResetHotkey`, `ToggleKeepLineBreaks`, `ToggleRestoreClipboard`, `ChooseTheme(Theme)`, `OpenAccessibilitySettings`, `Quit`.
  - `app::EventSender` (`Clone`, `Send`, `Sync`) with `pub fn send(&self, event: AppEvent)`; `app::channel() -> (EventSender, async_channel::Receiver<AppEvent>)`.
  - `app::OneShotTimer` trait: `fn schedule(&self, after: Duration, on_expire: Box<dyn FnOnce()>)`; `app::SlintTimer` (production, `slint::Timer::single_shot`).
  - `app::App<C: Clipboard, T: OneShotTimer>` with `pub fn new(settings, settings_path, clipboard, timer, hotkeys, events, ui) -> Self` and `pub fn handle(&mut self, event: AppEvent)`.
  - `app::connect_sources(ui: &Flyout, events: &EventSender)`, `app::spawn_event_pump(app, events_rx, tray_events) -> Result<(), slint::EventLoopError>`.
  - `flyout::{MonitorArea, Placement, anchored_placement, centered_position, apply_theme, install_focus_hook, show, hide}`.
  - `tray::{Tray, TrayError, build(events: EventSender) -> Result<Tray, TrayError>}`.

**Behavior:**
*Startup (`main`):* init env_logger with default filter `info`; `settings_path()`; `load_or_create()` (any error stops startup: log it and exit with code 1); create `Flyout`, `PlatformClipboard::new()`, `HotkeyService::new(settings.hotkey)` (failure stops startup); `channel()`; `connect_sources`; `install_focus_hook`; `App::new`; `spawn_event_pump`; `slint::run_event_loop_until_quit()`.
*Event pump:* a `slint::spawn_local` task first builds the tray (on macOS the tray must be created after the event loop runs; a tray error is logged and quits), then loops `while let Ok(event) = rx.recv().await { app.handle(event) }`.
*Paste flow (on `Hotkey` Released of the registered hotkey):*
1. `accessibility_granted()` false → log warn "paste skipped: Accessibility permission not granted", stop.
2. `read_text()` → `None` → log info "paste skipped: clipboard has no plain text, nothing changed", stop.
3. `out = normalize_line_breaks(text, keep_line_breaks)`.
4. `restore_clipboard == false` → `write_text(out)`, `simulate_paste(hotkey.mods)`, done.
5. Otherwise: restore a still-pending snapshot first; `snapshot()`; take a new session id; `offer_lazy_text(out, on_read = send ClipboardDataRead{session})`; on error restore the snapshot and return the error; `simulate_paste`; on error restore and return; store `pending = {session, snapshot}`; `timer.schedule(PASTE_READ_TIMEOUT, send PasteReadTimeout{session})`.
6. `ClipboardDataRead{session}` (arrives through the channel, so on a later event-loop turn than the provider callback) → if `session` is pending, restore and log "clipboard restored: paste target read the clipboard". `PasteReadTimeout{session}` → if still pending, restore and log "paste target never read the clipboard". Events for other sessions are ignored.
Any clipboard or paste error aborts the paste with `log::error!("paste aborted: …")`.
*Flyout:* `OpenFlyout{anchor}` re-checks Accessibility (sets `accessibility-granted`), then `flyout::show`: show the window, read its physical size and scale factor, find the monitor that contains the icon (or the window's current monitor, or the primary monitor), and place it with `anchored_placement` (rect with non-zero size) or `centered_position` (no anchor or a zero rect from KSNI), then `focus_window()`. `anchored_placement`: caret center over the icon center (the `Tokens` caret offset read from the flyout UI, × scale from the flyout's right edge), x clamped to the monitor; below the icon when the icon's y is in the top half of the monitor (caret visible), above it otherwise (caret hidden). `FlyoutFocusLost` (from `WindowEvent::Focused(false)`) cancels a running recording and hides the window.
*Settings:* every toggle or theme change updates `Settings`, saves the file with `config::save` (error logged) and re-syncs the UI: keycaps, switches, `preview-in` = lines of `"一行\n二行\n三行"`, `preview-out` = `transform` of that text split by `\n` (keep on) or by `' '` (keep off), `theme`, and `Palette.color-scheme` (`System → Unknown`, `Light`, `Dark`).
*Recorder:* `StartRecording` → `hotkeys.suspend()`, clear error, `recording = true`. `RecordKey` → `hotkey_from_key_press`; `None` → error text "無效的組合：至少需一個修飾鍵，按鍵限字母、數字、空白或 F1–F12。" and keep recording; `Some(new)` → `apply_hotkey(new)`. `apply_hotkey`: `recording = false`; `register(new)`; success → save and sync; `Err(AlreadyRegistered)` → error text "已被其他程式使用"; other errors → the error's text; on failure re-register the previous hotkey. `CancelRecording` → `recording = false`, re-register the previous hotkey. `ResetHotkey` → `apply_hotkey(default_hotkey())`.
*Quit:* menu 結束 or the footer button → restore a pending snapshot, `slint::quit_event_loop()`.
*Tray:* 32×32 RGBA icon drawn in code (brand blue #0f6cbd clipboard outline), tooltip "PlainPaste", menu 設定 (→ `OpenFlyout{anchor: None}`) and 結束 (→ `Quit`), `with_menu_on_left_click(false)`; `TrayIconEvent::Click{button: Left, button_state: Up, rect}` → `OpenFlyout{anchor: Some(rect)}`.

**Constraints:**
- `App` is owned by the event-pump task; no `Rc<RefCell<…>>`, no `thread_local!`, no `static mut`.
- UI callbacks, the hotkey handler, tray and menu handlers, the focus hook, the provider callback and the deadline only send `AppEvent`s.
- `PASTE_READ_TIMEOUT` is armed only through `OneShotTimer::schedule` (injectable clock).
- Layer: binary; may use every library module.

**Acceptance criteria:**
- [ ] `cargo clippy --all-targets` on macOS prints no `warning` line; `python3 scripts/test.py` ends with `RESULT: PASS`.
- [ ] macOS manual checks M1–M12 in Step 6 all pass, each with the log line or observation it names.

**Steps:**

No committed test (app wiring and UI are outside the committed-test scope). Verification: clippy, the suite, and the macOS manual checks in Step 6.

- [ ] **Step 1: Write the app state machine**

`src/app/mod.rs`:

```rust
//! App state and the single event handler. Every event source (hotkey, tray, menu,
//! clipboard provider, one-shot timer, UI) sends an `AppEvent`. One local task on
//! the Slint event loop receives them in order, so handlers never nest.

use std::path::PathBuf;
use std::time::Duration;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use plainpaste::clipboard::{Clipboard, ClipboardError};
use plainpaste::config::{self, Settings, Theme};
use plainpaste::hotkey::{self, HotkeyService};
use plainpaste::paste;
use plainpaste::transform::normalize_line_breaks;
use slint::{ModelRc, SharedString, VecModel};

use crate::flyout;
use crate::{Flyout, ThemeChoice};

/// Fail-fast bound: if no app reads the lazy text within this time, restore the snapshot.
pub const PASTE_READ_TIMEOUT: Duration = Duration::from_secs(2);
const PREVIEW_SOURCE: &str = "一行\n二行\n三行";
const PREVIEW_LINE_BREAK: char = '\n';
const LINE_BREAK_REPLACEMENT: char = ' ';
const HOTKEY_TAKEN_MESSAGE: &str = "已被其他程式使用";
const HOTKEY_INVALID_MESSAGE: &str =
    "無效的組合：至少需一個修飾鍵，按鍵限字母、數字、空白或 F1–F12。";

#[derive(Debug)]
pub enum AppEvent {
    Hotkey {
        id: u32,
        state: HotKeyState,
    },
    OpenFlyout {
        anchor: Option<tray_icon::Rect>,
    },
    FlyoutFocusLost,
    ClipboardDataRead {
        session: u64,
    },
    PasteReadTimeout {
        session: u64,
    },
    StartRecording,
    CancelRecording,
    RecordKey {
        text: String,
        control: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    },
    ResetHotkey,
    ToggleKeepLineBreaks,
    ToggleRestoreClipboard,
    ChooseTheme(Theme),
    OpenAccessibilitySettings,
    Quit,
}

/// Cloneable, thread-safe sender for `AppEvent`.
#[derive(Clone)]
pub struct EventSender(async_channel::Sender<AppEvent>);

impl EventSender {
    pub fn send(&self, event: AppEvent) {
        if let Err(e) = self.0.try_send(event) {
            log::error!(
                "app event channel: expected open, actual closed. Dropped {:?}",
                e.into_inner()
            );
        }
    }
}

pub fn channel() -> (EventSender, async_channel::Receiver<AppEvent>) {
    let (tx, rx) = async_channel::unbounded();
    (EventSender(tx), rx)
}

/// Injectable clock for the paste-read deadline.
pub trait OneShotTimer {
    /// Calls `on_expire` once, `after` from now, on the event-loop thread.
    fn schedule(&self, after: Duration, on_expire: Box<dyn FnOnce()>);
}

/// Production clock: a Slint single-shot timer.
pub struct SlintTimer;

impl OneShotTimer for SlintTimer {
    fn schedule(&self, after: Duration, on_expire: Box<dyn FnOnce()>) {
        slint::Timer::single_shot(after, on_expire);
    }
}

struct PendingRestore<S> {
    session: u64,
    snapshot: S,
}

#[derive(Debug)]
enum PasteFlowError {
    Clipboard(ClipboardError),
    Paste(paste::PasteError),
}

impl std::fmt::Display for PasteFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasteFlowError::Clipboard(e) => write!(f, "{e}"),
            PasteFlowError::Paste(e) => write!(f, "{e}"),
        }
    }
}

pub struct App<C: Clipboard, T: OneShotTimer> {
    settings: Settings,
    settings_path: PathBuf,
    clipboard: C,
    timer: T,
    hotkeys: HotkeyService,
    events: EventSender,
    ui: Flyout,
    pending: Option<PendingRestore<C::Snapshot>>,
    next_session: u64,
}

impl<C: Clipboard, T: OneShotTimer> App<C, T> {
    pub fn new(
        settings: Settings,
        settings_path: PathBuf,
        clipboard: C,
        timer: T,
        hotkeys: HotkeyService,
        events: EventSender,
        ui: Flyout,
    ) -> Self {
        let app = App {
            settings,
            settings_path,
            clipboard,
            timer,
            hotkeys,
            events,
            ui,
            pending: None,
            next_session: 0,
        };
        app.ui
            .set_version(SharedString::from(env!("CARGO_PKG_VERSION")));
        app.ui.set_show_accessibility(cfg!(target_os = "macos"));
        app.sync_ui();
        app
    }

    pub fn handle(&mut self, event: AppEvent) {
        match event {
            AppEvent::Hotkey { id, state } => {
                if self.hotkeys.matches_release_event(id, state)
                    && let Err(e) = self.paste_flow()
                {
                    log::error!("paste aborted: {e}");
                }
            }
            AppEvent::OpenFlyout { anchor } => self.open_flyout(anchor),
            AppEvent::FlyoutFocusLost => {
                if self.ui.get_recording() {
                    self.cancel_recording();
                }
                flyout::hide(&self.ui);
            }
            AppEvent::ClipboardDataRead { session } => {
                self.finish_restore(session, "paste target read the clipboard")
            }
            AppEvent::PasteReadTimeout { session } => {
                self.finish_restore(session, "paste target never read the clipboard")
            }
            AppEvent::StartRecording => self.start_recording(),
            AppEvent::CancelRecording => self.cancel_recording(),
            AppEvent::RecordKey {
                text,
                control,
                alt,
                shift,
                meta,
            } => match recorder::hotkey_from_key_press(&text, control, alt, shift, meta) {
                Some(new) => self.apply_hotkey(new),
                None => self.ui.set_hotkey_error(HOTKEY_INVALID_MESSAGE.into()),
            },
            AppEvent::ResetHotkey => self.apply_hotkey(config::default_hotkey()),
            AppEvent::ToggleKeepLineBreaks => {
                self.settings.keep_line_breaks = !self.settings.keep_line_breaks;
                self.save_and_sync();
            }
            AppEvent::ToggleRestoreClipboard => {
                self.settings.restore_clipboard = !self.settings.restore_clipboard;
                self.save_and_sync();
            }
            AppEvent::ChooseTheme(theme) => {
                self.settings.theme = theme;
                self.save_and_sync();
            }
            AppEvent::OpenAccessibilitySettings => {
                if let Err(e) = paste::open_accessibility_settings() {
                    log::error!("{e}");
                }
            }
            AppEvent::Quit => self.quit(),
        }
    }

    fn paste_flow(&mut self) -> Result<(), PasteFlowError> {
        if !paste::accessibility_granted().map_err(PasteFlowError::Paste)? {
            log::warn!("paste skipped: Accessibility permission not granted");
            return Ok(());
        }
        let Some(text) = self.clipboard.read_text().map_err(PasteFlowError::Clipboard)? else {
            log::info!("paste skipped: clipboard has no plain text, nothing changed");
            return Ok(());
        };
        let out = normalize_line_breaks(&text, self.settings.keep_line_breaks);
        let modifiers = self.settings.hotkey.mods;
        if !self.settings.restore_clipboard {
            self.clipboard
                .write_text(&out)
                .map_err(PasteFlowError::Clipboard)?;
            return paste::simulate_paste(modifiers).map_err(PasteFlowError::Paste);
        }
        if let Some(previous) = self.pending.take() {
            self.restore(
                previous.snapshot,
                "a new paste started before the previous restore",
            );
        }
        let snapshot = self.clipboard.snapshot().map_err(PasteFlowError::Clipboard)?;
        let session = self.next_session;
        self.next_session += 1;
        let read_events = self.events.clone();
        if let Err(e) = self.clipboard.offer_lazy_text(
            out,
            Box::new(move || read_events.send(AppEvent::ClipboardDataRead { session })),
        ) {
            self.restore(snapshot, "offering the lazy text failed");
            return Err(PasteFlowError::Clipboard(e));
        }
        if let Err(e) = paste::simulate_paste(modifiers) {
            self.restore(snapshot, "simulating the paste failed");
            return Err(PasteFlowError::Paste(e));
        }
        self.pending = Some(PendingRestore { session, snapshot });
        let timeout_events = self.events.clone();
        self.timer.schedule(
            PASTE_READ_TIMEOUT,
            Box::new(move || timeout_events.send(AppEvent::PasteReadTimeout { session })),
        );
        Ok(())
    }

    /// Restores the pending snapshot if `session` is still pending. Stale events are ignored.
    fn finish_restore(&mut self, session: u64, reason: &str) {
        if self.pending.as_ref().is_some_and(|p| p.session == session) {
            let pending = self.pending.take().expect("checked above");
            self.restore(pending.snapshot, reason);
        }
    }

    fn restore(&mut self, snapshot: C::Snapshot, reason: &str) {
        match self.clipboard.restore(snapshot) {
            Ok(()) => log::info!("clipboard restored: {reason}"),
            Err(e) => log::error!("clipboard restore after `{reason}` failed: {e}"),
        }
    }

    fn open_flyout(&mut self, anchor: Option<tray_icon::Rect>) {
        match paste::accessibility_granted() {
            Ok(granted) => self.ui.set_accessibility_granted(granted),
            Err(e) => log::error!("accessibility check failed: {e}"),
        }
        flyout::show(&self.ui, anchor);
    }

    fn start_recording(&mut self) {
        if let Err(e) = self.hotkeys.suspend() {
            log::error!("hotkey recording: unregister failed: {e}");
            self.ui.set_hotkey_error(SharedString::from(e.to_string()));
            return;
        }
        self.ui.set_hotkey_error(SharedString::new());
        self.ui.set_recording(true);
    }

    fn cancel_recording(&mut self) {
        self.ui.set_recording(false);
        if let Err(e) = self.hotkeys.register(self.settings.hotkey) {
            log::error!("hotkey re-register of {} failed: {e}", self.settings.hotkey);
            self.ui.set_hotkey_error(SharedString::from(e.to_string()));
        }
    }

    /// Registers `new`. On failure the previous hotkey is registered again.
    fn apply_hotkey(&mut self, new: HotKey) {
        self.ui.set_recording(false);
        match self.hotkeys.register(new) {
            Ok(()) => {
                self.settings.hotkey = new;
                self.ui.set_hotkey_error(SharedString::new());
                self.save_and_sync();
            }
            Err(e) => {
                log::warn!("hotkey {new} registration failed: {e}");
                let message = match e {
                    hotkey::HotkeyError::AlreadyRegistered(_) => HOTKEY_TAKEN_MESSAGE.to_string(),
                    other => other.to_string(),
                };
                self.ui.set_hotkey_error(SharedString::from(message));
                if let Err(e) = self.hotkeys.register(self.settings.hotkey) {
                    log::error!("hotkey re-register of {} failed: {e}", self.settings.hotkey);
                }
            }
        }
    }

    fn save_and_sync(&mut self) {
        if let Err(e) = config::save(&self.settings_path, &self.settings) {
            log::error!("{e}");
        }
        self.sync_ui();
    }

    fn sync_ui(&self) {
        let ui = &self.ui;
        let caps: Vec<SharedString> = recorder::keycap_labels(self.settings.hotkey)
            .into_iter()
            .map(SharedString::from)
            .collect();
        ui.set_hotkey_caps(ModelRc::new(VecModel::from(caps)));
        ui.set_keep_line_breaks(self.settings.keep_line_breaks);
        ui.set_restore_clipboard(self.settings.restore_clipboard);
        let preview_out = normalize_line_breaks(PREVIEW_SOURCE, self.settings.keep_line_breaks);
        let separator = if self.settings.keep_line_breaks {
            PREVIEW_LINE_BREAK
        } else {
            LINE_BREAK_REPLACEMENT
        };
        ui.set_preview_in(string_model(PREVIEW_SOURCE.split(PREVIEW_LINE_BREAK)));
        ui.set_preview_out(string_model(preview_out.split(separator)));
        ui.set_theme(match self.settings.theme {
            Theme::System => ThemeChoice::System,
            Theme::Light => ThemeChoice::Light,
            Theme::Dark => ThemeChoice::Dark,
        });
        flyout::apply_theme(ui, self.settings.theme);
    }

    fn quit(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.restore(pending.snapshot, "the app is quitting");
        }
        if let Err(e) = slint::quit_event_loop() {
            log::error!("quit: event loop already gone: {e}");
        }
    }
}

fn string_model<'a>(parts: impl Iterator<Item = &'a str>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        parts.map(SharedString::from).collect::<Vec<_>>(),
    ))
}

/// Wires every UI callback and the hotkey handler to `events`.
pub fn connect_sources(ui: &Flyout, events: &EventSender) {
    let tx = events.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        tx.send(AppEvent::Hotkey {
            id: e.id,
            state: e.state,
        })
    }));
    let tx = events.clone();
    ui.on_start_recording(move || tx.send(AppEvent::StartRecording));
    let tx = events.clone();
    ui.on_cancel_recording(move || tx.send(AppEvent::CancelRecording));
    let tx = events.clone();
    ui.on_record_key(move |text, control, alt, shift, meta| {
        tx.send(AppEvent::RecordKey {
            text: text.to_string(),
            control,
            alt,
            shift,
            meta,
        })
    });
    let tx = events.clone();
    ui.on_reset_hotkey(move || tx.send(AppEvent::ResetHotkey));
    let tx = events.clone();
    ui.on_toggle_keep_line_breaks(move || tx.send(AppEvent::ToggleKeepLineBreaks));
    let tx = events.clone();
    ui.on_toggle_restore_clipboard(move || tx.send(AppEvent::ToggleRestoreClipboard));
    let tx = events.clone();
    ui.on_choose_theme(move |choice| {
        tx.send(AppEvent::ChooseTheme(match choice {
            ThemeChoice::System => Theme::System,
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::Dark => Theme::Dark,
        }))
    });
    let tx = events.clone();
    ui.on_open_accessibility_settings(move || tx.send(AppEvent::OpenAccessibilitySettings));
    let tx = events.clone();
    ui.on_quit(move || tx.send(AppEvent::Quit));
}

/// Runs the app: one local task receives every event in order.
pub fn spawn_event_pump<C: Clipboard + 'static, T: OneShotTimer + 'static>(
    app: App<C, T>,
    events: async_channel::Receiver<AppEvent>,
    tray_events: EventSender,
) -> Result<(), slint::EventLoopError> {
    let mut app = app;
    slint::spawn_local(async move {
        // macOS: the tray must be created after the event loop has started. This task runs then.
        let _tray = match crate::tray::build(tray_events) {
            Ok(tray) => tray,
            Err(e) => {
                log::error!("{e}");
                if let Err(e) = slint::quit_event_loop() {
                    log::error!("quit: {e}");
                }
                return;
            }
        };
        while let Ok(event) = events.recv().await {
            app.handle(event);
        }
    })
    .map(drop)
}
```

- [ ] **Step 2: Write the flyout window helpers**

`src/app/flyout.rs`:

```rust
//! Flyout window placement, focus-loss closing and theme switching.

use plainpaste::config::Theme;
use slint::language::ColorScheme;
use slint::winit_030::winit::dpi::{PhysicalPosition as WinitPosition, PhysicalSize as WinitSize};
use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize};

use crate::app::{AppEvent, EventSender};
use crate::{Flyout, Palette};

const HALF: f64 = 2.0;

/// A monitor's area in physical pixels.
#[derive(Debug, Clone, Copy)]
pub struct MonitorArea {
    pub position: WinitPosition<i32>,
    pub size: WinitSize<u32>,
}

/// Where the flyout goes relative to the tray icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub position: PhysicalPosition,
    /// True when the flyout hangs below the icon, so the caret points up at it.
    pub caret_visible: bool,
}

/// Places the flyout next to the tray icon: below it when the icon is in the top half
/// of its monitor (macOS menu bar), above it otherwise (Windows taskbar).
/// The caret sits over the icon center. The result is clamped to the monitor horizontally.
pub fn anchored_placement(
    icon: tray_icon::Rect,
    flyout: PhysicalSize,
    scale: f32,
    caret_from_right: f32,
    monitor: MonitorArea,
) -> Placement {
    let icon_center_x = icon.position.x + f64::from(icon.size.width) / HALF;
    let caret_from_right = f64::from(caret_from_right * scale);
    let monitor_left = f64::from(monitor.position.x);
    let monitor_right = monitor_left + f64::from(monitor.size.width) - f64::from(flyout.width);
    let x = (icon_center_x - f64::from(flyout.width) + caret_from_right)
        .clamp(monitor_left, monitor_right.max(monitor_left));
    let monitor_middle_y = f64::from(monitor.position.y) + f64::from(monitor.size.height) / HALF;
    let below = icon.position.y < monitor_middle_y;
    let y = if below {
        icon.position.y + f64::from(icon.size.height)
    } else {
        icon.position.y - f64::from(flyout.height)
    };
    Placement {
        position: PhysicalPosition::new(x.round() as i32, y.round() as i32),
        caret_visible: below,
    }
}

/// Centers the flyout on the monitor (Linux: AppIndicator/KSNI give no icon position).
pub fn centered_position(flyout: PhysicalSize, monitor: MonitorArea) -> PhysicalPosition {
    let x = i64::from(monitor.position.x)
        + (i64::from(monitor.size.width) - i64::from(flyout.width)) / 2;
    let y = i64::from(monitor.position.y)
        + (i64::from(monitor.size.height) - i64::from(flyout.height)) / 2;
    PhysicalPosition::new(x as i32, y as i32)
}

/// Switches the Fluent palette. `System` uses `ColorScheme::Unknown`, which follows the OS.
pub fn apply_theme(ui: &Flyout, theme: Theme) {
    let scheme = match theme {
        Theme::System => ColorScheme::Unknown,
        Theme::Light => ColorScheme::Light,
        Theme::Dark => ColorScheme::Dark,
    };
    ui.global::<Palette>().set_color_scheme(scheme);
}

/// Installs the focus-loss hook. The hook only sends an event, so it never re-enters app state.
pub fn install_focus_hook(ui: &Flyout, events: EventSender) {
    ui.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::Focused(false) = event {
            events.send(AppEvent::FlyoutFocusLost);
        }
        EventResult::Propagate
    });
}

/// Shows the flyout anchored to the tray icon, or centered when there is no usable anchor, then focuses it.
pub fn show(ui: &Flyout, anchor: Option<tray_icon::Rect>) {
    if let Err(e) = ui.show() {
        log::error!("flyout show failed: {e}");
        return;
    }
    let window = ui.window();
    let size = window.size();
    let scale = window.scale_factor();
    // KSNI reports an all-zero rect: treat it as "no anchor".
    let anchor = anchor.filter(|r| r.size.width > 0 && r.size.height > 0);
    let monitor = window.with_winit_window(|w| {
        let wanted = anchor.map(|r| (r.position.x, r.position.y));
        let containing = wanted.and_then(|(x, y)| {
            w.available_monitors().find(|m| {
                let (p, s) = (m.position(), m.size());
                x >= f64::from(p.x)
                    && x < f64::from(p.x) + f64::from(s.width)
                    && y >= f64::from(p.y)
                    && y < f64::from(p.y) + f64::from(s.height)
            })
        });
        containing
            .or_else(|| w.current_monitor())
            .or_else(|| w.primary_monitor())
            .map(|m| MonitorArea {
                position: m.position(),
                size: m.size(),
            })
    });
    let Some(Some(monitor)) = monitor else {
        log::error!(
            "flyout placement: expected a monitor from winit, actual none. The flyout stays at its last position."
        );
        focus(ui);
        return;
    };
    match anchor {
        Some(icon) => {
            let placement = anchored_placement(icon, size, scale, monitor);
            ui.set_caret_visible(placement.caret_visible);
            window.set_position(placement.position);
        }
        None => {
            ui.set_caret_visible(false);
            window.set_position(centered_position(size, monitor));
        }
    }
    focus(ui);
}

fn focus(ui: &Flyout) {
    if ui
        .window()
        .with_winit_window(|w| w.focus_window())
        .is_none()
    {
        log::error!("flyout focus: expected a winit window, actual none");
    }
}

pub fn hide(ui: &Flyout) {
    if let Err(e) = ui.hide() {
        log::error!("flyout hide failed: {e}");
    }
}
```

- [ ] **Step 3: Write the tray**

`src/app/tray.rs`:

```rust
//! Tray icon and menu. Events are forwarded to the app event channel.

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app::{AppEvent, EventSender};

const TOOLTIP: &str = "PlainPaste";
const MENU_SETTINGS: &str = "設定";
const MENU_QUIT: &str = "結束";
const ICON_SIZE: u32 = 32;
const BYTES_PER_PIXEL: usize = 4;
/// Brand blue #0f6cbd, opaque.
const ICON_RGBA: [u8; BYTES_PER_PIXEL] = [0x0f, 0x6c, 0xbd, 0xff];
/// Clipboard glyph: board outline and clip, in icon pixels.
const BOARD_LEFT: u32 = 6;
const BOARD_RIGHT: u32 = 26;
const BOARD_TOP: u32 = 6;
const BOARD_BOTTOM: u32 = 30;
const BOARD_STROKE: u32 = 2;
const CLIP_LEFT: u32 = 11;
const CLIP_RIGHT: u32 = 21;
const CLIP_TOP: u32 = 3;
const CLIP_BOTTOM: u32 = 9;

/// Keeps the tray icon alive. Dropping it removes the icon.
pub struct Tray {
    _icon: TrayIcon,
}

#[derive(Debug)]
pub enum TrayError {
    Icon(tray_icon::BadIcon),
    Menu(tray_icon::menu::Error),
    Build(tray_icon::Error),
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrayError::Icon(e) => {
                write!(f, "tray: icon {ICON_SIZE}x{ICON_SIZE} RGBA rejected: {e}")
            }
            TrayError::Menu(e) => write!(f, "tray: menu creation failed: {e}"),
            TrayError::Build(e) => write!(f, "tray: icon creation failed: {e}"),
        }
    }
}

impl std::error::Error for TrayError {}

fn glyph_pixel(x: u32, y: u32) -> bool {
    let in_clip = (CLIP_LEFT..CLIP_RIGHT).contains(&x) && (CLIP_TOP..CLIP_BOTTOM).contains(&y);
    let in_board = (BOARD_LEFT..BOARD_RIGHT).contains(&x) && (BOARD_TOP..BOARD_BOTTOM).contains(&y);
    let in_board_inner = (BOARD_LEFT + BOARD_STROKE..BOARD_RIGHT - BOARD_STROKE).contains(&x)
        && (BOARD_TOP + BOARD_STROKE..BOARD_BOTTOM - BOARD_STROKE).contains(&y);
    in_clip || (in_board && !in_board_inner)
}

fn icon() -> Result<Icon, TrayError> {
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE) as usize * BYTES_PER_PIXEL);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let pixel = if glyph_pixel(x, y) {
                ICON_RGBA
            } else {
                [0; BYTES_PER_PIXEL]
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).map_err(TrayError::Icon)
}

/// Builds the tray. Call it on the main thread after the event loop has started (macOS requirement).
/// A left click sends `OpenFlyout` with the icon rect. The menu has 設定 and 結束.
pub fn build(events: EventSender) -> Result<Tray, TrayError> {
    let settings = MenuItem::new(MENU_SETTINGS, true, None);
    let quit = MenuItem::new(MENU_QUIT, true, None);
    let menu = Menu::with_items(&[&settings, &quit]).map_err(TrayError::Menu)?;
    let settings_id: MenuId = settings.id().clone();
    let quit_id: MenuId = quit.id().clone();

    let menu_events = events.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == settings_id {
            menu_events.send(AppEvent::OpenFlyout { anchor: None });
        } else if event.id == quit_id {
            menu_events.send(AppEvent::Quit);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            rect,
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            events.send(AppEvent::OpenFlyout { anchor: Some(rect) });
        }
    }));

    let icon = TrayIconBuilder::new()
        .with_icon(icon()?)
        .with_tooltip(TOOLTIP)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .map_err(TrayError::Build)?;
    Ok(Tray { _icon: icon })
}
```

- [ ] **Step 4: Write `main`**

`src/main.rs`:

```rust
mod app;
mod flyout;
mod tray;

use std::process::ExitCode;

use plainpaste::{clipboard, config, hotkey};

slint::include_modules!();

const DEFAULT_LOG_FILTER: &str = "info";

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = config::settings_path()?;
    let settings = config::load_or_create(&settings_path)?;
    log::info!("settings loaded from {}", settings_path.display());

    let ui = Flyout::new()?;
    let clipboard = clipboard::PlatformClipboard::new()?;
    let hotkeys = hotkey::HotkeyService::new(settings.hotkey)?;
    let (events, receiver) = app::channel();
    app::connect_sources(&ui, &events);
    flyout::install_focus_hook(&ui, events.clone());

    let app = app::App::new(
        settings,
        settings_path,
        clipboard,
        app::SlintTimer,
        hotkeys,
        events.clone(),
        ui,
    );
    app::spawn_event_pump(app, receiver, events)?;
    slint::run_event_loop_until_quit()?;
    Ok(())
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(DEFAULT_LOG_FILTER))
        .init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("PlainPaste stopped: {e}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 5: Lint and run the suite**

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning"; python3 scripts/test.py | tail -1`
Expected: `0` and `RESULT: PASS`.

- [ ] **Step 6: macOS manual checks (USER-RUN: a person at the Mac performs them)**

The app runs in the foreground until it quits, and the checks need clicks on the menu bar, so a person performs them. Start it from Terminal (the terminal app must be in System Settings > Privacy & Security > Accessibility):

```bash
cargo build && ./target/debug/plainpaste 2> tmp/app-macos.log
```

Prepare rich text once: open TextEdit, type three lines `一行`, `二行`, `三行`, make them bold, select all, Cmd+C.

| Id | Action | Expected |
|---|---|---|
| M1 | First start with no settings file (`rm -f ~/Library/Application\ Support/PlainPaste/settings.json` before starting). | Log line `created config file ~/Library/Application Support/PlainPaste/settings.json with defaults`; a blue clipboard icon appears in the menu bar. |
| M2 | Click the tray icon. Open `design/mockups/variant-b.html#light` in a browser next to it. Click the desktop. | The flyout opens below the icon with the caret pointing at it, keycaps `Cmd + Shift + V`, and matches the mockup's "互動版" card by card. Clicking elsewhere closes it. A right click on the icon shows the menu 設定 / 結束. |
| M3 | Flyout: set 保留換行 off and 貼上後還原剪貼簿 off. In a new TextEdit document press Cmd+Shift+V, then Cmd+V. | The hotkey pastes `一行 二行 三行` as plain text (not bold); the preview shows the tinted space markers. Cmd+V pastes the same plain text (the clipboard was replaced). |
| M4 | Copy the bold text again. Set 保留換行 on. Press Cmd+Shift+V. | Three plain lines are pasted; the preview shows three lines. |
| M5 | Copy the bold text again. Set 貼上後還原剪貼簿 on. Press Cmd+Shift+V, then Cmd+V. | The hotkey pastes plain text; the log shows `clipboard restored: paste target read the clipboard`; Cmd+V pastes the bold original; `osascript -e 'clipboard info'` lists `«class RTF »`. |
| M6 | Restore on. Click the empty desktop (Finder frontmost, no text field). Press Cmd+Shift+V. | About 2 s later the log shows `clipboard restored: paste target never read the clipboard`; `osascript -e 'clipboard info'` lists `«class RTF »`. If the log instead shows `paste target read the clipboard`, repeat with Activity Monitor frontmost. |
| M7 | Press Cmd+Shift+V twice quickly in TextEdit. | Two pastes, no hang; the log shows one restore per paste (the first may say `a new paste started before the previous restore`). |
| M8 | Flyout: click the keys box, press Cmd+Option+P. Then click it again and press Esc. Then click 重設. | Recording state shows `請按下按鍵組合…`, 取消, and the Esc hint; after Cmd+Option+P the keycaps show `Cmd + Alt + P`, `settings.json` contains `"hotkey": "alt+meta+KeyP"`, Cmd+Option+P pastes and Cmd+Shift+V no longer does. Esc restores the previous keycaps. 重設 returns to `Cmd + Shift + V`. Pressing only `P` while recording shows `無效的組合：…`. (macOS lets two processes register the same combo, so `已被其他程式使用` is checked on Windows in Task 9.) |
| M9 | Theme: click 深色, 淺色, 跟隨系統; with 跟隨系統 selected, switch macOS Appearance between Light and Dark. | The flyout switches immediately to the mockup's dark / light tokens; with 跟隨系統 it follows the system switch live. `settings.json` `theme` changes accordingly. |
| M10 | Remove the terminal app from the Accessibility list. Open the flyout. Press Cmd+Shift+V in TextEdit. Click 開啟系統設定. Add the terminal back, reopen the flyout. | The card shows the warn tile, `需授權才能模擬貼上`, badge `未授權` and a primary button. The hotkey logs `paste skipped: Accessibility permission not granted` and the clipboard is unchanged. The button opens System Settings at Accessibility. After re-adding, the card shows `已授權`. |
| M11 | Click 結束 in the flyout. Start again, then choose 結束 in the tray menu. | The process exits with code 0 both times (`echo $?` prints `0`). |
| M12 | Quit. Edit `settings.json` so `"theme": "blue"`. Record `shasum` of the file. Start the app. Record `shasum` again. | The app exits with code 1 and logs `config file ~/Library/Application Support/PlainPaste/settings.json: field `theme`: expected one of "system", "light", "dark", actual "blue"`. Both shasums are equal. Restore the file afterwards (`rm` it; the next start recreates defaults). |

Record each check as PASS or FAIL with the observed log line in `tmp/macos-checks.md` (scratch).

- [ ] **Step 7: Commit**

```bash
git rev-parse --abbrev-ref HEAD
git add src/app/mod.rs src/app/flyout.rs src/app/tray.rs src/main.rs
git commit -m "feat: wire tray, hotkey, flyout and paste flow" -- src/app/mod.rs src/app/flyout.rs src/app/tray.rs src/main.rs
git status --porcelain
```

### Task 9: Windows and Linux end-to-end verification

**Files:**
- No committed files. Scratch: `tmp/linux/Dockerfile`, `tmp/linux/e2e.sh`.

**Interfaces:**
- Consumes: the complete application from Tasks 1–8 on `main`.
- Produces: verification evidence only (command output saved under `tmp/` and summarized in the sign-off report).

**Behavior:**
Windows: build and run the suite over ssh, then a person at the Windows desktop runs the manual checks W1–W8 (an ssh session cannot drive the interactive desktop).
Linux: in the Docker image from Task 5, start Xvfb, start the app, drive it with `xdotool` against a GTK entry (`zenity --entry`), and check paste, restore and the timeout path with `xclip`. The tray needs a StatusNotifierWatcher, which the container does not have; the tray check (L5) is done on a real X11 desktop with KDE or GNOME + AppIndicator extension by a person.

**Constraints:**
- No code change in this task. A failing check is reported as `BLOCKED` with the output, and fixed in a new task.

**Acceptance criteria:**
- [ ] Windows: `cargo clippy --all-targets` has no `warning` line and `python3 scripts/test.py` ends with `RESULT: PASS` on the host.
- [ ] Windows manual checks W1–W8 pass.
- [ ] Linux container: `tmp/linux/e2e.sh` prints `E2E PLAIN PASTE: PASS`, `E2E RESTORE: PASS`, `E2E TIMEOUT RESTORE: PASS` and exits 0.
- [ ] Linux desktop check L5 passes or is reported as not run with the reason.

**Steps:**

- [ ] **Step 1: Windows — copy the tree, lint and run the suite**

Abbreviation: `WIN="ssh -i <ssh-key> <windows-user>@<windows-host>"`.

```bash
tar -cf tmp/win-src.tar Cargo.toml Cargo.lock build.rs src tests scripts ui
$WIN "powershell -Command \"New-Item -ItemType Directory -Force -Path plainpaste | Out-Null\""
scp -i <ssh-key> tmp/win-src.tar <windows-user>@<windows-host>:plainpaste/win-src.tar
$WIN "powershell -Command \"cd plainpaste; tar -xf win-src.tar; Remove-Item win-src.tar\""
rm tmp/win-src.tar
$WIN "powershell -Command \"cd plainpaste; cargo clippy --all-targets 2>&1 | Select-String -Pattern '^(warning|error)' | Measure-Object | Select-Object -ExpandProperty Count\""
$WIN "powershell -Command \"cd plainpaste; python scripts/test.py\""
```

Expected: `0`, then a last line `RESULT: PASS`.

- [ ] **Step 2: Windows manual checks (USER-RUN: a person at the Windows desktop)**

In a PowerShell window on the Windows desktop: `cd ~\plainpaste; $env:RUST_LOG='info'; cargo run 2> app-windows.log`. Rich source text: open `design/mockups/variant-b.html` (copy it over or open it from a share) in Edge, select the three lines in the "複製的文字" box, Ctrl+C. Paste target: Edge tab with the address `data:text/html,<div contenteditable style="min-height:200px;border:1px solid">`.

| Id | Action | Expected |
|---|---|---|
| W1 | First start with no settings file. | Log `created config file …\PlainPaste\config\settings.json with defaults` (the exact path is the one in `settings loaded from …`); the tray icon appears (it may be in the overflow area). |
| W2 | Left-click the tray icon; click elsewhere; right-click the icon. | The flyout opens above the icon (taskbar at the bottom, so no caret), keycaps `Ctrl + Shift + V`, no Accessibility card. Clicking elsewhere closes it. Right click shows 設定 / 結束. |
| W3 | 保留換行 off, 貼上後還原剪貼簿 off. In the contenteditable tab press Ctrl+Shift+V, then Ctrl+V. | Both paste `一行 二行 三行` as plain text. |
| W4 | Copy the source again. 保留換行 on. Ctrl+Shift+V. | Three plain lines. |
| W5 | Copy the source again. 貼上後還原剪貼簿 on. Ctrl+Shift+V, then Ctrl+V. | The hotkey pastes plain text; log `clipboard restored: paste target read the clipboard`; Ctrl+V pastes the formatted original. |
| W6 | Restore on. Click the empty desktop. Ctrl+Shift+V. | About 2 s later the log shows `clipboard restored: paste target never read the clipboard`; `Get-Clipboard -TextFormatType Html` prints HTML. |
| W7 | Flyout: record Ctrl+Alt+K. Then, in a second PowerShell window, run the helper below (prints `True`). Then click 重設 in the flyout. Then press Enter in the helper window. Click 重設 again. | After recording, keycaps `Ctrl + Alt + K` and Ctrl+Alt+K pastes. With the helper holding Ctrl+Shift+V, 重設 shows `已被其他程式使用`, the keycaps stay `Ctrl + Alt + K`, and Ctrl+Alt+K still pastes. After the helper releases, 重設 succeeds (`Ctrl + Shift + V`). |
| W8 | Click 結束 in the flyout; start again; choose 結束 in the tray menu. | The process exits with code 0 both times (`$LASTEXITCODE` is `0`). |

W7 helper (scratch, paste into a second PowerShell window):

```powershell
Add-Type -Namespace W -Name H -MemberDefinition '[DllImport("user32.dll")] public static extern bool RegisterHotKey(System.IntPtr h, int id, uint mods, uint vk);'
[W.H]::RegisterHotKey([System.IntPtr]::Zero, 1, 0x0006, 0x56)
Read-Host 'Holding Ctrl+Shift+V. Press Enter to release'
```

(`0x0006` = MOD_CONTROL | MOD_SHIFT, `0x56` = V. A hotkey owned by another process is consumed by Windows before the flyout sees the keys, so the recorder itself cannot capture a taken combo; 重設 is the path that reaches the "taken" error.)

Record each check as PASS or FAIL with the observed log line in `tmp/windows-checks.md` (scratch).

- [ ] **Step 3: Linux — build the app in the container**

Abbreviation: `DRUN="docker run --rm -v $PWD:/src -v $HOME/.cargo/registry:/usr/local/cargo/registry -e CARGO_TARGET_DIR=/src/tmp/target-linux"`. The image `plainpaste-linux` comes from Task 5 Step 2 (`docker build -q -t plainpaste-linux tmp/linux` if it is missing).

Run: `$DRUN -w /src plainpaste-linux cargo build 2>&1 | tail -1`
Expected: `Finished …`

- [ ] **Step 4: Linux — automated end-to-end run under Xvfb**

`tmp/linux/e2e.sh`:

```bash
#!/usr/bin/env bash
# Linux end-to-end check. Runs inside the plainpaste-linux container under xvfb-run. Scratch only.
# Every wait is bounded by `timeout` and waits for a log line or a window, never for a fixed time.
set -u
export LANG=C.UTF-8
APP=/src/tmp/target-linux/debug/plainpaste
LOG=/src/tmp/linux/app.log
CONFIG_DIR="$HOME/.config/plainpaste"
SOURCE_TEXT=$'一行\n二行\n三行'
EXPECTED_PASTE='一行 二行 三行'
STEP_BOUND=15
FAILED=0

wait_log() {
  timeout "$STEP_BOUND" grep -m1 -q "$1" <(tail -n +1 -f "$LOG")
}

result() {
  if [ "$2" = "$3" ]; then echo "$1: PASS"; else echo "$1: FAIL (expected [$2], actual [$3])"; FAILED=1; fi
}

eval "$(dbus-launch --sh-syntax)"
mkdir -p "$CONFIG_DIR"
printf '{"hotkey":"shift+control+KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"system"}' > "$CONFIG_DIR/settings.json"
: > "$LOG"
RUST_LOG=info "$APP" 2> "$LOG" &
APP_PID=$!
wait_log "settings loaded" || { echo "app did not start"; cat "$LOG"; exit 1; }

# Paste into a GTK entry, then check the entry text and the restored clipboard.
printf '%s' "$SOURCE_TEXT" | xclip -selection clipboard -t UTF8_STRING -silent
zenity --entry --title=e2e-target > /src/tmp/linux/entry.txt 2>/dev/null &
ZENITY_PID=$!
timeout "$STEP_BOUND" xdotool search --sync --name e2e-target windowfocus --sync
xdotool key --clearmodifiers ctrl+shift+v
wait_log "paste target read the clipboard" || echo "no read log line"
xdotool search --name e2e-target windowfocus --sync key Return
timeout "$STEP_BOUND" tail --pid="$ZENITY_PID" -f /dev/null
result "E2E PLAIN PASTE" "$EXPECTED_PASTE" "$(cat /src/tmp/linux/entry.txt)"
result "E2E RESTORE" "$SOURCE_TEXT" "$(timeout 5 xclip -selection clipboard -o)"

# Timeout path: no window has focus, so nothing reads the lazy text.
xdotool key --clearmodifiers ctrl+shift+v
wait_log "paste target never read the clipboard" || echo "no timeout log line"
result "E2E TIMEOUT RESTORE" "$SOURCE_TEXT" "$(timeout 5 xclip -selection clipboard -o)"

kill "$APP_PID"
echo "--- app log ---"
cat "$LOG"
exit "$FAILED"
```

Run: `$DRUN -w /src plainpaste-linux timeout 120 xvfb-run -a bash /src/tmp/linux/e2e.sh > tmp/linux/e2e.log 2>/dev/null; echo RC=$?; head -3 tmp/linux/e2e.log`
Expected:

```
RC=0
E2E PLAIN PASTE: PASS
E2E RESTORE: PASS
E2E TIMEOUT RESTORE: PASS
```

The app log at the end of `tmp/linux/e2e.log` contains `settings loaded from /root/.config/plainpaste/settings.json`, `clipboard restored: paste target read the clipboard` and `clipboard restored: paste target never read the clipboard`. (Observed exactly so during planning. `enigo` also logs `modifier_no: 5 is unmapped` warnings under Xvfb; they are harmless.)

- [ ] **Step 5: Linux desktop tray check L5 (USER-RUN on a real X11 desktop)**

On an X11 session with KDE Plasma, or GNOME with the AppIndicator extension: build with `cargo build` and run `RUST_LOG=info ./target/debug/plainpaste`.
Expected: the tray icon appears; a left click and the menu item 設定 both open the flyout centered on the screen without a caret; clicking elsewhere closes it; 結束 quits with code 0. If no such desktop is available, report L5 as "not run" with that reason.

- [ ] **Step 6: Report**

No commit. Report the Windows checks W1–W8, the Linux e2e output and L5 to the lead.

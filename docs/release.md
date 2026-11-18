# Release runbook

A `v<version>` tag push (version = `Cargo.toml` version) triggers `.github/workflows/release.yml`:
three native build jobs (macOS arm64, Windows x86_64, Linux x86_64) each run `scripts/ci.py`,
then one release job publishes the three assets. CI runs no tests. Toolchain: Rust 1.100.0
(`rust-toolchain.toml`; Linux builds in docker `rust:1.100.0-bookworm`).

Assets: `plainpaste-<ver>-macos-arm64.zip`, `plainpaste-<ver>-windows-x86_64.exe`,
`plainpaste-<ver>-linux-x86_64`.

## Cut a release

1. Set the version in `Cargo.toml`. Write `release-notes/v<ver>.md` in the halcyon format (English block, `---`, zh-TW block; sections Highlights, Added, Fixed & Improved, Internal / Build). `ci.py publish` passes it as the release body and fails if it is missing or off-format. Commit on `main`.
2. Run the three local legs below. All must exit 0.
3. Push `main`, then tag and push the tag:
   ```
   git tag v<ver>
   git push origin v<ver>
   ```
4. Watch the run: `gh run list --workflow release.yml --limit 1`, then `gh run view <id> --exit-status`.
5. Verify: `gh release view v<ver> --json assets --jq '.assets[].name'` lists the three assets.

## Local three-platform run (from the Mac)

Run each leg as its own foreground command:

```
python3 scripts/ci.py build --platform macos --tag v<ver>
python3 scripts/ci.py build --platform linux --tag v<ver>
python3 scripts/ci.py build --platform windows --tag v<ver>
```

Linux builds in OrbStack (native aarch64 locally, asset `-linux-aarch64`). Windows runs over ssh on the Windows host.
The Windows leg from a non-Windows machine needs two env vars (`scripts/ci.py` fails naming the one that is unset):
`PLAINPASTE_WIN_HOST` (ssh target `user@host`) and `PLAINPASTE_WIN_KEY` (path to the ssh private key).
If you keep them in a local `.env` file, it is gitignored, and you must load it into the shell yourself (`scripts/ci.py` reads only real environment variables).
Check the guard alone with `python3 scripts/ci.py guard`.

## A tag run failed

- Transient runner or network failure, no code change: `gh run rerun <id> --failed`.
- Code or workflow fix needed: commit the fix and push `main`, then move the tag. Deleting the remote
  tag is destructive, so get the owner's approval first and list the target before each delete:
  ```
  git push origin :refs/tags/v<ver>
  git tag -f v<ver> HEAD
  git push origin v<ver>
  ```
  The version stays the same.
- The release job failed after it created the release: after approval, `gh release delete v<ver> --yes`
  (this keeps the tag), then `gh run rerun <id> --failed`.

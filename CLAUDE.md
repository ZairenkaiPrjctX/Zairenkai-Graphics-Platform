# CLAUDE.md: Zairenkai-Graphics-Platform

Instructions for Claude Code in `ZairenkaiPrjctX/Zairenkai-Graphics-Platform`. Keep this file under 200 lines;
put detail in `AGENT/`.

@AGENTS.md
@AGENT/PROJECT_CONTEXT.md

## Current state (read first)

This repository holds **`zgfx`**, read-only Vulkan and OpenGL ES/EGL capability probes (Rust,
MSRV 1.85): `crates/zgfx-core` (contract types and decoders, no I/O) and `crates/zgfx` (the
probes and CLI). Decision: `docs/adr/0001-probe-stack.md`. Contract: `docs/CONTRACT.md`.
Host-tested on Mesa lavapipe/llvmpipe; nothing has run on an Android device
(`docs/TESTING.md`).

Non-negotiables in this repo:
- **Read only.** No logical device, no drawing, no file, property or setting written. Never
  write `debug.*` or `vendor.*` properties, force GPU frequencies, or install, replace or select a
  driver (Mesa, Turnip, ANGLE). A control needs its own ADR, hardware validation and Core.
- **Vulkan and GLES stay independent**: separate reports, statuses and tests.
- **No guessed values.** A failed call is a status plus the API's error name; Android feature
  flags are not proof of runtime support.
- Libraries are opened at run time (`ash` `Entry::load_from`, `libloading`); never link
  libvulkan, libEGL or libGLESv2 at build time.
- The GLES context is created, used and released on one probe thread of the zgfx process.
- Tests never need a GPU unless marked `#[ignore]` with the driver they need.

## Commands

| Purpose | Command | Status |
|---|---|---|
| Build | `cargo build --workspace` | Verified. Evidence: 2026-10-23 |
| Test | `cargo test --workspace` | Verified. Evidence: 22 passed, 3 ignored (driver tests), 2026-10-23 |
| Test with host drivers | `cargo test --workspace -- --include-ignored` (Mesa: `docs/TESTING.md`) | Verified. Evidence: 25 passed on Mesa 25.2.8 lavapipe/llvmpipe, Ubuntu 24.04 x86_64, 2026-10-23 |
| Lint / format | `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` | Verified. Evidence: clean, 2026-10-23 |
| MSRV | `cargo +1.85.0 test --workspace` | Verified. Evidence: passed, 2026-10-23 |
| Shell / workflow lint | `shellcheck scripts/*.sh`; `actionlint .github/workflows/ci.yml` | Verified. Evidence: clean, 2026-10-23 |
| Android binaries | `ANDROID_NDK_HOME=<ndk> scripts/build-android.sh` | Verified. Evidence: NDK r27d, arm64-v8a/x86_64/armeabi-v7a, 16 KB LOAD alignment checked, NEEDED only libc and libdl, 2026-10-23 |
| Validate agent foundation | `python3 <path-to-.github>/agent-foundation/scripts/validate_agent_setup.py .` | Configured |

Only list commands that exist in this repository. Never invent one.

## Working rules

- Inspect the relevant code before editing it. Preserve unrelated and uncommitted changes.
- Generated files are edited at their source, then regenerated. Managed foundation files
  (listed in `AGENT/.foundation-manifest.json`) are edited in `ZairenkaiPrjctX/.github`, not here.
- Destructive or outward-facing actions (deleting data, force-push, history rewrites,
  publishing releases, deploying, flashing devices) need explicit human approval each time.
- Do not claim a build, test, or signature succeeded without showing the output.

## Git

- Work on a topic branch; open **draft** pull requests; never merge or approve.
- Commits must be cryptographically signed. If signing fails, stop and report the error.
  Trailer format and verification: `AGENT/policies/git-and-commit-signing.md`.

## Where things are

- `AGENT/README.md`: index of the agent library (prompts, policies, playbooks, templates).
- `.claude/skills/`: run `/repo-audit`, `/implement-feature`, `/bug-fix`, `/test-and-verify`,
  `/security-review`, `/release-readiness`.
- `.claude/rules/`: path-scoped rules, loaded automatically.

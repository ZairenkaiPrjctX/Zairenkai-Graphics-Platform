# Project context: Zairenkai-Graphics-Platform

> Owned by this repository. Update it whenever verified facts change. Label every statement
> **Verified** (with `Evidence:`), **Unverified**, or **Unknown / Not Configured**.

## Identity

| Field | Value | Status |
|---|---|---|
| Repository | `ZairenkaiPrjctX/Zairenkai-Graphics-Platform` | Verified. Evidence: GitHub API |
| GitHub description | GPU Tweaks Zairenkai | Verified. Evidence: GitHub API. Note: the code is read-only graphics diagnostics, not tweaks |
| Visibility | public | Verified. Evidence: `list_repos`, 2026-10-22 (earlier entries said private) |
| Default branch | `dev` | Verified. Evidence: `git ls-remote`, 2026-10-23 (earlier entries said `main` with no commits) |
| Purpose | Read-only Vulkan and OpenGL ES/EGL capability evidence (`zgfx`) | Verified. Evidence: `crates/`, `docs/adr/0001-probe-stack.md` |
| Licence | Apache-2.0 (`LICENSE`, `NOTICE`); not yet confirmed by the maintainer | **Unverified** (maintainer decision pending) |

## Contents

| Area | Location | Status |
|---|---|---|
| Rust workspace (MSRV 1.85; ash 0.38, libloading 0.8, serde_json) | `Cargo.toml`, `Cargo.lock` | Verified. Evidence: `cargo +1.85.0 test --workspace`, 2026-10-23 |
| Contract types and decoders, no I/O | `crates/zgfx-core` | Verified. Evidence: 13 tests, 2026-10-23 |
| Vulkan and GLES/EGL probes, CLI | `crates/zgfx` | Verified on a Linux host. Evidence: 3 unit + 9 CLI tests (3 need Mesa drivers), 2026-10-23. **Unverified** on Android |
| Android binaries | `scripts/build-android.sh`, `.cargo/config.toml` | Verified. Evidence: NDK r27d, three ABIs, 16 KB LOAD alignment, 2026-10-23 |
| CI | `.github/workflows/ci.yml` | actionlint clean. **Unverified** on GitHub |
| Docs | `README.md`, `docs/` | Verified. Evidence: files present |

## Key facts and limitations

- On Mesa 25.2.8 (Ubuntu 24.04, x86_64) `zgfx vulkan` enumerated lavapipe (`llvmpipe (LLVM
  20.1.2, 256 bits)`, vendor 0x10005, driver `mesa_llvmpipe`, device API 1.4.318, 161 device
  extensions) and `zgfx gles` created an ES 3 context reporting `OpenGL ES 3.2 Mesa 25.2.8`.
  Verified. Evidence: host runs and `cli.rs` with `--include-ignored`, 2026-10-23.
- Failure states reproduced on the host: missing loader or library, a library without the
  entry points, a loader without drivers (`no_driver`, `ERROR_INCOMPATIBLE_DRIVER`), Mesa EGL
  without a display server (`initialize_failed`, `EGL_NOT_INITIALIZED`). Verified. Evidence:
  same runs.
- The Android binaries need only libc and libdl at load time. Verified. Evidence:
  `llvm-readelf -d`, 2026-10-23. Whether a child process of an app can open the GPU driver is
  **Unverified**: no device was available.
- No control of any kind exists. Verified. Evidence: the code calls only Vulkan, EGL and GL
  query functions plus context setup and teardown.

## Authoritative sources

| Topic | Source |
|---|---|
| Repository facts | this file |
| Architecture | `AGENT/ARCHITECTURE.md`, `docs/adr/0001-probe-stack.md` |
| Contract | `docs/CONTRACT.md` |
| Testing | `docs/TESTING.md` |
| Licences, reference sources | `docs/THIRD_PARTY.md` |
| Organization standards | `ZairenkaiPrjctX/.github` → `agent-foundation/standards/` |

## Related repositories

- `Zairenkai-App`: intended consumer, running zgfx like zart and zthermal. **Unverified**: not
  integrated yet.
- `Zairenkai-Sdk`: should mirror `docs/CONTRACT.md` as a JSON Schema with captured output.
  **Unverified**: not done yet.
- `Zairenkai-Tuning`: its device identity leaves the GPU to this repository. Verified. Evidence:
  Tuning `crates/ztune-core` device module comment, 2026-10-22.

## Open questions

- Which device runs the first read-only validation, and as which uid (shell or app)?
- Confirm Apache-2.0 for this repository (maintainer).
- Should the GitHub description change from "GPU Tweaks" to match the read-only scope?

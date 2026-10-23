# Architecture: Zairenkai-Graphics-Platform

> Owned by this repository. Document only what inspected code supports, citing file paths.

## Overview

`zgfx` is a command-line program that probes the Vulkan and OpenGL ES/EGL stacks of the device
it runs on and prints one JSON document (`zairenkai.graphics` schema 1). It only queries.
Rationale: `docs/adr/0001-probe-stack.md`.

## Components

| Path | Responsibility |
|---|---|
| `crates/zgfx-core/src/lib.rs` | Envelope, error document, extension splitting, flag names, exit codes |
| `crates/zgfx-core/src/vulkan.rs` | Vulkan report types, statuses, version decoding, vendor and type names, driver-properties rule |
| `crates/zgfx-core/src/gles.rs` | GLES report types, statuses, EGL error names, ES version parsing, limits per ES version |
| `crates/zgfx/src/vulkan.rs` | Opens the loader (`ash::Entry::load_from`), one instance, physical-device queries |
| `crates/zgfx/src/gles.rs` | Opens libEGL/libGLESv2 (`libloading`), pbuffer + ES 3/2 context, GL queries, teardown (`Session` drop) |
| `crates/zgfx/src/main.rs` | Arguments, per-probe thread with a time limit, output |
| `scripts/build-android.sh` | NDK cross-build, 16 KB alignment check, checksums |

## Entry points

`zgfx vulkan | gles | probe | version`, options `--vulkan-library`, `--egl-library`,
`--gles-library`, `--timeout-ms` (`crates/zgfx/src/main.rs`).

## Data flow

Input: the system graphics libraries, opened at run time. Output: JSON on stdout, usage text on
stderr. Nothing is persisted.

## External interfaces

The JSON contract (`docs/CONTRACT.md`) for Zairenkai-App and, later, Zairenkai-Sdk.

## Trust boundaries

zgfx runs with the caller's privileges (the app's uid, or the shell). It needs no root and
writes nothing. A driver crash or hang stays inside the zgfx process; the caller sees an exit
code or a `timeout` document.

## Generated artifacts

`dist/` (from `scripts/build-android.sh`), not committed.

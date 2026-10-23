# ADR 0001: Read-only graphics probes as a Rust executable that opens the system loaders at run time

Status: proposed (2026-10-23). Needs maintainer review, like every first decision in a repository.

## Context

The ecosystem brief (graphics and tuning architecture) gives Graphics-Platform one first job:
probe Vulkan and OpenGL ES/EGL **independently**, on the real device and process, and report
measured values or a specific failure. It also sets the limits:

- read only. No driver installation or substitution (Mesa, Turnip, ANGLE), no `debug.*` or
  `vendor.*` properties, no forced GPU frequencies, no change to other apps' rendering;
- the GLES context belongs to the probing process, with correct lifecycle and thread handling,
  never created from a background service or on the UI thread;
- absent loader or driver, instance failure, no device, access denied, malformed output and
  timeout must be told apart;
- `vulkaninfo` need not be bundled if direct enumeration is better.

The other Zairenkai components the app runs (zart, zthermal) are Rust executables, MSRV 1.85,
built with NDK r27d, 16 KB-aligned, and run by the app from `nativeLibraryDir` with a fixed
argv. Their output is one JSON document with a schema name and version.

## Options

| | A. Rust executable, loaders opened at run time | B. Kotlin in the app (`EGL14`/`GLES30`, `PackageManager` features) | C. Bundle `vulkaninfo` and parse its output |
|---|---|---|---|
| Vulkan properties | full: loader version, devices, driver properties, extensions, limits, memory, queues | none from the runtime (Android has no Java Vulkan API); feature flags only, which the brief says are not proof | full, but in vulkaninfo's own format, which changes between releases |
| GLES context | owned by the probe process, its own thread | owned by the app process: a driver crash or hang takes the app with it | not covered |
| Driver crash or hang | contained in a child process; the app keeps running; a time limit ends the probe | kills or blocks the app | contained |
| Toolchain | the one Core, FAS, ART, ThermalTools and Tuning already use | the app's | C++ / CMake, a second native build |
| Host testing | yes, against Mesa lavapipe and llvmpipe | Robolectric has no GPU | yes |
| Licences | Apache-2.0, plus MIT/Apache/ISC crates | Apache-2.0 | Apache-2.0 plus its bundled dependencies |

## Decision

**Option A.** `zgfx` is a Rust executable with two crates:

- `zgfx-core`: the `zairenkai.graphics` schema 1 types and pure decoders (versions, flags, error
  names, limits per ES version). No I/O.
- `zgfx`: the probes. `ash` 0.38 (MIT OR Apache-2.0) with `Entry::load_from` opens the Vulkan
  loader at run time. A hand-written FFI of 13 EGL and 4 GL entry points over `libloading` 0.8
  (ISC) opens libEGL and libGLESv2. Nothing is linked at build time, so a device without Vulkan
  or GLES still runs zgfx and reports what is missing.

Each probe runs on its own thread with a time limit (default 5 s). The GLES probe creates a 1x1
pbuffer and an ES 3 (else ES 2) context, makes it current on that thread, reads strings and
limits, and releases everything on the same thread. The Vulkan probe creates one instance with
no layers or extensions and creates no logical device.

## Consequences

- Integration into the app follows the zart/zthermal pattern (packaged as `libzgfx.so` in
  `jniLibs`, run as the app's uid with a fixed argv). That is a separate App change, once this
  repository has a reviewed commit to lock.
- Whether a child process of an app can open the GPU driver the same way the app can is
  **unverified** until a device runs it. It is the expected behaviour (same uid and SELinux
  domain, public NDK libraries), not a measured one.
- No control of any kind is in scope. A future control needs its own ADR with an exact path and
  value allowlist, device and build scope, read-back, restore and hardware validation, executed
  through Core.

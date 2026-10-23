# Zairenkai-Graphics-Platform

Read-only graphics capability evidence for Zairenkai: `zgfx` probes **Vulkan** and **OpenGL
ES/EGL** separately on the device it runs on and prints one JSON document
(`zairenkai.graphics` schema 1, [docs/CONTRACT.md](docs/CONTRACT.md)).

```sh
zgfx vulkan    # loader, instance, physical devices: IDs, driver, extensions, features, limits, memory, queues
zgfx gles      # EGL display, an ES 3 (else ES 2) context owned by zgfx: strings, extensions, limits
zgfx probe     # both, each with its own status
```

zgfx only queries. It creates no logical device, draws nothing, writes no file, property or
setting, and never installs, replaces or selects a driver. A missing loader, a missing driver,
a failed context and a timeout are distinct statuses; no value is ever guessed.

- Decision and alternatives: [docs/adr/0001-probe-stack.md](docs/adr/0001-probe-stack.md)
- What has been tested where: [docs/TESTING.md](docs/TESTING.md). Host-tested on Mesa lavapipe
  and llvmpipe; **not yet run on any Android device**.
- Licences and the audited reference sources: [docs/THIRD_PARTY.md](docs/THIRD_PARTY.md)

## Build

```sh
cargo build --workspace && cargo test --workspace
ANDROID_NDK_HOME=<ndk r27d> scripts/build-android.sh   # dist/zgfx-<version>-<abi>, SHA256SUMS
```

Licence: Apache-2.0 ([LICENSE](LICENSE), [NOTICE](NOTICE)), pending the maintainer's
confirmation for this repository.

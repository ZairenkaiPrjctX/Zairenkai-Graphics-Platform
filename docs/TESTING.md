# Testing

Evidence levels follow the graphics architecture brief: `source_only`, `fixture_tested`,
`host_tested`, `emulator_tested`, `physical_device_read_only_tested`,
`physical_device_control_tested`, `unsupported`.

| What | Level | Evidence |
|---|---|---|
| Decoders, statuses, envelope | fixture_tested | `cargo test -p zgfx-core` |
| Missing and wrong libraries, usage errors, time limit | host_tested | `crates/zgfx/tests/cli.rs`, `main.rs` tests |
| Vulkan on Mesa lavapipe; no installable driver | host_tested | ignored tests in `cli.rs`, run with `--include-ignored` (below); also in CI, job `host-drivers` (run 39888606008) |
| ES 3.2 context on Mesa llvmpipe (EGL surfaceless platform) | host_tested | the same |
| Android emulator | not run | |
| Any phone | **not run**: no device was available | |

## Running

```sh
cargo test --workspace                      # no GPU needed; 3 driver tests are ignored
sudo apt-get install mesa-vulkan-drivers libegl-mesa0 libegl1 libgles2   # Ubuntu 24.04
cargo test --workspace -- --include-ignored
```

`EGL_PLATFORM=surfaceless` is Mesa's variable for a host without a display server; the tests set
it themselves. Android's EGL needs nothing like it. On a host without a display server and
without that variable, Mesa's `eglInitialize` fails, and zgfx reports `initialize_failed` with
`EGL_NOT_INITIALIZED`: an accurate report, not a zgfx defect.

## On a device (read only)

```sh
adb push dist/zgfx-0.1.0-arm64-v8a /data/local/tmp/zgfx && adb shell chmod 755 /data/local/tmp/zgfx
adb shell /data/local/tmp/zgfx probe
```

This runs as the shell user, not as the app's uid; record which one in a validation report.
zgfx writes nothing, so there is nothing to restore.

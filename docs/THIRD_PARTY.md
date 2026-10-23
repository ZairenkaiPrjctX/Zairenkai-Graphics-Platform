# Third-party software

## Crates linked into `zgfx`

From `Cargo.lock` (`cargo tree -p zgfx --target aarch64-linux-android -e normal`, 2026-10-23).
Each crate's licence text ships in its published package; the versions are pinned by
`Cargo.lock`.

| Crate | Version | Licence | Purpose |
|---|---|---|---|
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 | Vulkan bindings (headers 1.3.281); the loader is opened at run time |
| libloading | 0.8.9 | ISC | `dlopen` of libvulkan, libEGL and libGLESv2 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | dependency of libloading |
| serde | 1.0.229 | MIT OR Apache-2.0 | serialization |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | dependency of serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | JSON output |
| itoa | 1.0.18 | MIT OR Apache-2.0 | dependency of serde_json |
| memchr | 2.8.3 | Unlicense OR MIT | dependency of serde_json |
| zmij | 1.0.23 | MIT | dependency of serde_json (float formatting) |

Build-time only (procedural macros, not in the binary): serde_derive, proc-macro2, quote, syn,
unicode-ident ((MIT OR Apache-2.0) AND Unicode-3.0). `windows-link` is in `Cargo.lock` for
Windows targets only.

`libloading` 0.8 is used instead of 0.9 because 0.9 needs Rust 1.88, above this workspace's
MSRV (1.85); `ash` 0.38 depends on 0.8 as well.

## System libraries opened at run time

`libvulkan.so`, `libEGL.so` and `libGLESv2.so` (Android NDK public libraries; `.so.1` / `.so.2`
on Linux) belong to the device. zgfx neither ships nor replaces them.

## Reference sources (studied, nothing copied)

Audited 2026-10-23 at these commits (`git fetch --depth 1` of each default branch):

| Project | Commit | Licence (file at that commit) | Use here |
|---|---|---|---|
| KhronosGroup/Vulkan-Tools | `a967170a2b8a1ded4dbdf82d533caaec89184efe` | Apache-2.0 (`LICENSE.txt`; more in `LICENSES/`) | `vulkaninfo` as a reference for which properties matter. Not bundled: zgfx enumerates through the loader directly (ADR 0001) |
| KhronosGroup/Vulkan-Headers | `73814d006638aa00bd553215b054f0f02e457e22` | mixed per file (`LICENSE.md`: Apache-2.0 and MIT) | none directly; ash is generated from vk.xml 1.3.281 |
| KhronosGroup/VK-GL-CTS | `d18e4c68543103371715845cd914c71e5859963e` | Apache-2.0 | conformance testing is out of scope for a probe; a separate test lane if ever needed |
| google/angle | `df00ff99a7134bcda4e9e803d001fff92c7e51ae` | BSD-3-Clause (`LICENSE`) | reference only. zgfx never installs or selects ANGLE or any other driver |
| ash-rs/ash | `f4c2ca3e4f6b998d5254ad101a32f024d87cdec2` (repository head; the crate used is the published 0.38.0+1.3.281) | MIT OR Apache-2.0 | dependency (above) |
| nagisa/rust_libloading | `f5dc3b81485b978596d518f7574d941ff811bf74` (head; the crate used is 0.8.9) | ISC | dependency (above) |

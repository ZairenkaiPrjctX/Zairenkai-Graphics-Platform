# Contract: `zairenkai.graphics` schema 1

`zgfx` prints one JSON object on standard output. A consumer needs only this page.

## Envelope

| Field | Meaning |
|---|---|
| `schema` | `"zairenkai.graphics"` |
| `schema_version` | `1`. Refuse any other value |
| `kind` | `vulkan`, `gles`, `probe` (both reports) or `error` |
| `component` | `{"name": "zairenkai-graphics", "version": "<semver>"}` |
| `abi` | Architecture of the probing process (`aarch64`, `x86_64`, `arm`) |
| `collected_at_ms` | Wall clock, Unix milliseconds |

Within schema 1, fields and enum values may be added, never removed or changed in meaning.
Ignore unknown fields. Exit codes: `0` ok, `1` failed, `2` usage, `5` unsupported here (the API
is absent). `probe` exits `0` whenever it printed a document: read each report's `status`.

## `vulkan`

| Field | Content |
|---|---|
| `status` | see below |
| `result` | VkResult name of the failing call (`ERROR_INCOMPATIBLE_DRIVER`, ...), when one failed |
| `detail` | human-readable reason, when not `ok` |
| `library` | the loader path zgfx opened |
| `loader_api_version` | `vkEnumerateInstanceVersion` (1.0 when the loader lacks it), as `{raw, variant, major, minor, patch, text}` |
| `instance_api_version` | the version zgfx requested: the loader's, capped at 1.3 |
| `instance_extensions[]` | `{name, spec_version}`, sorted |
| `devices[]` | one per physical device, below |

| `status` | Meaning | Exit |
|---|---|---|
| `ok` | at least one physical device enumerated | 0 |
| `loader_missing` | the loader library could not be opened | 5 |
| `loader_incomplete` | it opened but lacks `vkGetInstanceProcAddr` or a core entry point | 1 |
| `no_driver` | `vkCreateInstance` returned `VK_ERROR_INCOMPATIBLE_DRIVER`: no installable driver | 5 |
| `instance_failed` | `vkCreateInstance` failed otherwise | 1 |
| `no_physical_device` | the instance reports none | 5 |
| `enumeration_failed` | a query after instance creation failed | 1 |
| `timeout` | no answer within the time limit | 1 |

A device: `index`, `vendor_id` (+ `vendor_id_hex`, display-only `vendor_name`), `device_id`
(+ hex), `device_type` (`integrated_gpu`, `discrete_gpu`, `virtual_gpu`, `cpu`, `other`),
`device_name`, `api_version`, `driver_version_raw` (vendor-specific encoding, not decoded),
`driver`, `extensions[]`, `features`, `limits`, `memory_heaps[]` (`index`, `size_bytes`,
`flags`), `memory_types[]` (`index`, `heap_index`, `flags`), `queue_families[]` (`index`,
`queue_count`, `flags`, `timestamp_valid_bits`).

`driver` is `VkPhysicalDeviceDriverProperties` (`driver_id`, `driver_id_name`, `driver_name`,
`driver_info`, `conformance_version`) with `status: "ok"`, or `status: "unsupported"` and a
`detail` when the device is below 1.2 without `VK_KHR_driver_properties`, or the instance is 1.0.

`features` (booleans): `robust_buffer_access`, `full_draw_index_uint32`, `geometry_shader`,
`tessellation_shader`, `multi_draw_indirect`, `sampler_anisotropy`, `texture_compression_etc2`,
`texture_compression_astc_ldr`, `texture_compression_bc`, `shader_float64`, `shader_int64`,
`shader_int16`, `fragment_stores_and_atomics`, `vertex_pipeline_stores_and_atomics`.

`limits`: `max_image_dimension_2d`, `max_image_dimension_3d`, `max_image_array_layers`,
`max_memory_allocation_count`, `max_bound_descriptor_sets`, `max_push_constants_size`,
`max_compute_shared_memory_size`, `max_compute_work_group_count` (3), `max_compute_work_group_invocations`,
`max_compute_work_group_size` (3), `max_sampler_anisotropy`, `max_viewports`,
`max_framebuffer_width`, `max_framebuffer_height`, `timestamp_compute_and_graphics`,
`timestamp_period_ns`.

Flag names: memory `device_local`, `host_visible`, `host_coherent`, `host_cached`,
`lazily_allocated`, `protected`, ...; heaps `device_local`, `multi_instance`; queues
`graphics`, `compute`, `transfer`, `sparse_binding`, `protected`, `video_decode`,
`video_encode`. A bit without a name appears as hex (`"0x200"`), never dropped.

## `gles`

| Field | Content |
|---|---|
| `status`, `result`, `detail` | as for Vulkan; `result` is the EGL error name |
| `egl_library`, `gles_library` | the paths opened |
| `egl` | `major`, `minor` (from `eglInitialize`), `vendor`, `version`, `client_apis`, `extensions[]` |
| `context` | `client_version` (3, else 2) and `surface` (`pbuffer`) of the context zgfx created |
| `gl` | `vendor`, `renderer`, `version`, `shading_language_version`, `es_version` (parsed from `version`, `"N.M"`, or null), `extensions[]`, `limits`, `limit_errors` |

| `status` | Meaning | Exit |
|---|---|---|
| `ok` | a context was current and queried | 0 |
| `egl_library_missing` / `gles_library_missing` | the library could not be opened | 5 |
| `library_incomplete` | a required entry point is missing | 1 |
| `no_display` | `eglGetDisplay(EGL_DEFAULT_DISPLAY)` returned `EGL_NO_DISPLAY` | 5 |
| `initialize_failed` | `eglInitialize` failed | 1 |
| `no_config` | no config renders ES 2 or 3 to a pbuffer | 5 |
| `surface_failed`, `context_failed`, `make_current_failed` | that EGL call failed | 1 |
| `timeout` | no answer within the time limit | 1 |

`limits` holds only the limits defined at `es_version`: ES 2.0 `max_texture_size`,
`max_cube_map_texture_size`, `max_renderbuffer_size`, `max_viewport_dims` (2),
`max_vertex_attribs`, `max_vertex_uniform_vectors`, `max_fragment_uniform_vectors`,
`max_texture_image_units`, `max_combined_texture_image_units`; ES 3.0 adds
`max_3d_texture_size`, `max_array_texture_layers`, `max_samples`, `max_color_attachments`,
`max_draw_buffers`; ES 3.1 adds `max_compute_work_group_invocations`,
`max_compute_shared_memory_size`. A limit the driver refuses with a GL error is absent from
`limits` and named in `limit_errors`.

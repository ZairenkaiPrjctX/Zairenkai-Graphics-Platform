// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! Vulkan probe: opens the loader at run time, creates one instance, enumerates physical
//! devices and reads their properties. It creates no logical device, allocates no memory and
//! submits no work: everything it calls is a query.

use std::collections::BTreeMap;
use std::ffi::c_char;

use ash::vk;
use serde_json::{json, Value};
use zgfx_core::vulkan::{
    device_type_name, driver_properties_available, requested_instance_version, vendor_name,
    ApiVersion, DriverProperties, Extension, MemoryHeap, MemoryType, PhysicalDevice, QueueFamily,
    VulkanReport, VulkanStatus,
};

/// The loader's file name: the NDK's public library on Android, the versioned SONAME elsewhere.
pub const DEFAULT_LIBRARY: &str = if cfg!(target_os = "android") {
    "libvulkan.so"
} else {
    "libvulkan.so.1"
};

fn name(r: vk::Result) -> String {
    format!("{r:?}")
}

/// A fixed-size C string from a Vulkan structure; stops at the first NUL or the array's end.
fn text(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn extensions(props: Vec<vk::ExtensionProperties>) -> Vec<Extension> {
    let mut v: Vec<Extension> = props
        .iter()
        .map(|p| Extension {
            name: text(&p.extension_name),
            spec_version: p.spec_version,
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

pub fn probe(library: &str) -> VulkanReport {
    let report = VulkanReport::new(library);
    // SAFETY: loading the system Vulkan loader runs its initialisers; that is the documented
    // way to use Vulkan without linking it, and this process exists only to probe.
    let entry = match unsafe { ash::Entry::load_from(library) } {
        Ok(e) => e,
        Err(ash::LoadingError::LibraryLoadFailure(e)) => {
            return report.fail(
                VulkanStatus::LoaderMissing,
                None,
                format!("cannot open {library}: {e}"),
            )
        }
        Err(ash::LoadingError::MissingEntryPoint(e)) => {
            return report.fail(
                VulkanStatus::LoaderIncomplete,
                None,
                format!("{library}: {e}"),
            )
        }
    };
    probe_entry(&entry, report)
}

fn probe_entry(entry: &ash::Entry, mut report: VulkanReport) -> VulkanReport {
    // SAFETY: plain queries on a loaded entry.
    let loader = match unsafe { entry.try_enumerate_instance_version() } {
        Ok(v) => v,
        Err(e) => {
            return report.fail(
                VulkanStatus::EnumerationFailed,
                Some(name(e)),
                "vkEnumerateInstanceVersion",
            )
        }
    };
    report.loader_api_version = Some(ApiVersion::from_raw(loader.unwrap_or(vk::API_VERSION_1_0)));
    match unsafe { entry.enumerate_instance_extension_properties(None) } {
        Ok(p) => report.instance_extensions = extensions(p),
        Err(e) => {
            return report.fail(
                VulkanStatus::EnumerationFailed,
                Some(name(e)),
                "vkEnumerateInstanceExtensionProperties",
            )
        }
    }

    let requested = requested_instance_version(loader);
    let instance_version = ApiVersion::from_raw(requested);
    report.instance_api_version = Some(instance_version.clone());
    let app = vk::ApplicationInfo::default()
        .application_name(c"zgfx")
        .engine_name(c"zgfx")
        .api_version(requested);
    // No layers and no extensions: the instance only enumerates.
    let info = vk::InstanceCreateInfo::default().application_info(&app);
    // SAFETY: the create info and the application info outlive the call.
    let instance = match unsafe { entry.create_instance(&info, None) } {
        Ok(i) => i,
        Err(e @ vk::Result::ERROR_INCOMPATIBLE_DRIVER) => {
            return report.fail(
                VulkanStatus::NoDriver,
                Some(name(e)),
                "vkCreateInstance: the loader found no installable Vulkan driver",
            )
        }
        Err(e) => {
            return report.fail(
                VulkanStatus::InstanceFailed,
                Some(name(e)),
                "vkCreateInstance",
            )
        }
    };

    let result = devices(&instance, &instance_version);
    // SAFETY: no object created from the instance outlives this point.
    unsafe { instance.destroy_instance(None) };
    match result {
        Ok(d) if d.is_empty() => report.fail(
            VulkanStatus::NoPhysicalDevice,
            None,
            "the instance reports no physical device",
        ),
        Ok(d) => {
            report.devices = d;
            report
        }
        Err((call, e)) => report.fail(VulkanStatus::EnumerationFailed, Some(name(e)), call),
    }
}

fn devices(
    instance: &ash::Instance,
    instance_version: &ApiVersion,
) -> Result<Vec<PhysicalDevice>, (&'static str, vk::Result)> {
    // SAFETY: every call below is a query on handles owned by `instance`.
    let handles = unsafe { instance.enumerate_physical_devices() }
        .map_err(|e| ("vkEnumeratePhysicalDevices", e))?;
    let mut out = Vec::new();
    for (index, pd) in handles.into_iter().enumerate() {
        let p = unsafe { instance.get_physical_device_properties(pd) };
        let exts = extensions(
            unsafe { instance.enumerate_device_extension_properties(pd) }
                .map_err(|e| ("vkEnumerateDeviceExtensionProperties", e))?,
        );
        let api = ApiVersion::from_raw(p.api_version);
        let driver = match driver_properties_available(instance_version, &api, &exts) {
            Ok(()) => driver_properties(instance, pd),
            Err(why) => DriverProperties::unsupported(why),
        };
        let mem = unsafe { instance.get_physical_device_memory_properties(pd) };
        let queues = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        let features = unsafe { instance.get_physical_device_features(pd) };
        out.push(PhysicalDevice {
            index: index as u32,
            vendor_id: p.vendor_id,
            vendor_id_hex: format!("{:#06x}", p.vendor_id),
            vendor_name: vendor_name(p.vendor_id),
            device_id: p.device_id,
            device_id_hex: format!("{:#06x}", p.device_id),
            device_type: device_type_name(p.device_type.as_raw()),
            device_name: text(&p.device_name),
            api_version: api,
            driver_version_raw: p.driver_version,
            driver,
            extensions: exts,
            features: feature_map(&features),
            limits: limit_map(&p.limits),
            memory_heaps: mem
                .memory_heaps_as_slice()
                .iter()
                .enumerate()
                .map(|(i, h)| MemoryHeap::new(i as u32, h.size, h.flags.as_raw()))
                .collect(),
            memory_types: mem
                .memory_types_as_slice()
                .iter()
                .enumerate()
                .map(|(i, t)| MemoryType::new(i as u32, t.heap_index, t.property_flags.as_raw()))
                .collect(),
            queue_families: queues
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    QueueFamily::new(
                        i as u32,
                        q.queue_count,
                        q.queue_flags.as_raw(),
                        q.timestamp_valid_bits,
                    )
                })
                .collect(),
        });
    }
    Ok(out)
}

fn driver_properties(instance: &ash::Instance, pd: vk::PhysicalDevice) -> DriverProperties {
    let mut driver = vk::PhysicalDeviceDriverProperties::default();
    let mut props2 = vk::PhysicalDeviceProperties2::default().push_next(&mut driver);
    // SAFETY: checked by driver_properties_available: the instance is 1.1+ (core entry point)
    // and the device knows the structure (1.2+ or VK_KHR_driver_properties).
    unsafe { instance.get_physical_device_properties2(pd, &mut props2) };
    let id = driver.driver_id;
    let id_name = format!("{id:?}");
    let c = driver.conformance_version;
    DriverProperties {
        status: "ok".into(),
        detail: None,
        driver_id: Some(id.as_raw()),
        // ash prints the bare number for an ID newer than its headers.
        driver_id_name: (!id_name.chars().all(|ch| ch.is_ascii_digit() || ch == '-'))
            .then(|| id_name.to_ascii_lowercase()),
        driver_name: Some(text(&driver.driver_name)),
        driver_info: Some(text(&driver.driver_info)),
        conformance_version: Some(format!(
            "{}.{}.{}.{}",
            c.major, c.minor, c.subminor, c.patch
        )),
    }
}

fn feature_map(f: &vk::PhysicalDeviceFeatures) -> BTreeMap<&'static str, bool> {
    let on = |b: vk::Bool32| b != vk::FALSE;
    BTreeMap::from([
        ("robust_buffer_access", on(f.robust_buffer_access)),
        ("full_draw_index_uint32", on(f.full_draw_index_uint32)),
        ("geometry_shader", on(f.geometry_shader)),
        ("tessellation_shader", on(f.tessellation_shader)),
        ("multi_draw_indirect", on(f.multi_draw_indirect)),
        ("sampler_anisotropy", on(f.sampler_anisotropy)),
        ("texture_compression_etc2", on(f.texture_compression_etc2)),
        (
            "texture_compression_astc_ldr",
            on(f.texture_compression_astc_ldr),
        ),
        ("texture_compression_bc", on(f.texture_compression_bc)),
        ("shader_float64", on(f.shader_float64)),
        ("shader_int64", on(f.shader_int64)),
        ("shader_int16", on(f.shader_int16)),
        (
            "fragment_stores_and_atomics",
            on(f.fragment_stores_and_atomics),
        ),
        (
            "vertex_pipeline_stores_and_atomics",
            on(f.vertex_pipeline_stores_and_atomics),
        ),
    ])
}

fn limit_map(l: &vk::PhysicalDeviceLimits) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        ("max_image_dimension_2d", json!(l.max_image_dimension2_d)),
        ("max_image_dimension_3d", json!(l.max_image_dimension3_d)),
        ("max_image_array_layers", json!(l.max_image_array_layers)),
        (
            "max_memory_allocation_count",
            json!(l.max_memory_allocation_count),
        ),
        (
            "max_bound_descriptor_sets",
            json!(l.max_bound_descriptor_sets),
        ),
        ("max_push_constants_size", json!(l.max_push_constants_size)),
        (
            "max_compute_shared_memory_size",
            json!(l.max_compute_shared_memory_size),
        ),
        (
            "max_compute_work_group_count",
            json!(l.max_compute_work_group_count),
        ),
        (
            "max_compute_work_group_invocations",
            json!(l.max_compute_work_group_invocations),
        ),
        (
            "max_compute_work_group_size",
            json!(l.max_compute_work_group_size),
        ),
        ("max_sampler_anisotropy", json!(l.max_sampler_anisotropy)),
        ("max_viewports", json!(l.max_viewports)),
        ("max_framebuffer_width", json!(l.max_framebuffer_width)),
        ("max_framebuffer_height", json!(l.max_framebuffer_height)),
        (
            "timestamp_compute_and_graphics",
            json!(l.timestamp_compute_and_graphics != vk::FALSE),
        ),
        ("timestamp_period_ns", json!(l.timestamp_period)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_size_names_stop_at_the_first_nul_or_the_end() {
        let mut a = [0 as c_char; 8];
        for (i, b) in b"llvm".iter().enumerate() {
            a[i] = *b as c_char;
        }
        assert_eq!(text(&a), "llvm");
        let full = [b'x' as c_char; 4];
        assert_eq!(text(&full), "xxxx");
    }
}

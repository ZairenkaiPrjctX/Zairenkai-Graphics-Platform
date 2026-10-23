// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! The Vulkan report (`kind: "vulkan"`).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::{exit, flag_names};

/// Outcome of the Vulkan probe. Each failure is distinct, so "no Vulkan here" is never confused
/// with "Vulkan is here but the probe failed".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VulkanStatus {
    Ok,
    /// The loader library could not be opened (no Vulkan on this device or in this process).
    LoaderMissing,
    /// The loader opened but lacks `vkGetInstanceProcAddr` or a core entry point.
    LoaderIncomplete,
    /// The loader found no installable driver (ICD): `vkCreateInstance` returned
    /// `VK_ERROR_INCOMPATIBLE_DRIVER`.
    NoDriver,
    /// `vkCreateInstance` failed otherwise; `result` names the VkResult.
    InstanceFailed,
    /// An instance was created but it reports no physical device.
    NoPhysicalDevice,
    /// A query after instance creation failed; `result` names the VkResult.
    EnumerationFailed,
    /// The driver did not answer within the time limit.
    Timeout,
}

impl VulkanStatus {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Ok => exit::OK,
            Self::LoaderMissing | Self::NoDriver | Self::NoPhysicalDevice => exit::UNSUPPORTED,
            _ => exit::FAILED,
        }
    }
}

/// A packed Vulkan version (`VK_MAKE_API_VERSION`): variant 3 bits, major 7, minor 10, patch 12.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiVersion {
    pub raw: u32,
    pub variant: u32,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub text: String,
}

impl ApiVersion {
    pub fn from_raw(raw: u32) -> Self {
        let (variant, major, minor, patch) = (
            raw >> 29,
            (raw >> 22) & 0x7f,
            (raw >> 12) & 0x3ff,
            raw & 0xfff,
        );
        Self {
            raw,
            variant,
            major,
            minor,
            patch,
            text: format!("{major}.{minor}.{patch}"),
        }
    }

    /// `VK_MAKE_API_VERSION(0, major, minor, 0)`.
    pub const fn make(major: u32, minor: u32) -> u32 {
        (major << 22) | (minor << 12)
    }

    /// Major and minor only, for comparisons (`VK_API_VERSION_1_2` and the like).
    pub fn at_least(&self, major: u32, minor: u32) -> bool {
        (self.major, self.minor) >= (major, minor)
    }
}

/// The highest instance version zgfx asks for. Properties beyond it are not queried, so a newer
/// loader or driver never makes zgfx read a structure it does not know.
pub const MAX_INSTANCE_VERSION: (u32, u32) = (1, 3);

/// The version to request at instance creation: the loader's own version, capped at
/// [`MAX_INSTANCE_VERSION`]. A 1.0 loader (no `vkEnumerateInstanceVersion`) gets 1.0.
pub fn requested_instance_version(loader: Option<u32>) -> u32 {
    let l = ApiVersion::from_raw(loader.unwrap_or(ApiVersion::make(1, 0)));
    let (cap_major, cap_minor) = MAX_INSTANCE_VERSION;
    if l.at_least(cap_major, cap_minor) {
        ApiVersion::make(cap_major, cap_minor)
    } else {
        ApiVersion::make(l.major, l.minor)
    }
}

/// Display names for PCI-SIG vendor IDs common on Android and desktop GPUs, and the Khronos
/// `VkVendorId` values (vk.xml). Display only: decisions use the numeric ID.
pub fn vendor_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0x1002 => "AMD",
        0x1010 => "Imagination Technologies",
        0x106B => "Apple",
        0x10DE => "NVIDIA",
        0x13B5 => "Arm",
        0x1414 => "Microsoft",
        0x144D => "Samsung",
        0x14E4 => "Broadcom",
        0x1AE0 => "Google",
        0x5143 => "Qualcomm",
        0x8086 => "Intel",
        0x1_0001 => "Vivante",
        0x1_0002 => "VeriSilicon",
        0x1_0003 => "Kazan",
        0x1_0004 => "Codeplay",
        0x1_0005 => "Mesa",
        0x1_0006 => "PoCL",
        0x1_0007 => "Mobileye",
        _ => return None,
    })
}

pub fn device_type_name(t: i32) -> &'static str {
    match t {
        0 => "other",
        1 => "integrated_gpu",
        2 => "discrete_gpu",
        3 => "virtual_gpu",
        4 => "cpu",
        _ => "unknown",
    }
}

/// `VkMemoryPropertyFlagBits`.
pub const MEMORY_PROPERTY_FLAGS: &[(u32, &str)] = &[
    (0x1, "device_local"),
    (0x2, "host_visible"),
    (0x4, "host_coherent"),
    (0x8, "host_cached"),
    (0x10, "lazily_allocated"),
    (0x20, "protected"),
    (0x40, "device_coherent_amd"),
    (0x80, "device_uncached_amd"),
    (0x100, "rdma_capable_nv"),
];

/// `VkMemoryHeapFlagBits`.
pub const MEMORY_HEAP_FLAGS: &[(u32, &str)] = &[(0x1, "device_local"), (0x2, "multi_instance")];

/// `VkQueueFlagBits`.
pub const QUEUE_FLAGS: &[(u32, &str)] = &[
    (0x1, "graphics"),
    (0x2, "compute"),
    (0x4, "transfer"),
    (0x8, "sparse_binding"),
    (0x10, "protected"),
    (0x20, "video_decode"),
    (0x40, "video_encode"),
    (0x100, "optical_flow_nv"),
];

#[derive(Debug, Clone, Serialize)]
pub struct Extension {
    pub name: String,
    pub spec_version: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryHeap {
    pub index: u32,
    pub size_bytes: u64,
    pub flags: Vec<String>,
}

impl MemoryHeap {
    pub fn new(index: u32, size_bytes: u64, flags: u32) -> Self {
        Self {
            index,
            size_bytes,
            flags: flag_names(flags, MEMORY_HEAP_FLAGS),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryType {
    pub index: u32,
    pub heap_index: u32,
    pub flags: Vec<String>,
}

impl MemoryType {
    pub fn new(index: u32, heap_index: u32, flags: u32) -> Self {
        Self {
            index,
            heap_index,
            flags: flag_names(flags, MEMORY_PROPERTY_FLAGS),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueFamily {
    pub index: u32,
    pub queue_count: u32,
    pub flags: Vec<String>,
    pub timestamp_valid_bits: u32,
}

impl QueueFamily {
    pub fn new(index: u32, queue_count: u32, flags: u32, timestamp_valid_bits: u32) -> Self {
        Self {
            index,
            queue_count,
            flags: flag_names(flags, QUEUE_FLAGS),
            timestamp_valid_bits,
        }
    }
}

/// `VkPhysicalDeviceDriverProperties`, queried only when the device is Vulkan 1.2+ or exposes
/// `VK_KHR_driver_properties` and the instance is 1.1+; otherwise `status` says why not.
#[derive(Debug, Clone, Serialize)]
pub struct DriverProperties {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_id_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conformance_version: Option<String>,
}

impl DriverProperties {
    pub fn unsupported(detail: impl Into<String>) -> Self {
        Self {
            status: "unsupported".into(),
            detail: Some(detail.into()),
            driver_id: None,
            driver_id_name: None,
            driver_name: None,
            driver_info: None,
            conformance_version: None,
        }
    }
}

/// When `VkPhysicalDeviceDriverProperties` can be queried: the device is 1.2+ or exposes
/// `VK_KHR_driver_properties`, and `vkGetPhysicalDeviceProperties2` exists (instance 1.1+).
pub fn driver_properties_available(
    instance: &ApiVersion,
    device: &ApiVersion,
    extensions: &[Extension],
) -> Result<(), String> {
    if !instance.at_least(1, 1) {
        return Err(format!(
            "instance version {} has no vkGetPhysicalDeviceProperties2",
            instance.text
        ));
    }
    if device.at_least(1, 2)
        || extensions
            .iter()
            .any(|e| e.name == "VK_KHR_driver_properties")
    {
        Ok(())
    } else {
        Err(format!(
            "device version {} without VK_KHR_driver_properties",
            device.text
        ))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalDevice {
    pub index: u32,
    pub vendor_id: u32,
    pub vendor_id_hex: String,
    pub vendor_name: Option<&'static str>,
    pub device_id: u32,
    pub device_id_hex: String,
    pub device_type: &'static str,
    pub device_name: String,
    pub api_version: ApiVersion,
    /// Vendor-specific encoding (NVIDIA and Qualcomm pack it differently): reported raw.
    pub driver_version_raw: u32,
    pub driver: DriverProperties,
    pub extensions: Vec<Extension>,
    /// Selected `VkPhysicalDeviceFeatures` (`docs/CONTRACT.md` lists them).
    pub features: BTreeMap<&'static str, bool>,
    /// Selected `VkPhysicalDeviceLimits`.
    pub limits: BTreeMap<&'static str, Value>,
    pub memory_heaps: Vec<MemoryHeap>,
    pub memory_types: Vec<MemoryType>,
    pub queue_families: Vec<QueueFamily>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VulkanReport {
    pub status: VulkanStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The VkResult name of the failing call (`ERROR_INCOMPATIBLE_DRIVER`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub library: String,
    pub loader_api_version: Option<ApiVersion>,
    pub instance_api_version: Option<ApiVersion>,
    pub instance_extensions: Vec<Extension>,
    pub devices: Vec<PhysicalDevice>,
}

impl VulkanReport {
    pub fn new(library: impl Into<String>) -> Self {
        Self {
            status: VulkanStatus::Ok,
            detail: None,
            result: None,
            library: library.into(),
            loader_api_version: None,
            instance_api_version: None,
            instance_extensions: Vec::new(),
            devices: Vec::new(),
        }
    }

    /// Record a failure; whatever was collected before it stays in the report.
    pub fn fail(
        mut self,
        status: VulkanStatus,
        result: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        self.status = status;
        self.result = result;
        self.detail = Some(detail.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_decode_like_vk_api_version_macros() {
        // VK_MAKE_API_VERSION(0, 1, 3, 275)
        let v = ApiVersion::from_raw((1 << 22) | (3 << 12) | 275);
        assert_eq!((v.variant, v.major, v.minor, v.patch), (0, 1, 3, 275));
        assert_eq!(v.text, "1.3.275");
        assert!(v.at_least(1, 2) && v.at_least(1, 3) && !v.at_least(1, 4));
        assert_eq!(ApiVersion::from_raw(ApiVersion::make(1, 1)).text, "1.1.0");
    }

    #[test]
    fn the_requested_instance_version_is_capped() {
        assert_eq!(requested_instance_version(None), ApiVersion::make(1, 0));
        assert_eq!(
            requested_instance_version(Some(ApiVersion::make(1, 1) | 120)),
            ApiVersion::make(1, 1)
        );
        assert_eq!(
            requested_instance_version(Some(ApiVersion::make(1, 4) | 3)),
            ApiVersion::make(1, 3)
        );
    }

    #[test]
    fn driver_properties_need_1_2_or_the_extension_and_an_1_1_instance() {
        let v = |major, minor| ApiVersion::from_raw(ApiVersion::make(major, minor));
        let ext = |n: &str| {
            vec![Extension {
                name: n.into(),
                spec_version: 1,
            }]
        };
        assert!(driver_properties_available(&v(1, 3), &v(1, 2), &[]).is_ok());
        assert!(
            driver_properties_available(&v(1, 1), &v(1, 1), &ext("VK_KHR_driver_properties"))
                .is_ok()
        );
        assert!(driver_properties_available(&v(1, 1), &v(1, 1), &[])
            .unwrap_err()
            .contains("VK_KHR_driver_properties"));
        assert!(driver_properties_available(&v(1, 0), &v(1, 3), &[])
            .unwrap_err()
            .contains("vkGetPhysicalDeviceProperties2"));
    }

    #[test]
    fn vendors_and_types_have_display_names() {
        assert_eq!(vendor_name(0x5143), Some("Qualcomm"));
        assert_eq!(vendor_name(0x13B5), Some("Arm"));
        assert_eq!(vendor_name(0x1_0005), Some("Mesa"));
        assert_eq!(vendor_name(0xFFFF), None);
        assert_eq!(device_type_name(1), "integrated_gpu");
        assert_eq!(device_type_name(9), "unknown");
    }

    #[test]
    fn statuses_serialize_and_map_to_exit_codes() {
        assert_eq!(
            serde_json::to_value(VulkanStatus::NoPhysicalDevice).unwrap(),
            "no_physical_device"
        );
        assert_eq!(VulkanStatus::Ok.exit_code(), 0);
        assert_eq!(VulkanStatus::LoaderMissing.exit_code(), 5);
        assert_eq!(VulkanStatus::NoDriver.exit_code(), 5);
        assert_eq!(VulkanStatus::InstanceFailed.exit_code(), 1);
        assert_eq!(VulkanStatus::Timeout.exit_code(), 1);
    }

    #[test]
    fn a_failure_keeps_what_was_collected() {
        let mut r = VulkanReport::new("libvulkan.so");
        r.loader_api_version = Some(ApiVersion::from_raw(ApiVersion::make(1, 3)));
        let r = r.fail(
            VulkanStatus::InstanceFailed,
            Some("ERROR_INCOMPATIBLE_DRIVER".into()),
            "vkCreateInstance",
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["status"], "instance_failed");
        assert_eq!(v["result"], "ERROR_INCOMPATIBLE_DRIVER");
        assert_eq!(v["loader_api_version"]["text"], "1.3.0");
        assert!(v["devices"].as_array().unwrap().is_empty());
    }
}

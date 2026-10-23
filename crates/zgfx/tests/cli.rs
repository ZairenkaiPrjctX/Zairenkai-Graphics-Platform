// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! The zgfx binary end to end. Missing and wrong libraries need no GPU and always run. The
//! real-driver tests need Mesa's software drivers (lavapipe, llvmpipe EGL); they are ignored by
//! default and CI runs them with `--include-ignored` after installing Mesa (docs/TESTING.md).

use std::process::Command;

use serde_json::Value;

fn zgfx(args: &[&str], env: &[(&str, &str)]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_zgfx"))
        .args(args)
        .envs(env.iter().copied())
        .output()
        .expect("run zgfx");
    let doc = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out.status.code().expect("exit code"), doc)
}

fn assert_envelope(doc: &Value, kind: &str) {
    assert_eq!(doc["schema"], "zairenkai.graphics");
    assert_eq!(doc["schema_version"], 1);
    assert_eq!(doc["kind"], kind);
    assert_eq!(doc["component"]["name"], "zairenkai-graphics");
    assert_eq!(doc["abi"], std::env::consts::ARCH);
    assert!(doc["collected_at_ms"].as_u64().unwrap() > 0);
}

#[test]
fn a_missing_vulkan_loader_is_unsupported_not_a_failure() {
    let (code, doc) = zgfx(
        &["vulkan", "--vulkan-library", "/nonexistent/libvulkan.so"],
        &[],
    );
    assert_envelope(&doc, "vulkan");
    assert_eq!(code, 5);
    assert_eq!(doc["vulkan"]["status"], "loader_missing");
    assert!(doc["vulkan"]["devices"].as_array().unwrap().is_empty());
}

#[test]
fn a_library_without_vulkan_entry_points_is_an_incomplete_loader() {
    let (code, doc) = zgfx(&["vulkan", "--vulkan-library", "libc.so.6"], &[]);
    assert_eq!(code, 1);
    assert_eq!(doc["vulkan"]["status"], "loader_incomplete");
    assert!(doc["vulkan"]["detail"]
        .as_str()
        .unwrap()
        .contains("vkGetInstanceProcAddr"));
}

#[test]
fn missing_egl_and_gles_libraries_are_reported_separately() {
    let (code, doc) = zgfx(&["gles", "--egl-library", "/nonexistent/libEGL.so"], &[]);
    assert_envelope(&doc, "gles");
    assert_eq!(code, 5);
    assert_eq!(doc["gles"]["status"], "egl_library_missing");
    assert!(doc["gles"]["gl"].is_null());
    let (code, doc) = zgfx(
        &[
            "gles",
            "--egl-library",
            "libc.so.6",
            "--gles-library",
            "/nonexistent/libGLESv2.so",
        ],
        &[],
    );
    assert_eq!(code, 5);
    assert_eq!(doc["gles"]["status"], "gles_library_missing");
}

#[test]
fn a_library_without_egl_entry_points_is_incomplete() {
    let (code, doc) = zgfx(
        &[
            "gles",
            "--egl-library",
            "libc.so.6",
            "--gles-library",
            "libc.so.6",
        ],
        &[],
    );
    assert_eq!(code, 1);
    assert_eq!(doc["gles"]["status"], "library_incomplete");
    assert!(doc["gles"]["detail"]
        .as_str()
        .unwrap()
        .contains("eglGetDisplay"));
}

#[test]
fn probe_reports_each_api_with_its_own_status() {
    let (code, doc) = zgfx(
        &[
            "probe",
            "--vulkan-library",
            "/nonexistent/libvulkan.so",
            "--egl-library",
            "/nonexistent/libEGL.so",
        ],
        &[],
    );
    assert_envelope(&doc, "probe");
    assert_eq!(code, 0);
    assert_eq!(doc["vulkan"]["status"], "loader_missing");
    assert_eq!(doc["gles"]["status"], "egl_library_missing");
}

#[test]
fn usage_errors_are_documents_with_exit_2() {
    let (code, doc) = zgfx(&["frobnicate"], &[]);
    assert_eq!(code, 2);
    assert_envelope(&doc, "error");
    assert_eq!(doc["error"]["code"], "usage");
    let (code, doc) = zgfx(&["vulkan", "--timeout-ms", "0"], &[]);
    assert_eq!(code, 2);
    assert!(doc["error"]["message"]
        .as_str()
        .unwrap()
        .contains("--timeout-ms"));
}

// Real drivers: Mesa lavapipe (Vulkan on the CPU) and llvmpipe through EGL's surfaceless platform.
const LVP_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";

#[test]
#[ignore = "needs Mesa lavapipe/llvmpipe (docs/TESTING.md)"]
fn lavapipe_is_enumerated_with_driver_properties() {
    let (code, doc) = zgfx(&["vulkan"], &[("VK_ICD_FILENAMES", LVP_ICD)]);
    let v = &doc["vulkan"];
    assert_eq!((code, v["status"].as_str()), (0, Some("ok")), "{doc:#}");
    assert!(v["loader_api_version"]["major"].as_u64() == Some(1));
    let d = &v["devices"][0];
    assert!(d["device_name"].as_str().unwrap().starts_with("llvmpipe"));
    assert_eq!(d["vendor_id"], 0x1_0005); // VK_VENDOR_ID_MESA
    assert_eq!(d["device_type"], "cpu");
    assert_eq!(d["driver"]["status"], "ok");
    assert_eq!(d["driver"]["driver_id_name"], "mesa_llvmpipe");
    assert!(d["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["name"] == "VK_KHR_driver_properties"));
    assert!(d["limits"]["max_image_dimension_2d"].as_u64().unwrap() >= 4096);
    assert!(!d["memory_heaps"].as_array().unwrap().is_empty());
    assert!(d["queue_families"][0]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "compute"));
}

#[test]
#[ignore = "needs Mesa lavapipe/llvmpipe (docs/TESTING.md)"]
fn a_loader_without_drivers_reports_no_driver() {
    let (code, doc) = zgfx(
        &["vulkan"],
        &[("VK_ICD_FILENAMES", "/nonexistent/icd.json")],
    );
    assert_eq!(code, 5);
    assert_eq!(doc["vulkan"]["status"], "no_driver");
    assert_eq!(doc["vulkan"]["result"], "ERROR_INCOMPATIBLE_DRIVER");
    assert!(doc["vulkan"]["loader_api_version"].is_object());
}

#[test]
#[ignore = "needs Mesa lavapipe/llvmpipe (docs/TESTING.md)"]
fn llvmpipe_answers_from_an_es3_context_owned_by_zgfx() {
    let (code, doc) = zgfx(&["gles"], &[("EGL_PLATFORM", "surfaceless")]);
    let g = &doc["gles"];
    assert_eq!((code, g["status"].as_str()), (0, Some("ok")), "{doc:#}");
    assert!(g["egl"]["major"].as_i64().unwrap() >= 1);
    assert!(!g["egl"]["extensions"].as_array().unwrap().is_empty());
    assert_eq!(g["context"]["client_version"], 3);
    assert_eq!(g["context"]["surface"], "pbuffer");
    let gl = &g["gl"];
    assert!(gl["renderer"].as_str().unwrap().starts_with("llvmpipe"));
    assert!(gl["version"].as_str().unwrap().starts_with("OpenGL ES 3."));
    assert!(gl["es_version"].as_str().unwrap().starts_with("3."));
    assert!(gl["extensions"].as_array().unwrap().len() > 10);
    assert!(gl["limits"]["max_texture_size"].as_i64().unwrap() >= 2048);
    assert_eq!(
        gl["limits"]["max_viewport_dims"].as_array().unwrap().len(),
        2
    );
    assert!(gl["limit_errors"].as_object().unwrap().is_empty());
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! `zgfx` command line: one JSON document (`zairenkai.graphics` schema 1) on standard output.

mod gles;
mod vulkan;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use zgfx_core::gles::{GlesReport, GlesStatus};
use zgfx_core::vulkan::{VulkanReport, VulkanStatus};
use zgfx_core::{envelope, error_document, exit, to_value};

const USAGE: &str = "\
zgfx - read-only Vulkan and OpenGL ES/EGL capability probes (Zairenkai-Graphics-Platform)

USAGE:
  zgfx vulkan [OPTIONS]   Vulkan loader, instance and physical devices (JSON)
  zgfx gles [OPTIONS]     EGL display and an OpenGL ES context owned by this process (JSON)
  zgfx probe [OPTIONS]    both, each with its own status
  zgfx version

OPTIONS:
  --vulkan-library PATH   Vulkan loader to open (default: libvulkan.so on Android)
  --egl-library PATH      EGL library (default: libEGL.so on Android)
  --gles-library PATH     OpenGL ES library (default: libGLESv2.so on Android)
  --timeout-ms N          time limit per probe, 1-60000 (default 5000)

zgfx only queries: it creates no logical device, draws nothing, and writes no file, property
or setting. Exit codes: 0 ok, 1 failed, 2 usage, 5 unsupported here; `probe` exits 0 when it
printed a document.
";

#[derive(Debug)]
struct Options {
    vulkan_library: String,
    egl_library: String,
    gles_library: String,
    timeout: Duration,
}

fn parse(args: &[String]) -> Result<(String, Options), String> {
    let mut it = args.iter();
    let command = it.next().ok_or("no command")?.clone();
    let mut o = Options {
        vulkan_library: vulkan::DEFAULT_LIBRARY.into(),
        egl_library: gles::DEFAULT_EGL_LIBRARY.into(),
        gles_library: gles::DEFAULT_GLES_LIBRARY.into(),
        timeout: Duration::from_millis(5000),
    };
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--vulkan-library" => o.vulkan_library = value.clone(),
            "--egl-library" => o.egl_library = value.clone(),
            "--gles-library" => o.gles_library = value.clone(),
            "--timeout-ms" => match value.parse::<u64>() {
                Ok(ms @ 1..=60_000) => o.timeout = Duration::from_millis(ms),
                _ => return Err(format!("--timeout-ms must be 1-60000, got {value:?}")),
            },
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    Ok((command, o))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Run a probe on its own thread. A driver that never returns leaves the thread blocked; the
/// process ends without it when `main` returns.
fn with_timeout<T: Send + 'static>(
    timeout: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("zgfx-probe".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .ok()?;
    rx.recv_timeout(timeout).ok()
}

fn run_vulkan(o: &Options) -> VulkanReport {
    let lib = o.vulkan_library.clone();
    with_timeout(o.timeout, move || vulkan::probe(&lib)).unwrap_or_else(|| {
        VulkanReport::new(&o.vulkan_library).fail(
            VulkanStatus::Timeout,
            None,
            format!("no answer within {} ms", o.timeout.as_millis()),
        )
    })
}

fn run_gles(o: &Options) -> GlesReport {
    let (egl, gles) = (o.egl_library.clone(), o.gles_library.clone());
    with_timeout(o.timeout, move || gles::probe(&egl, &gles)).unwrap_or_else(|| {
        GlesReport::new(&o.egl_library, &o.gles_library).fail(
            GlesStatus::Timeout,
            None,
            format!("no answer within {} ms", o.timeout.as_millis()),
        )
    })
}

fn print(doc: &Value) {
    match serde_json::to_string_pretty(doc) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("zgfx: cannot encode JSON: {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arch = std::env::consts::ARCH;
    if args.first().map(String::as_str) == Some("version") && args.len() == 1 {
        println!("zgfx {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let code = match parse(&args) {
        Ok((cmd, o)) if cmd == "vulkan" => {
            let r = run_vulkan(&o);
            print(&envelope(
                "vulkan",
                arch,
                now_ms(),
                json!({"vulkan": to_value(&r)}),
            ));
            r.status.exit_code()
        }
        Ok((cmd, o)) if cmd == "gles" => {
            let r = run_gles(&o);
            print(&envelope(
                "gles",
                arch,
                now_ms(),
                json!({"gles": to_value(&r)}),
            ));
            r.status.exit_code()
        }
        Ok((cmd, o)) if cmd == "probe" => {
            let (v, g) = (run_vulkan(&o), run_gles(&o));
            print(&envelope(
                "probe",
                arch,
                now_ms(),
                json!({"vulkan": to_value(&v), "gles": to_value(&g)}),
            ));
            exit::OK
        }
        Ok((cmd, _)) => usage(arch, &format!("unknown command {cmd:?}")),
        Err(e) => usage(arch, &e),
    };
    std::process::exit(code);
}

fn usage(arch: &str, message: &str) -> i32 {
    print(&error_document(arch, now_ms(), "usage", message));
    eprint!("zgfx: {message}\n\n{USAGE}");
    exit::USAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_probe_that_does_not_answer_times_out() {
        let r = with_timeout(Duration::from_millis(50), || {
            thread::sleep(Duration::from_secs(5));
            1
        });
        assert_eq!(r, None);
        assert_eq!(with_timeout(Duration::from_secs(5), || 7), Some(7));
    }

    #[test]
    fn options_are_validated() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let (cmd, o) = parse(&a(&[
            "vulkan",
            "--vulkan-library",
            "/x/libvulkan.so",
            "--timeout-ms",
            "250",
        ]))
        .unwrap();
        assert_eq!(cmd, "vulkan");
        assert_eq!(o.vulkan_library, "/x/libvulkan.so");
        assert_eq!(o.timeout, Duration::from_millis(250));
        assert!(parse(&a(&["gles", "--timeout-ms", "60001"])).is_err());
        assert!(parse(&a(&["gles", "--egl-library"]))
            .unwrap_err()
            .contains("needs a value"));
        assert!(parse(&a(&["gles", "--color", "red"]))
            .unwrap_err()
            .contains("unknown option"));
        assert!(parse(&[]).is_err());
    }
}

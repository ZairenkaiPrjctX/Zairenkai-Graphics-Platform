// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! The `zairenkai.graphics` contract, schema 1 (`docs/CONTRACT.md`): report types and the pure
//! decoders zgfx uses to fill them. No I/O and no FFI: the probes live in the `zgfx` crate.
//!
//! Vulkan and OpenGL ES/EGL are separate reports with separate statuses. A failed probe carries
//! its status and the API's own error name; it never carries a guessed value.

pub mod gles;
pub mod vulkan;

use serde::Serialize;
use serde_json::{json, Value};

pub const SCHEMA: &str = "zairenkai.graphics";
pub const SCHEMA_VERSION: u32 = 1;
pub const COMPONENT: &str = "zairenkai-graphics";

/// Exit codes, shared with the other Zairenkai tools: the document is printed in every case and
/// is the source of truth; the code is a summary.
pub mod exit {
    pub const OK: i32 = 0;
    pub const FAILED: i32 = 1;
    pub const USAGE: i32 = 2;
    /// The API is absent here (no loader or library, no device, no matching config).
    pub const UNSUPPORTED: i32 = 5;
}

/// Wrap a report in the schema-1 envelope. `abi` is the probing process's architecture and
/// `collected_at_ms` its wall clock (Unix milliseconds).
pub fn envelope(kind: &str, abi: &str, collected_at_ms: u64, body: Value) -> Value {
    let mut doc = json!({
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "kind": kind,
        "component": {"name": COMPONENT, "version": env!("CARGO_PKG_VERSION")},
        "abi": abi,
        "collected_at_ms": collected_at_ms,
    });
    if let (Some(d), Value::Object(b)) = (doc.as_object_mut(), body) {
        d.extend(b);
    }
    doc
}

/// A usage or internal error document (`kind: "error"`).
pub fn error_document(abi: &str, collected_at_ms: u64, code: &str, message: &str) -> Value {
    envelope(
        "error",
        abi,
        collected_at_ms,
        json!({"error": {"code": code, "message": message}}),
    )
}

/// Split a space-separated extension string (EGL, GLES 2) into a sorted list without
/// duplicates or empty entries.
pub fn split_extensions(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.split_ascii_whitespace().map(str::to_owned).collect();
    v.sort();
    v.dedup();
    v
}

/// Names of the set bits of a flag word, in bit order; a bit without a name is kept as hex so
/// nothing is dropped.
pub fn flag_names(bits: u32, names: &[(u32, &'static str)]) -> Vec<String> {
    (0..32)
        .map(|i| 1u32 << i)
        .filter(|b| bits & b != 0)
        .map(|b| match names.iter().find(|(v, _)| *v == b) {
            Some((_, n)) => (*n).to_owned(),
            None => format!("{b:#x}"),
        })
        .collect()
}

/// Serialize a report into a JSON object value (reports never fail to serialize).
pub fn to_value<T: Serialize>(t: &T) -> Value {
    serde_json::to_value(t).unwrap_or_else(|e| json!({"status": "failed", "detail": e.to_string()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_carries_the_contract_identity() {
        let d = envelope("vulkan", "aarch64", 7, json!({"vulkan": {"status": "ok"}}));
        assert_eq!(d["schema"], SCHEMA);
        assert_eq!(d["schema_version"], 1);
        assert_eq!(d["kind"], "vulkan");
        assert_eq!(d["component"]["name"], COMPONENT);
        assert_eq!(d["abi"], "aarch64");
        assert_eq!(d["collected_at_ms"], 7);
        assert_eq!(d["vulkan"]["status"], "ok");
    }

    #[test]
    fn extensions_are_sorted_and_deduplicated() {
        assert_eq!(
            split_extensions("  EGL_KHR_b EGL_KHR_a\nEGL_KHR_b  "),
            vec!["EGL_KHR_a", "EGL_KHR_b"]
        );
        assert!(split_extensions("").is_empty());
    }

    #[test]
    fn unknown_flag_bits_are_kept_as_hex() {
        let names = [(0x1, "a"), (0x4, "c")];
        assert_eq!(
            flag_names(0x1 | 0x2 | 0x4 | 0x100, &names),
            vec!["a", "0x2", "c", "0x100"]
        );
        assert!(flag_names(0, &names).is_empty());
    }
}

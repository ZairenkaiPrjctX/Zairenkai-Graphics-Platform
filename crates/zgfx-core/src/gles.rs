// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! The OpenGL ES/EGL report (`kind: "gles"`). Constants are the Khronos header values
//! (EGL/egl.h, GLES2/gl2.h, GLES3/gl3.h, GLES3/gl31.h).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::exit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlesStatus {
    Ok,
    /// libEGL could not be opened.
    EglLibraryMissing,
    /// libGLESv2 could not be opened.
    GlesLibraryMissing,
    /// A library opened but lacks a required entry point.
    LibraryIncomplete,
    /// `eglGetDisplay(EGL_DEFAULT_DISPLAY)` returned `EGL_NO_DISPLAY`.
    NoDisplay,
    /// `eglInitialize` failed; `result` names the EGL error.
    InitializeFailed,
    /// No config renders OpenGL ES 2 or 3 to a pbuffer.
    NoConfig,
    SurfaceFailed,
    ContextFailed,
    MakeCurrentFailed,
    Timeout,
}

impl GlesStatus {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Ok => exit::OK,
            Self::EglLibraryMissing
            | Self::GlesLibraryMissing
            | Self::NoDisplay
            | Self::NoConfig => exit::UNSUPPORTED,
            _ => exit::FAILED,
        }
    }
}

/// `eglGetError` codes.
pub fn egl_error_name(code: i32) -> String {
    match code {
        0x3000 => "EGL_SUCCESS",
        0x3001 => "EGL_NOT_INITIALIZED",
        0x3002 => "EGL_BAD_ACCESS",
        0x3003 => "EGL_BAD_ALLOC",
        0x3004 => "EGL_BAD_ATTRIBUTE",
        0x3005 => "EGL_BAD_CONFIG",
        0x3006 => "EGL_BAD_CONTEXT",
        0x3007 => "EGL_BAD_CURRENT_SURFACE",
        0x3008 => "EGL_BAD_DISPLAY",
        0x3009 => "EGL_BAD_MATCH",
        0x300A => "EGL_BAD_NATIVE_PIXMAP",
        0x300B => "EGL_BAD_NATIVE_WINDOW",
        0x300C => "EGL_BAD_PARAMETER",
        0x300D => "EGL_BAD_SURFACE",
        0x300E => "EGL_CONTEXT_LOST",
        other => return format!("{other:#x}"),
    }
    .to_owned()
}

/// The OpenGL ES version in a `GL_VERSION` string, which the ES 2.0+ specifications define as
/// `"OpenGL ES N.M <vendor-specific information>"`.
pub fn parse_es_version(gl_version: &str) -> Option<(u32, u32)> {
    let rest = gl_version.strip_prefix("OpenGL ES ")?;
    let num = rest.split_ascii_whitespace().next()?;
    let (major, minor) = num.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

pub const GL_VENDOR: u32 = 0x1F00;
pub const GL_RENDERER: u32 = 0x1F01;
pub const GL_VERSION: u32 = 0x1F02;
pub const GL_EXTENSIONS: u32 = 0x1F03;
pub const GL_SHADING_LANGUAGE_VERSION: u32 = 0x8B8C;
pub const GL_NUM_EXTENSIONS: u32 = 0x821D;
pub const GL_NO_ERROR: u32 = 0;

/// A queried GL limit: report name, GL enum, the ES version that defines it, and how many
/// integers `glGetIntegerv` writes.
pub struct GlLimit {
    pub name: &'static str,
    pub pname: u32,
    pub since: (u32, u32),
    pub count: usize,
}

const fn limit(name: &'static str, pname: u32, since: (u32, u32), count: usize) -> GlLimit {
    GlLimit {
        name,
        pname,
        since,
        count,
    }
}

pub const GL_LIMITS: &[GlLimit] = &[
    limit("max_texture_size", 0x0D33, (2, 0), 1),
    limit("max_cube_map_texture_size", 0x851C, (2, 0), 1),
    limit("max_renderbuffer_size", 0x84E8, (2, 0), 1),
    limit("max_viewport_dims", 0x0D3A, (2, 0), 2),
    limit("max_vertex_attribs", 0x8869, (2, 0), 1),
    limit("max_vertex_uniform_vectors", 0x8DFB, (2, 0), 1),
    limit("max_fragment_uniform_vectors", 0x8DFD, (2, 0), 1),
    limit("max_texture_image_units", 0x8872, (2, 0), 1),
    limit("max_combined_texture_image_units", 0x8B4D, (2, 0), 1),
    limit("max_3d_texture_size", 0x8073, (3, 0), 1),
    limit("max_array_texture_layers", 0x88FF, (3, 0), 1),
    limit("max_samples", 0x8D57, (3, 0), 1),
    limit("max_color_attachments", 0x8CDF, (3, 0), 1),
    limit("max_draw_buffers", 0x8824, (3, 0), 1),
    limit("max_compute_work_group_invocations", 0x90EB, (3, 1), 1),
    limit("max_compute_shared_memory_size", 0x8262, (3, 1), 1),
];

/// The limits defined at an ES version; the others are not queried (a GL error is not a value).
pub fn limits_for(version: (u32, u32)) -> impl Iterator<Item = &'static GlLimit> {
    GL_LIMITS.iter().filter(move |l| version >= l.since)
}

#[derive(Debug, Clone, Serialize)]
pub struct EglInfo {
    pub major: i32,
    pub minor: i32,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub client_apis: Option<String>,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextInfo {
    /// `EGL_CONTEXT_CLIENT_VERSION` of the context that was created (3, else 2).
    pub client_version: i32,
    pub surface: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlInfo {
    pub vendor: Option<String>,
    pub renderer: Option<String>,
    pub version: Option<String>,
    pub shading_language_version: Option<String>,
    /// Parsed from `version`; null when the string does not follow the ES format.
    pub es_version: Option<String>,
    pub extensions: Vec<String>,
    /// Limits defined at `es_version`; a limit the driver refused (GL error) is absent and
    /// named in `limit_errors`.
    pub limits: BTreeMap<&'static str, Value>,
    pub limit_errors: BTreeMap<&'static str, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlesReport {
    pub status: GlesStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The EGL error name of the failing call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub egl_library: String,
    pub gles_library: String,
    pub egl: Option<EglInfo>,
    pub context: Option<ContextInfo>,
    pub gl: Option<GlInfo>,
}

impl GlesReport {
    pub fn new(egl_library: impl Into<String>, gles_library: impl Into<String>) -> Self {
        Self {
            status: GlesStatus::Ok,
            detail: None,
            result: None,
            egl_library: egl_library.into(),
            gles_library: gles_library.into(),
            egl: None,
            context: None,
            gl: None,
        }
    }

    pub fn fail(
        mut self,
        status: GlesStatus,
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
    fn es_versions_parse_from_the_specified_format_only() {
        assert_eq!(parse_es_version("OpenGL ES 3.2 Mesa 25.2.8"), Some((3, 2)));
        assert_eq!(
            parse_es_version("OpenGL ES 3.2 V@0615.0 (GIT@e9a1b8f, Ib1bf7c9a2c) (Date:04/17/23)"),
            Some((3, 2))
        );
        assert_eq!(parse_es_version("OpenGL ES 2.0"), Some((2, 0)));
        assert_eq!(parse_es_version("OpenGL ES-CM 1.1"), None);
        assert_eq!(parse_es_version("4.6 (Core Profile) Mesa"), None);
        assert_eq!(parse_es_version(""), None);
    }

    #[test]
    fn limits_follow_the_es_version() {
        let es2: Vec<_> = limits_for((2, 0)).map(|l| l.name).collect();
        assert!(es2.contains(&"max_texture_size") && !es2.contains(&"max_samples"));
        let es30: Vec<_> = limits_for((3, 0)).map(|l| l.name).collect();
        assert!(
            es30.contains(&"max_samples") && !es30.contains(&"max_compute_work_group_invocations")
        );
        assert_eq!(limits_for((3, 2)).count(), GL_LIMITS.len());
    }

    #[test]
    fn egl_errors_have_names() {
        assert_eq!(egl_error_name(0x3008), "EGL_BAD_DISPLAY");
        assert_eq!(egl_error_name(0x3001), "EGL_NOT_INITIALIZED");
        assert_eq!(egl_error_name(0x4000), "0x4000");
    }

    #[test]
    fn statuses_map_to_exit_codes() {
        assert_eq!(
            serde_json::to_value(GlesStatus::EglLibraryMissing).unwrap(),
            "egl_library_missing"
        );
        assert_eq!(GlesStatus::NoConfig.exit_code(), 5);
        assert_eq!(GlesStatus::ContextFailed.exit_code(), 1);
    }
}

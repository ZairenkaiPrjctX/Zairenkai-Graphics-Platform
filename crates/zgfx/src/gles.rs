// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! OpenGL ES/EGL probe. Opens libEGL and libGLESv2 at run time, initialises the default display,
//! creates a 1x1 pbuffer and an ES 3 (else ES 2) context owned by this process, makes it current
//! on the calling thread, reads strings and limits, then releases everything on the same thread.
//! Nothing is drawn and no global state is changed.

use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

use libloading::Library;
use serde_json::json;
use zgfx_core::gles::{
    egl_error_name, limits_for, parse_es_version, ContextInfo, EglInfo, GlInfo, GlesReport,
    GlesStatus, GL_EXTENSIONS, GL_NO_ERROR, GL_NUM_EXTENSIONS, GL_RENDERER,
    GL_SHADING_LANGUAGE_VERSION, GL_VENDOR, GL_VERSION,
};
use zgfx_core::split_extensions;

pub const DEFAULT_EGL_LIBRARY: &str = if cfg!(target_os = "android") {
    "libEGL.so"
} else {
    "libEGL.so.1"
};
pub const DEFAULT_GLES_LIBRARY: &str = if cfg!(target_os = "android") {
    "libGLESv2.so"
} else {
    "libGLESv2.so.2"
};

type EglDisplay = *mut c_void;
type EglConfig = *mut c_void;
type EglContext = *mut c_void;
type EglSurface = *mut c_void;
type EglInt = i32;
type EglBoolean = u32;

const EGL_FALSE: EglBoolean = 0;
const EGL_VENDOR: EglInt = 0x3053;
const EGL_VERSION: EglInt = 0x3054;
const EGL_EXTENSIONS: EglInt = 0x3055;
const EGL_CLIENT_APIS: EglInt = 0x308D;
const EGL_SURFACE_TYPE: EglInt = 0x3033;
const EGL_PBUFFER_BIT: EglInt = 0x0001;
const EGL_RENDERABLE_TYPE: EglInt = 0x3040;
const EGL_OPENGL_ES2_BIT: EglInt = 0x0004;
const EGL_OPENGL_ES3_BIT: EglInt = 0x0040;
const EGL_NONE: EglInt = 0x3038;
const EGL_WIDTH: EglInt = 0x3057;
const EGL_HEIGHT: EglInt = 0x3056;
const EGL_CONTEXT_CLIENT_VERSION: EglInt = 0x3098;
const EGL_OPENGL_ES_API: u32 = 0x30A0;

struct Egl {
    get_display: unsafe extern "C" fn(*mut c_void) -> EglDisplay,
    initialize: unsafe extern "C" fn(EglDisplay, *mut EglInt, *mut EglInt) -> EglBoolean,
    query_string: unsafe extern "C" fn(EglDisplay, EglInt) -> *const c_char,
    choose_config: unsafe extern "C" fn(
        EglDisplay,
        *const EglInt,
        *mut EglConfig,
        EglInt,
        *mut EglInt,
    ) -> EglBoolean,
    bind_api: unsafe extern "C" fn(u32) -> EglBoolean,
    create_pbuffer_surface:
        unsafe extern "C" fn(EglDisplay, EglConfig, *const EglInt) -> EglSurface,
    create_context:
        unsafe extern "C" fn(EglDisplay, EglConfig, EglContext, *const EglInt) -> EglContext,
    make_current:
        unsafe extern "C" fn(EglDisplay, EglSurface, EglSurface, EglContext) -> EglBoolean,
    destroy_context: unsafe extern "C" fn(EglDisplay, EglContext) -> EglBoolean,
    destroy_surface: unsafe extern "C" fn(EglDisplay, EglSurface) -> EglBoolean,
    terminate: unsafe extern "C" fn(EglDisplay) -> EglBoolean,
    release_thread: unsafe extern "C" fn() -> EglBoolean,
    get_error: unsafe extern "C" fn() -> EglInt,
}

struct Gl {
    get_string: unsafe extern "C" fn(u32) -> *const u8,
    /// ES 3.0; absent from older libraries, then `GL_EXTENSIONS` is read as one string.
    get_stringi: Option<unsafe extern "C" fn(u32, u32) -> *const u8>,
    get_integerv: unsafe extern "C" fn(u32, *mut i32),
    get_error: unsafe extern "C" fn() -> u32,
}

/// Resolve a symbol to a plain function pointer. The caller keeps the library loaded for as
/// long as the pointer is used.
unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T, String> {
    let c = format!("{name}\0");
    lib.get::<T>(c.as_bytes())
        .map(|s| *s)
        .map_err(|e| format!("{name}: {e}"))
}

impl Egl {
    unsafe fn load(lib: &Library) -> Result<Self, String> {
        Ok(Self {
            get_display: sym(lib, "eglGetDisplay")?,
            initialize: sym(lib, "eglInitialize")?,
            query_string: sym(lib, "eglQueryString")?,
            choose_config: sym(lib, "eglChooseConfig")?,
            bind_api: sym(lib, "eglBindAPI")?,
            create_pbuffer_surface: sym(lib, "eglCreatePbufferSurface")?,
            create_context: sym(lib, "eglCreateContext")?,
            make_current: sym(lib, "eglMakeCurrent")?,
            destroy_context: sym(lib, "eglDestroyContext")?,
            destroy_surface: sym(lib, "eglDestroySurface")?,
            terminate: sym(lib, "eglTerminate")?,
            release_thread: sym(lib, "eglReleaseThread")?,
            get_error: sym(lib, "eglGetError")?,
        })
    }

    fn error(&self) -> Option<String> {
        // SAFETY: eglGetError has no preconditions.
        Some(egl_error_name(unsafe { (self.get_error)() }))
    }
}

impl Gl {
    unsafe fn load(lib: &Library) -> Result<Self, String> {
        Ok(Self {
            get_string: sym(lib, "glGetString")?,
            get_stringi: sym(lib, "glGetStringi").ok(),
            get_integerv: sym(lib, "glGetIntegerv")?,
            get_error: sym(lib, "glGetError")?,
        })
    }
}

unsafe fn owned(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| CStr::from_ptr(p).to_string_lossy().into_owned())
}

/// EGL objects created so far, released in reverse order on every path out of the probe.
struct Session<'a> {
    egl: &'a Egl,
    display: EglDisplay,
    surface: EglSurface,
    context: EglContext,
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        // SAFETY: each handle is either null (never created) or was created on this display by
        // this thread; releasing in reverse order is what EGL requires.
        unsafe {
            (self.egl.make_current)(
                self.display,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if !self.context.is_null() {
                (self.egl.destroy_context)(self.display, self.context);
            }
            if !self.surface.is_null() {
                (self.egl.destroy_surface)(self.display, self.surface);
            }
            (self.egl.terminate)(self.display);
            (self.egl.release_thread)();
        }
    }
}

pub fn probe(egl_library: &str, gles_library: &str) -> GlesReport {
    let report = GlesReport::new(egl_library, gles_library);
    // SAFETY: opening the system graphics libraries runs their initialisers; this process exists
    // only to probe them.
    let egl_lib = match unsafe { Library::new(egl_library) } {
        Ok(l) => l,
        Err(e) => {
            return report.fail(
                GlesStatus::EglLibraryMissing,
                None,
                format!("cannot open {egl_library}: {e}"),
            )
        }
    };
    let gles_lib = match unsafe { Library::new(gles_library) } {
        Ok(l) => l,
        Err(e) => {
            return report.fail(
                GlesStatus::GlesLibraryMissing,
                None,
                format!("cannot open {gles_library}: {e}"),
            )
        }
    };
    // SAFETY: the libraries stay loaded until this function returns, after every call.
    let (egl, gl) = match unsafe { (Egl::load(&egl_lib), Gl::load(&gles_lib)) } {
        (Ok(e), Ok(g)) => (e, g),
        (Err(e), _) | (_, Err(e)) => return report.fail(GlesStatus::LibraryIncomplete, None, e),
    };
    let r = run(&egl, &gl, report);
    drop((gles_lib, egl_lib));
    r
}

fn run(egl: &Egl, gl: &Gl, mut report: GlesReport) -> GlesReport {
    // SAFETY (whole function): EGL and GL calls with valid handles and attribute lists that end
    // with EGL_NONE, all on this thread; the Session releases what was created.
    unsafe {
        let display = (egl.get_display)(ptr::null_mut()); // EGL_DEFAULT_DISPLAY
        if display.is_null() {
            return report.fail(
                GlesStatus::NoDisplay,
                egl.error(),
                "eglGetDisplay(EGL_DEFAULT_DISPLAY) returned EGL_NO_DISPLAY",
            );
        }
        let (mut major, mut minor) = (0, 0);
        if (egl.initialize)(display, &mut major, &mut minor) == EGL_FALSE {
            return report.fail(GlesStatus::InitializeFailed, egl.error(), "eglInitialize");
        }
        let mut session = Session {
            egl,
            display,
            surface: ptr::null_mut(),
            context: ptr::null_mut(),
        };
        report.egl = Some(EglInfo {
            major,
            minor,
            vendor: owned((egl.query_string)(display, EGL_VENDOR)),
            version: owned((egl.query_string)(display, EGL_VERSION)),
            client_apis: owned((egl.query_string)(display, EGL_CLIENT_APIS)),
            extensions: owned((egl.query_string)(display, EGL_EXTENSIONS))
                .map(|s| split_extensions(&s))
                .unwrap_or_default(),
        });
        if (egl.bind_api)(EGL_OPENGL_ES_API) == EGL_FALSE {
            return report.fail(
                GlesStatus::ContextFailed,
                egl.error(),
                "eglBindAPI(EGL_OPENGL_ES_API)",
            );
        }

        // An ES 3 config if there is one, else ES 2; both must render to a pbuffer.
        let mut chosen = None;
        for (bit, version) in [(EGL_OPENGL_ES3_BIT, 3), (EGL_OPENGL_ES2_BIT, 2)] {
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RENDERABLE_TYPE,
                bit,
                EGL_NONE,
            ];
            let (mut config, mut count) = (ptr::null_mut(), 0);
            if (egl.choose_config)(display, attrs.as_ptr(), &mut config, 1, &mut count) != EGL_FALSE
                && count > 0
            {
                chosen = Some((config, version));
                break;
            }
        }
        let Some((config, version)) = chosen else {
            return report.fail(
                GlesStatus::NoConfig,
                egl.error(),
                "no EGL config renders OpenGL ES 2 or 3 to a pbuffer",
            );
        };
        let pbuffer = [EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE];
        session.surface = (egl.create_pbuffer_surface)(display, config, pbuffer.as_ptr());
        if session.surface.is_null() {
            return report.fail(
                GlesStatus::SurfaceFailed,
                egl.error(),
                "eglCreatePbufferSurface(1x1)",
            );
        }
        for v in (2..=version).rev() {
            let attrs = [EGL_CONTEXT_CLIENT_VERSION, v, EGL_NONE];
            session.context =
                (egl.create_context)(display, config, ptr::null_mut(), attrs.as_ptr());
            if !session.context.is_null() {
                report.context = Some(ContextInfo {
                    client_version: v,
                    surface: "pbuffer",
                });
                break;
            }
        }
        if session.context.is_null() {
            return report.fail(
                GlesStatus::ContextFailed,
                egl.error(),
                format!("eglCreateContext (client version {version} and lower)"),
            );
        }
        if (egl.make_current)(display, session.surface, session.surface, session.context)
            == EGL_FALSE
        {
            return report.fail(GlesStatus::MakeCurrentFailed, egl.error(), "eglMakeCurrent");
        }
        let client = report.context.as_ref().map_or(2, |c| c.client_version);
        report.gl = Some(gl_info(gl, client as u32));
        drop(session);
        report
    }
}

/// Strings, extensions and limits of the current context.
unsafe fn gl_info(gl: &Gl, client_version: u32) -> GlInfo {
    let s = |name| owned((gl.get_string)(name).cast());
    let version = s(GL_VERSION);
    let es = version.as_deref().and_then(parse_es_version);
    let effective = es.unwrap_or((client_version, 0));
    let extensions = match gl.get_stringi {
        Some(get_stringi) if effective.0 >= 3 => {
            let mut n = 0;
            (gl.get_integerv)(GL_NUM_EXTENSIONS, &mut n);
            let mut v: Vec<String> = (0..n.max(0) as u32)
                .filter_map(|i| owned(get_stringi(GL_EXTENSIONS, i).cast()))
                .collect();
            v.sort();
            v.dedup();
            v
        }
        _ => s(GL_EXTENSIONS)
            .map(|e| split_extensions(&e))
            .unwrap_or_default(),
    };
    let mut limits = BTreeMap::new();
    let mut limit_errors = BTreeMap::new();
    for l in limits_for(effective) {
        // Clear errors left by earlier calls so the check below belongs to this query.
        for _ in 0..16 {
            if (gl.get_error)() == GL_NO_ERROR {
                break;
            }
        }
        let mut out = [0i32; 4];
        (gl.get_integerv)(l.pname, out.as_mut_ptr());
        match (gl.get_error)() {
            GL_NO_ERROR if l.count == 1 => {
                limits.insert(l.name, json!(out[0]));
            }
            GL_NO_ERROR => {
                limits.insert(l.name, json!(out[..l.count]));
            }
            e => {
                limit_errors.insert(l.name, format!("GL error {e:#06x}"));
            }
        }
    }
    GlInfo {
        vendor: s(GL_VENDOR),
        renderer: s(GL_RENDERER),
        version,
        shading_language_version: s(GL_SHADING_LANGUAGE_VERSION),
        es_version: es.map(|(a, b)| format!("{a}.{b}")),
        extensions,
        limits,
        limit_errors,
    }
}

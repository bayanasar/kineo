use std::{collections::BTreeMap, ptr, slice, str};

use crate::guarded;

use kineto_project::manifest::{
    CanonicalProject, PROJECT_FORMAT_VERSION, ProjectDefaults, ProjectManifest,
    ProjectManifestError, ProjectSource, ProjectStoreError, ProjectWorkflow,
};

const PROJECT_OK: i32 = 0;
const PROJECT_ERR_INVALID_ARGUMENT: i32 = 100;
const PROJECT_ERR_INVALID_UTF8: i32 = 101;
const PROJECT_ERR_IO: i32 = 102;
const PROJECT_ERR_ALREADY_EXISTS: i32 = 103;
const PROJECT_ERR_INVALID_MANIFEST: i32 = 104;
const PROJECT_ERR_UNSUPPORTED_FORMAT: i32 = 105;
const PROJECT_ERR_BUFFER_TOO_SMALL: i32 = 106;
const PROJECT_ERR_PANIC: i32 = 199;

/// Opaque canonical-project handle. Dart never dereferences this allocation.
pub struct KinetoProjectSession {
    project: CanonicalProject,
}

/// Open and validate an existing canonical Kineto project.
///
/// `path_ptr` is borrowed only for the duration of this call. On success the
/// resulting session owns all state it needs; the caller may immediately free
/// the input buffer.
///
/// # Safety
///
/// `out_session` must point to writable storage for one pointer. When
/// `path_len` is non-zero, `path_ptr` must point to at least `path_len` readable
/// bytes. The byte sequence must be UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_open(
    path_ptr: *const u8,
    path_len: u64,
    out_session: *mut *mut KinetoProjectSession,
) -> i32 {
    guarded(PROJECT_ERR_PANIC, || {
        if out_session.is_null() {
            return PROJECT_ERR_INVALID_ARGUMENT;
        }
        // SAFETY: caller guarantees writable storage for one pointer.
        unsafe { ptr::write(out_session, ptr::null_mut()) };

        let path = match unsafe { utf8_owned(path_ptr, path_len) } {
            Ok(path) if !path.is_empty() => path,
            Ok(_) => return PROJECT_ERR_INVALID_ARGUMENT,
            Err(code) => return code,
        };

        match CanonicalProject::open(&path) {
            Ok(project) => {
                let session = Box::into_raw(Box::new(KinetoProjectSession { project }));
                // SAFETY: caller guarantees writable storage for one pointer.
                unsafe { ptr::write(out_session, session) };
                PROJECT_OK
            }
            Err(error) => project_error_code(&error),
        }
    })
}

/// Create a new canonical UTF-8 text-source project and return it opened.
///
/// All input buffers are borrowed only for this call and all must be valid
/// UTF-8. `source_ptr` is written to `source/story.txt`. Binary document ingest
/// such as PDF/EPUB is intentionally a separate future import boundary rather
/// than being smuggled through this text constructor.
///
/// # Safety
///
/// `out_session` must point to writable storage for one pointer. Each non-empty
/// input buffer must be valid for reads of its declared length for the duration
/// of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_create_text(
    path_ptr: *const u8,
    path_len: u64,
    project_id_ptr: *const u8,
    project_id_len: u64,
    title_ptr: *const u8,
    title_len: u64,
    created_at_ptr: *const u8,
    created_at_len: u64,
    language_ptr: *const u8,
    language_len: u64,
    source_ptr: *const u8,
    source_len: u64,
    out_session: *mut *mut KinetoProjectSession,
) -> i32 {
    guarded(PROJECT_ERR_PANIC, || {
        if out_session.is_null() {
            return PROJECT_ERR_INVALID_ARGUMENT;
        }
        // SAFETY: caller guarantees writable storage for one pointer.
        unsafe { ptr::write(out_session, ptr::null_mut()) };

        let path = match unsafe { utf8_owned(path_ptr, path_len) } {
            Ok(path) if !path.is_empty() => path,
            Ok(_) => return PROJECT_ERR_INVALID_ARGUMENT,
            Err(code) => return code,
        };
        let project_id = match unsafe { utf8_owned(project_id_ptr, project_id_len) } {
            Ok(value) => value,
            Err(code) => return code,
        };
        let title = match unsafe { utf8_owned(title_ptr, title_len) } {
            Ok(value) => value,
            Err(code) => return code,
        };
        let created_at = match unsafe { utf8_owned(created_at_ptr, created_at_len) } {
            Ok(value) => value,
            Err(code) => return code,
        };
        let language = match unsafe { utf8_owned(language_ptr, language_len) } {
            Ok(value) => value,
            Err(code) => return code,
        };

        let source_len = match usize::try_from(source_len) {
            Ok(length) => length,
            Err(_) => return PROJECT_ERR_INVALID_ARGUMENT,
        };
        if source_len != 0 && source_ptr.is_null() {
            return PROJECT_ERR_INVALID_ARGUMENT;
        }
        let source: &[u8] = if source_len == 0 {
            &[]
        } else {
            // SAFETY: caller guarantees this buffer remains readable for the call.
            unsafe { slice::from_raw_parts(source_ptr, source_len) }
        };
        if str::from_utf8(source).is_err() {
            return PROJECT_ERR_INVALID_UTF8;
        }

        let manifest = ProjectManifest {
            format_version: PROJECT_FORMAT_VERSION,
            project_id,
            title,
            created_at,
            source: ProjectSource {
                kind: "text".to_owned(),
                path: "source/story.txt".to_owned(),
                extra: BTreeMap::new(),
            },
            defaults: ProjectDefaults {
                language,
                extra: BTreeMap::new(),
            },
            workflow: ProjectWorkflow {
                recipe_id: "default-film".to_owned(),
                recipe_version: 1,
                extra: BTreeMap::new(),
            },
            extra: BTreeMap::new(),
        };

        match CanonicalProject::create(&path, manifest, source) {
            Ok(project) => {
                let session = Box::into_raw(Box::new(KinetoProjectSession { project }));
                // SAFETY: caller guarantees writable storage for one pointer.
                unsafe { ptr::write(out_session, session) };
                PROJECT_OK
            }
            Err(error) => project_error_code(&error),
        }
    })
}

/// # Safety
///
/// `session` must be null or a live pointer returned exactly once by
/// [`kineto_project_open`] or [`kineto_project_create_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_destroy(session: *mut KinetoProjectSession) {
    if session.is_null() {
        return;
    }
    guarded((), || {
        // SAFETY: the caller contract requires ownership of this allocation.
        unsafe { drop(Box::from_raw(session)) };
    });
}

/// # Safety
///
/// `session` must be a live project-session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_format_version(
    session: *const KinetoProjectSession,
) -> u32 {
    guarded(0, || {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return 0;
        };
        session.project.manifest().format_version
    })
}

/// Return whether an opened project is read-only because it uses an older
/// format that requires explicit migration before mutation.
///
/// # Safety
///
/// `session` must be a live project-session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_is_read_only(session: *const KinetoProjectSession) -> u8 {
    guarded(1, || {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return 1;
        };
        u8::from(session.project.is_read_only())
    })
}

/// Copy the UTF-8 project ID into caller-owned memory.
///
/// `out_len` is always set to the required byte length on a valid session. A
/// null/zero-capacity buffer may be used to query that length. If `out_cap` is
/// too small, returns `PROJECT_ERR_BUFFER_TOO_SMALL` and writes nothing.
///
/// # Safety
///
/// `session` must be live. `out_len` must point to writable `u64` storage. If
/// `out_cap` is non-zero, `out_buf` must be writable for at least `out_cap`
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_id_copy(
    session: *const KinetoProjectSession,
    out_buf: *mut u8,
    out_cap: u64,
    out_len: *mut u64,
) -> i32 {
    guarded(PROJECT_ERR_PANIC, || {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return PROJECT_ERR_INVALID_ARGUMENT;
        };
        unsafe {
            copy_utf8(
                session.project.manifest().project_id.as_bytes(),
                out_buf,
                out_cap,
                out_len,
            )
        }
    })
}

/// Copy the UTF-8 project title into caller-owned memory.
///
/// Same sizing contract as [`kineto_project_id_copy`].
///
/// # Safety
///
/// `session` must be live. `out_len` must point to writable `u64` storage. If
/// `out_cap` is non-zero, `out_buf` must be writable for at least `out_cap`
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_project_title_copy(
    session: *const KinetoProjectSession,
    out_buf: *mut u8,
    out_cap: u64,
    out_len: *mut u64,
) -> i32 {
    guarded(PROJECT_ERR_PANIC, || {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return PROJECT_ERR_INVALID_ARGUMENT;
        };
        unsafe {
            copy_utf8(
                session.project.manifest().title.as_bytes(),
                out_buf,
                out_cap,
                out_len,
            )
        }
    })
}

unsafe fn copy_utf8(bytes: &[u8], out_buf: *mut u8, out_cap: u64, out_len: *mut u64) -> i32 {
    if out_len.is_null() {
        return PROJECT_ERR_INVALID_ARGUMENT;
    }
    let required = match u64::try_from(bytes.len()) {
        Ok(length) => length,
        Err(_) => return PROJECT_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: caller guarantees writable storage for one u64.
    unsafe { ptr::write(out_len, required) };

    if out_cap < required {
        return PROJECT_ERR_BUFFER_TOO_SMALL;
    }
    if required == 0 {
        return PROJECT_OK;
    }
    if out_buf.is_null() {
        return PROJECT_ERR_INVALID_ARGUMENT;
    }

    // SAFETY: caller guarantees `out_buf` is writable for `out_cap` bytes, and
    // the capacity check above proves the destination can hold `bytes`.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf, bytes.len()) };
    PROJECT_OK
}

unsafe fn utf8_owned(ptr: *const u8, len: u64) -> Result<String, i32> {
    let len = usize::try_from(len).map_err(|_| PROJECT_ERR_INVALID_ARGUMENT)?;
    if len == 0 {
        return Ok(String::new());
    }
    if ptr.is_null() {
        return Err(PROJECT_ERR_INVALID_ARGUMENT);
    }
    // SAFETY: callers of this private helper inherit the exported FFI contract
    // requiring `ptr` to be readable for `len` bytes during this call.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    str::from_utf8(bytes)
        .map(|value| value.to_owned())
        .map_err(|_| PROJECT_ERR_INVALID_UTF8)
}

fn project_error_code(error: &ProjectStoreError) -> i32 {
    match error {
        ProjectStoreError::AlreadyExists(_) => PROJECT_ERR_ALREADY_EXISTS,
        ProjectStoreError::InvalidTarget(_) => PROJECT_ERR_INVALID_ARGUMENT,
        ProjectStoreError::Utf8(_) => PROJECT_ERR_INVALID_UTF8,
        ProjectStoreError::Manifest(ProjectManifestError::UnsupportedFormatVersion(_)) => {
            PROJECT_ERR_UNSUPPORTED_FORMAT
        }
        ProjectStoreError::Manifest(_) => PROJECT_ERR_INVALID_MANIFEST,
        ProjectStoreError::Fs(_) => PROJECT_ERR_IO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_target() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kineto-project-ffi-{}-{nonce}", std::process::id()))
    }

    unsafe fn create_text(path: &str, out: *mut *mut KinetoProjectSession) -> i32 {
        let id = "ffi_project_001";
        let title = "FFI Project";
        let created_at = "2026-09-10T08:00:00Z";
        let language = "en";
        let source = "A filmmaker waits across the table.\n";
        unsafe {
            kineto_project_create_text(
                path.as_ptr(),
                path.len() as u64,
                id.as_ptr(),
                id.len() as u64,
                title.as_ptr(),
                title.len() as u64,
                created_at.as_ptr(),
                created_at.len() as u64,
                language.as_ptr(),
                language.len() as u64,
                source.as_ptr(),
                source.len() as u64,
                out,
            )
        }
    }

    #[test]
    fn project_session_create_and_open_round_trip() {
        let target = temp_target();
        let path = target.to_string_lossy();
        let title = "FFI Project";
        let mut created = ptr::null_mut();

        let code = unsafe { create_text(&path, &raw mut created) };
        assert_eq!(code, PROJECT_OK);
        assert!(!created.is_null());
        assert_eq!(unsafe { kineto_project_format_version(created) }, 1);
        assert_eq!(unsafe { kineto_project_is_read_only(created) }, 0);
        unsafe { kineto_project_destroy(created) };

        let mut reopened = ptr::null_mut();
        let code =
            unsafe { kineto_project_open(path.as_ptr(), path.len() as u64, &raw mut reopened) };
        assert_eq!(code, PROJECT_OK);
        assert!(!reopened.is_null());

        let mut required = 0_u64;
        assert_eq!(
            unsafe { kineto_project_title_copy(reopened, ptr::null_mut(), 0, &raw mut required) },
            PROJECT_ERR_BUFFER_TOO_SMALL
        );
        assert_eq!(required, title.len() as u64);
        let mut title_bytes = vec![0_u8; required as usize];
        assert_eq!(
            unsafe {
                kineto_project_title_copy(
                    reopened,
                    title_bytes.as_mut_ptr(),
                    title_bytes.len() as u64,
                    &raw mut required,
                )
            },
            PROJECT_OK
        );
        assert_eq!(title_bytes, title.as_bytes());
        unsafe { kineto_project_destroy(reopened) };

        fs::remove_dir_all(target).unwrap();
    }

    #[test]
    fn ffi_rejects_null_out_session() {
        let path = "unused";
        assert_eq!(
            unsafe { kineto_project_open(path.as_ptr(), path.len() as u64, ptr::null_mut()) },
            PROJECT_ERR_INVALID_ARGUMENT
        );
    }

    #[test]
    fn ffi_rejects_invalid_utf8() {
        let invalid = [0xff_u8];
        let mut out = ptr::null_mut();
        assert_eq!(
            unsafe { kineto_project_open(invalid.as_ptr(), invalid.len() as u64, &raw mut out) },
            PROJECT_ERR_INVALID_UTF8
        );
        assert!(out.is_null());
    }

    #[test]
    fn ffi_create_refuses_existing_target() {
        let target = temp_target();
        let path = target.to_string_lossy();
        let mut first = ptr::null_mut();
        assert_eq!(unsafe { create_text(&path, &raw mut first) }, PROJECT_OK);
        unsafe { kineto_project_destroy(first) };

        let mut second = ptr::null_mut();
        assert_eq!(
            unsafe { create_text(&path, &raw mut second) },
            PROJECT_ERR_ALREADY_EXISTS
        );
        assert!(second.is_null());
        fs::remove_dir_all(target).unwrap();
    }

    #[test]
    fn ffi_text_create_rejects_non_utf8_source() {
        let target = temp_target();
        let path = target.to_string_lossy();
        let id = "ffi_project_bad_source";
        let title = "Bad Source";
        let created_at = "2026-09-10T08:00:00Z";
        let language = "en";
        let source = [0xff_u8];
        let mut out = ptr::null_mut();

        let code = unsafe {
            kineto_project_create_text(
                path.as_ptr(),
                path.len() as u64,
                id.as_ptr(),
                id.len() as u64,
                title.as_ptr(),
                title.len() as u64,
                created_at.as_ptr(),
                created_at.len() as u64,
                language.as_ptr(),
                language.len() as u64,
                source.as_ptr(),
                source.len() as u64,
                &raw mut out,
            )
        };
        assert_eq!(code, PROJECT_ERR_INVALID_UTF8);
        assert!(out.is_null());
        assert!(!target.exists());
    }

    #[test]
    fn metadata_copy_reports_required_capacity_without_exposing_rust_memory() {
        let target = temp_target();
        let path = target.to_string_lossy();
        let mut session = ptr::null_mut();
        assert_eq!(unsafe { create_text(&path, &raw mut session) }, PROJECT_OK);

        let mut required = 0_u64;
        let mut too_small = [0_u8; 2];
        assert_eq!(
            unsafe {
                kineto_project_id_copy(
                    session,
                    too_small.as_mut_ptr(),
                    too_small.len() as u64,
                    &raw mut required,
                )
            },
            PROJECT_ERR_BUFFER_TOO_SMALL
        );
        assert!(required > too_small.len() as u64);
        assert_eq!(too_small, [0, 0]);

        unsafe { kineto_project_destroy(session) };
        fs::remove_dir_all(target).unwrap();
    }
}

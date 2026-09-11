use std::path::{Path, PathBuf};

use kineto_core::{DemoError, DemoProject, DemoSnapshot, Engine};

mod project_session;

const KINETO_NATIVE_ABI_VERSION: u32 = 8;
const KINETO_OK: i32 = 0;
const KINETO_ERR_INVALID_HANDLE: i32 = 1;
const KINETO_ERR_IO: i32 = 2;
const KINETO_ERR_INVALID_STATE: i32 = 3;
const KINETO_ERR_NOT_GENERATED: i32 = 4;
const KINETO_ERR_INVALID_CANDIDATE: i32 = 5;
const KINETO_ERR_LOCKED: i32 = 6;
const KINETO_ERR_NO_SELECTION: i32 = 7;
const KINETO_ERR_LIFECYCLE: i32 = 8;
const KINETO_ERR_INVALID_INTENT: i32 = 9;
const KINETO_ERR_INVALID_SHOT: i32 = 10;
const KINETO_ERR_PANIC: i32 = 99;
const DEMO_INTENT_STATE_VERSION: u8 = 1;
const DEMO_SHOT_COUNT: usize = 2;

/// Stop a Rust panic at the C ABI.
///
/// Every exported entry point runs inside this guard. A caught panic is
/// reported best-effort to stderr and converted to the caller-provided fallback
/// so unwinding never crosses into Dart. Logging must itself never panic.
pub(crate) fn guarded<T>(fallback: T, operation: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(value) => value,
        Err(_) => {
            use std::io::Write as _;
            let _ = std::io::stderr()
                .lock()
                .write_all(b"Kineto native panic caught at FFI boundary\n");
            fallback
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DemoIntent {
    #[default]
    Reaction = 1,
    SpatialClarity = 2,
    Intimacy = 3,
    Tension = 4,
}

impl DemoIntent {
    const fn from_wire(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Reaction),
            2 => Some(Self::SpatialClarity),
            3 => Some(Self::Intimacy),
            4 => Some(Self::Tension),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct DemoShot {
    demo: DemoProject,
    intent: DemoIntent,
    intent_path: PathBuf,
}

impl DemoShot {
    fn open(base_path: &Path, index: usize) -> Self {
        let state_path = demo_shot_state_path(base_path, index);
        let intent_path = state_path.with_extension("intent");
        let shot_index = u32::try_from(index).unwrap_or(0);
        let demo = DemoProject::open_for_shot(&state_path, shot_index)
            .unwrap_or_else(|_| DemoProject::empty_for_shot(state_path, shot_index));
        let intent = read_demo_intent(&intent_path).unwrap_or_default();
        Self {
            demo,
            intent,
            intent_path,
        }
    }
}

/// Opaque engine handle. Dart never dereferences this type; it only passes the
/// pointer back to this library.
pub struct KinetoEngine {
    _engine: Engine,
    shots: [DemoShot; DEMO_SHOT_COUNT],
}

#[unsafe(no_mangle)]
pub extern "C" fn kineto_engine_create() -> *mut KinetoEngine {
    guarded(std::ptr::null_mut(), || {
        let base_path = demo_state_path();
        let shots = [DemoShot::open(&base_path, 0), DemoShot::open(&base_path, 1)];
        Box::into_raw(Box::new(KinetoEngine {
            _engine: Engine::new(),
            shots,
        }))
    })
}

/// # Safety
///
/// `engine` must either be null or a pointer returned exactly once by
/// [`kineto_engine_create`] that has not already been destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_engine_destroy(engine: *mut KinetoEngine) {
    if engine.is_null() {
        return;
    }

    guarded((), || {
        // SAFETY: the caller contract requires a live pointer allocated by
        // `kineto_engine_create`, and this function consumes that allocation once.
        unsafe {
            drop(Box::from_raw(engine));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn kineto_engine_abi_version() -> u32 {
    KINETO_NATIVE_ABI_VERSION
}

/// Return one shot's entire demo snapshot as one fixed-width value.
///
/// Layout:
/// - bit 0: candidates generated
/// - bit 1: selection locked
/// - bits 8..15: candidate count
/// - bits 16..23: selected index + 1 (0 means none)
/// - bits 24..31: shot-direction intent
/// - bits 32..47: current generation revision
/// - bits 48..63: number of superseded candidates in this shot lineage
///
/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_state(engine: *mut KinetoEngine, shot_index: u32) -> u64 {
    guarded(0, || {
        let Some(engine) = (unsafe { engine.as_ref() }) else {
            return 0;
        };
        let Some(shot) = demo_shot(engine, shot_index) else {
            return 0;
        };
        encode_demo_snapshot(shot.demo.snapshot(), shot.intent)
    })
}

/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_set_intent(
    engine: *mut KinetoEngine,
    shot_index: u32,
    intent: u32,
) -> i32 {
    guarded(KINETO_ERR_PANIC, || {
        let Some(engine) = (unsafe { engine.as_mut() }) else {
            return KINETO_ERR_INVALID_HANDLE;
        };
        let Some(intent) = DemoIntent::from_wire(intent) else {
            return KINETO_ERR_INVALID_INTENT;
        };
        let Some(shot) = demo_shot_mut(engine, shot_index) else {
            return KINETO_ERR_INVALID_SHOT;
        };
        if shot.demo.snapshot().locked {
            return KINETO_ERR_LOCKED;
        }
        if shot.intent == intent {
            return KINETO_OK;
        }

        // Direction is an upstream semantic input. Changing it invalidates only
        // this shot's unlocked candidates and selection. Locked output is never
        // rewritten implicitly.
        if let Err(error) = shot.demo.reset() {
            return result_code(Err(error));
        }
        if write_demo_intent(&shot.intent_path, intent).is_err() {
            return KINETO_ERR_IO;
        }
        shot.intent = intent;
        KINETO_OK
    })
}

/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_generate(engine: *mut KinetoEngine, shot_index: u32) -> i32 {
    guarded(KINETO_ERR_PANIC, || {
        let Some(engine) = (unsafe { engine.as_mut() }) else {
            return KINETO_ERR_INVALID_HANDLE;
        };
        let Some(shot) = demo_shot_mut(engine, shot_index) else {
            return KINETO_ERR_INVALID_SHOT;
        };
        result_code(shot.demo.generate_candidates())
    })
}

/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_select(
    engine: *mut KinetoEngine,
    shot_index: u32,
    candidate_index: u32,
) -> i32 {
    guarded(KINETO_ERR_PANIC, || {
        let Some(engine) = (unsafe { engine.as_mut() }) else {
            return KINETO_ERR_INVALID_HANDLE;
        };
        let Some(shot) = demo_shot_mut(engine, shot_index) else {
            return KINETO_ERR_INVALID_SHOT;
        };
        let Ok(candidate_index) = u8::try_from(candidate_index) else {
            return KINETO_ERR_INVALID_CANDIDATE;
        };
        result_code(shot.demo.select_candidate(candidate_index))
    })
}

/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_lock(engine: *mut KinetoEngine, shot_index: u32) -> i32 {
    guarded(KINETO_ERR_PANIC, || {
        let Some(engine) = (unsafe { engine.as_mut() }) else {
            return KINETO_ERR_INVALID_HANDLE;
        };
        let Some(shot) = demo_shot_mut(engine, shot_index) else {
            return KINETO_ERR_INVALID_SHOT;
        };
        result_code(shot.demo.lock_selection())
    })
}

/// # Safety
/// `engine` must be a live handle returned by [`kineto_engine_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kineto_demo_reset(engine: *mut KinetoEngine, shot_index: u32) -> i32 {
    guarded(KINETO_ERR_PANIC, || {
        let Some(engine) = (unsafe { engine.as_mut() }) else {
            return KINETO_ERR_INVALID_HANDLE;
        };
        let Some(shot) = demo_shot_mut(engine, shot_index) else {
            return KINETO_ERR_INVALID_SHOT;
        };
        if let Err(error) = shot.demo.reset() {
            return result_code(Err(error));
        }
        match std::fs::remove_file(&shot.intent_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return KINETO_ERR_IO,
        }
        shot.intent = DemoIntent::default();
        KINETO_OK
    })
}

fn demo_shot(engine: &KinetoEngine, shot_index: u32) -> Option<&DemoShot> {
    let index = usize::try_from(shot_index).ok()?;
    engine.shots.get(index)
}

fn demo_shot_mut(engine: &mut KinetoEngine, shot_index: u32) -> Option<&mut DemoShot> {
    let index = usize::try_from(shot_index).ok()?;
    engine.shots.get_mut(index)
}

fn encode_demo_snapshot(snapshot: DemoSnapshot, intent: DemoIntent) -> u64 {
    let selected_code = snapshot
        .selected_index
        .map_or(0, |index| u64::from(index) + 1);
    u64::from(snapshot.generated)
        | (u64::from(snapshot.locked) << 1)
        | (u64::from(snapshot.candidate_count) << 8)
        | (selected_code << 16)
        | (u64::from(intent as u8) << 24)
        | (u64::from(snapshot.generation) << 32)
        | (u64::from(snapshot.superseded_count) << 48)
}

fn result_code(result: Result<(), DemoError>) -> i32 {
    match result {
        Ok(()) => KINETO_OK,
        Err(DemoError::Io(_)) => KINETO_ERR_IO,
        Err(DemoError::InvalidState) => KINETO_ERR_INVALID_STATE,
        Err(DemoError::NotGenerated) => KINETO_ERR_NOT_GENERATED,
        Err(DemoError::InvalidCandidate) => KINETO_ERR_INVALID_CANDIDATE,
        Err(DemoError::Locked) => KINETO_ERR_LOCKED,
        Err(DemoError::NoSelection) => KINETO_ERR_NO_SELECTION,
        Err(DemoError::Lifecycle(_) | DemoError::Selection(_)) => KINETO_ERR_LIFECYCLE,
    }
}

fn read_demo_intent(path: &Path) -> Option<DemoIntent> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => return None,
    };
    if bytes.len() != 2 || bytes[0] != DEMO_INTENT_STATE_VERSION {
        return None;
    }
    DemoIntent::from_wire(u32::from(bytes[1]))
}

fn write_demo_intent(path: &Path, intent: DemoIntent) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, [DEMO_INTENT_STATE_VERSION, intent as u8])
}

fn demo_shot_state_path(base_path: &Path, index: usize) -> PathBuf {
    if index == 0 {
        base_path.to_path_buf()
    } else {
        base_path.with_extension(format!("shot-{:03}.state", index + 1))
    }
}

fn demo_state_path() -> PathBuf {
    if let Some(path) = std::env::var_os("KINETO_DEMO_STATE")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "windows")]
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base)
            .join("Kineto")
            .join("demo-project.state");
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Kineto")
            .join("demo-project.state");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(base) = std::env::var_os("XDG_STATE_HOME") {
            return PathBuf::from(base)
                .join("kineto")
                .join("demo-project.state");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("kineto")
                .join("demo-project.state");
        }
    }

    std::env::temp_dir().join("kineto-demo-project.state")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_handle_round_trip_is_minimal_and_typed() {
        let engine = kineto_engine_create();
        assert!(!engine.is_null());
        assert_eq!(kineto_engine_abi_version(), 8);

        // SAFETY: `engine` was created above and has not yet been destroyed.
        unsafe { kineto_engine_destroy(engine) };
    }

    #[test]
    fn packed_demo_snapshot_round_trips_expected_fields() {
        let bits = encode_demo_snapshot(
            DemoSnapshot {
                generated: true,
                selected_index: Some(2),
                locked: true,
                candidate_count: 3,
                generation: 7,
                superseded_count: 18,
            },
            DemoIntent::Tension,
        );
        assert_eq!(bits & 1, 1);
        assert_eq!((bits >> 1) & 1, 1);
        assert_eq!((bits >> 8) & 0xff, 3);
        assert_eq!((bits >> 16) & 0xff, 3);
        assert_eq!((bits >> 24) & 0xff, u64::from(DemoIntent::Tension as u8));
        assert_eq!((bits >> 32) & 0xffff, 7);
        assert_eq!((bits >> 48) & 0xffff, 18);
    }

    #[test]
    fn second_shot_uses_an_independent_sidecar_path() {
        let base = PathBuf::from("demo-project.state");
        assert_eq!(demo_shot_state_path(&base, 0), base);
        assert_eq!(
            demo_shot_state_path(&base, 1),
            PathBuf::from("demo-project.shot-002.state")
        );
    }
}

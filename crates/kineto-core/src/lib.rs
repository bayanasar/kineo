use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use kineto_project::{
    ArtifactId, ArtifactRecord, ArtifactStatus, ContentHash, LifecycleError, SelectionError,
    SelectionManifest,
};

/// Process-local engine state shared by all machine-local Kineto work.
///
/// The Flutter desktop application embeds this engine in-process. Expensive
/// work may still use Rust threads, GPU runtimes, provider clients, ffmpeg, or
/// dedicated child processes, but the UI/native boundary itself is FFI.
#[derive(Debug)]
pub struct Engine {
    // Keep the bootstrap handle non-zero-sized without inventing scheduler or
    // resource policy before real workloads exist.
    _anchor: u8,
}

impl Engine {
    #[must_use]
    pub const fn new() -> Self {
        Self { _anchor: 0 }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

const DEMO_STATE_VERSION: u8 = 2;
const DEMO_CANDIDATE_LABELS: [&str; 3] = ["a", "b", "c"];

/// Small native-only vertical slice used by the first interactive Kineto shell.
///
/// This is deliberately not the final canonical project serializer. It uses the
/// real artifact lifecycle and selection types, while persisting only compact
/// state necessary to prove generation/select/lock/reopen behavior. The production
/// project store will replace this demo persistence with schema-validated TOML/JSON IO.
#[derive(Debug, Clone)]
pub struct DemoProject {
    state_path: PathBuf,
    shot_index: u32,
    generated: bool,
    generation: u16,
    superseded_count: u16,
    candidates: Vec<ArtifactRecord>,
    selection: SelectionManifest,
    locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemoSnapshot {
    pub generated: bool,
    pub selected_index: Option<u8>,
    pub locked: bool,
    pub candidate_count: u8,
    pub generation: u16,
    pub superseded_count: u16,
}

fn derive_shot_index(path: &Path) -> u32 {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return 0;
    };
    if let Some(pos) = name.find("shot-") {
        let digits: String = name[pos + 5..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(num) = digits.parse::<u32>()
            && num > 0
        {
            return num - 1;
        }
    }
    0
}

impl DemoProject {
    #[must_use]
    pub fn empty(state_path: impl Into<PathBuf>) -> Self {
        let state_path = state_path.into();
        let shot_index = derive_shot_index(&state_path);
        Self::empty_for_shot(state_path, shot_index)
    }

    #[must_use]
    pub fn empty_for_shot(state_path: impl Into<PathBuf>, shot_index: u32) -> Self {
        Self {
            state_path: state_path.into(),
            shot_index,
            generated: false,
            generation: 0,
            superseded_count: 0,
            candidates: Vec::new(),
            selection: SelectionManifest::default(),
            locked: false,
        }
    }

    pub fn open(state_path: impl Into<PathBuf>) -> Result<Self, DemoError> {
        let state_path = state_path.into();
        let shot_index = derive_shot_index(&state_path);
        Self::open_for_shot(state_path, shot_index)
    }

    pub fn open_for_shot(
        state_path: impl Into<PathBuf>,
        shot_index: u32,
    ) -> Result<Self, DemoError> {
        let state_path = state_path.into();
        let bytes = match fs::read(&state_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::empty_for_shot(state_path, shot_index));
            }
            Err(error) => return Err(DemoError::Io(error)),
        };

        if !matches!(bytes.len(), 4 | 8) || bytes[1] > 1 || bytes[3] > 1 {
            return Err(DemoError::InvalidState);
        }

        let generated = bytes[1] == 1;
        let selected_code = bytes[2];
        let locked = bytes[3] == 1;
        let (generation, superseded_count) = match (bytes[0], bytes.len()) {
            // Migrate the original four-byte demo state in memory. A subsequent
            // mutation rewrites it using the current compact format.
            (1, 4) => (u16::from(generated), 0),
            (DEMO_STATE_VERSION, 8) => (
                u16::from_le_bytes([bytes[4], bytes[5]]),
                u16::from_le_bytes([bytes[6], bytes[7]]),
            ),
            _ => return Err(DemoError::InvalidState),
        };

        if selected_code > DEMO_CANDIDATE_LABELS.len() as u8
            || (!generated && (selected_code != 0 || locked || generation != 0))
            || (generated && generation == 0)
            || (locked && selected_code == 0)
        {
            return Err(DemoError::InvalidState);
        }

        let mut project = Self::empty_for_shot(state_path, shot_index);
        project.superseded_count = superseded_count;
        if generated {
            project.install_generation(generation)?;
        }
        if selected_code != 0 {
            project.select_in_memory(selected_code - 1)?;
        }
        if locked {
            project.lock_in_memory()?;
        }
        Ok(project)
    }

    #[must_use]
    pub fn shot_index(&self) -> u32 {
        self.shot_index
    }

    #[must_use]
    pub fn candidates(&self) -> &[ArtifactRecord] {
        &self.candidates
    }

    #[must_use]
    pub fn snapshot(&self) -> DemoSnapshot {
        DemoSnapshot {
            generated: self.generated,
            selected_index: self.selected_index(),
            locked: self.locked,
            candidate_count: if self.generated {
                DEMO_CANDIDATE_LABELS.len() as u8
            } else {
                0
            },
            generation: self.generation,
            superseded_count: self.superseded_count,
        }
    }

    pub fn generate_candidates(&mut self) -> Result<(), DemoError> {
        if self.locked {
            return Err(DemoError::Locked);
        }
        let next_generation = self
            .generation
            .checked_add(1)
            .filter(|generation| *generation != 0)
            .ok_or(DemoError::InvalidState)?;
        let previous = self.clone();
        let result = (|| {
            if self.generated {
                self.supersede_current_generation()?;
            }
            self.install_generation(next_generation)?;
            self.persist()
        })();
        if let Err(error) = result {
            *self = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn select_candidate(&mut self, index: u8) -> Result<(), DemoError> {
        if self.locked {
            return Err(DemoError::Locked);
        }
        let previous = self.clone();
        self.select_in_memory(index)?;
        self.persist_or_rollback(previous)
    }

    pub fn lock_selection(&mut self) -> Result<(), DemoError> {
        if self.locked {
            return Ok(());
        }
        let previous = self.clone();
        self.lock_in_memory()?;
        self.persist_or_rollback(previous)
    }

    pub fn reset(&mut self) -> Result<(), DemoError> {
        let previous = self.clone();
        self.generated = false;
        self.generation = 0;
        self.superseded_count = 0;
        self.candidates.clear();
        self.selection = SelectionManifest::default();
        self.locked = false;

        match fs::remove_file(&self.state_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                *self = previous;
                Err(DemoError::Io(error))
            }
        }
    }

    fn supersede_current_generation(&mut self) -> Result<(), DemoError> {
        let mut current = std::mem::take(&mut self.candidates);
        for candidate in &mut current {
            candidate.transition(ArtifactStatus::Superseded)?;
        }
        let superseded = u16::try_from(current.len()).map_err(|_| DemoError::InvalidState)?;
        self.superseded_count = self
            .superseded_count
            .checked_add(superseded)
            .ok_or(DemoError::InvalidState)?;
        self.selection = SelectionManifest::default();
        Ok(())
    }

    fn install_generation(&mut self, generation: u16) -> Result<(), DemoError> {
        if generation == 0 {
            return Err(DemoError::InvalidState);
        }
        self.generated = true;
        self.generation = generation;
        self.locked = false;
        let shot_number = self.shot_index + 1;
        self.candidates = DEMO_CANDIDATE_LABELS
            .iter()
            .enumerate()
            .map(|(index, label)| {
                Ok(ArtifactRecord {
                    artifact_id: ArtifactId::new(format!(
                        "shot_{shot_number:03}_g{generation:04}_candidate_{label}"
                    ))
                    .map_err(|_| DemoError::InvalidState)?,
                    status: ArtifactStatus::Candidate,
                    content_hash: ContentHash::new(format!(
                        "demo:s{shot_number:03}:g{generation}:candidate-{index}"
                    ))
                    .map_err(|_| DemoError::InvalidState)?,
                    input_hash: None,
                    dependencies: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, DemoError>>()?;
        self.selection = SelectionManifest {
            selected_artifact_id: None,
            candidate_artifact_ids: self
                .candidates
                .iter()
                .map(|candidate| candidate.artifact_id.clone())
                .collect(),
        };
        Ok(())
    }

    fn select_in_memory(&mut self, index: u8) -> Result<(), DemoError> {
        if !self.generated {
            return Err(DemoError::NotGenerated);
        }
        let index = usize::from(index);
        if index >= self.candidates.len() {
            return Err(DemoError::InvalidCandidate);
        }

        if let Some(previous_id) = self.selection.selected_artifact_id.as_ref()
            && let Some(previous) = self
                .candidates
                .iter_mut()
                .find(|candidate| &candidate.artifact_id == previous_id)
            && previous.status == ArtifactStatus::Selected
        {
            previous.transition(ArtifactStatus::Candidate)?;
        }

        let selected_id = self.candidates[index].artifact_id.clone();
        self.selection.select(&selected_id)?;
        self.candidates[index].transition(ArtifactStatus::Selected)?;
        Ok(())
    }

    fn lock_in_memory(&mut self) -> Result<(), DemoError> {
        if !self.generated {
            return Err(DemoError::NotGenerated);
        }
        let selected_id = self
            .selection
            .selected_artifact_id
            .as_ref()
            .ok_or(DemoError::NoSelection)?;
        let selected = self
            .candidates
            .iter_mut()
            .find(|candidate| &candidate.artifact_id == selected_id)
            .ok_or(DemoError::InvalidState)?;
        selected.transition(ArtifactStatus::Locked)?;
        self.locked = true;
        Ok(())
    }

    fn selected_index(&self) -> Option<u8> {
        let selected_id = self.selection.selected_artifact_id.as_ref()?;
        self.candidates
            .iter()
            .position(|candidate| &candidate.artifact_id == selected_id)
            .and_then(|index| u8::try_from(index).ok())
    }

    fn persist_or_rollback(&mut self, previous: Self) -> Result<(), DemoError> {
        match self.persist() {
            Ok(()) => Ok(()),
            Err(error) => {
                *self = previous;
                Err(error)
            }
        }
    }

    fn persist(&self) -> Result<(), DemoError> {
        let selected_code = self.selected_index().map_or(0, |index| index + 1);
        let generation = self.generation.to_le_bytes();
        let superseded_count = self.superseded_count.to_le_bytes();
        let bytes = [
            DEMO_STATE_VERSION,
            u8::from(self.generated),
            selected_code,
            u8::from(self.locked),
            generation[0],
            generation[1],
            superseded_count[0],
            superseded_count[1],
        ];

        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&self.state_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum DemoError {
    Io(io::Error),
    InvalidState,
    NotGenerated,
    InvalidCandidate,
    Locked,
    NoSelection,
    Lifecycle(LifecycleError),
    Selection(SelectionError),
}

impl fmt::Display for DemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "demo persistence failed: {error}"),
            Self::InvalidState => formatter.write_str("demo state is invalid"),
            Self::NotGenerated => formatter.write_str("generate demo candidates first"),
            Self::InvalidCandidate => formatter.write_str("demo candidate index is invalid"),
            Self::Locked => formatter.write_str("demo selection is locked"),
            Self::NoSelection => formatter.write_str("select a demo candidate before locking"),
            Self::Lifecycle(error) => error.fmt(formatter),
            Self::Selection(error) => error.fmt(formatter),
        }
    }
}

impl Error for DemoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Lifecycle(error) => Some(error),
            Self::Selection(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for DemoError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<LifecycleError> for DemoError {
    fn from(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}

impl From<SelectionError> for DemoError {
    fn from(error: SelectionError) -> Self {
        Self::Selection(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{process, time::SystemTime};

    fn temp_state_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kineto-{name}-{}-{nonce}.state", process::id()))
    }

    #[test]
    fn engine_constructs_without_starting_background_runtime() {
        let _engine = Engine::new();
    }

    #[test]
    fn demo_selection_can_change_before_lock_and_survives_reopen() {
        let path = temp_state_path("demo-reopen");
        let mut demo = DemoProject::open(&path).unwrap();

        demo.generate_candidates().unwrap();
        demo.select_candidate(0).unwrap();
        demo.select_candidate(2).unwrap();
        demo.lock_selection().unwrap();

        let reopened = DemoProject::open(&path).unwrap();
        assert_eq!(
            reopened.snapshot(),
            DemoSnapshot {
                generated: true,
                selected_index: Some(2),
                locked: true,
                candidate_count: 3,
                generation: 1,
                superseded_count: 0,
            }
        );

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn regenerate_creates_new_candidate_identity_and_counts_superseded_artifacts() {
        let path = temp_state_path("demo-regenerate");
        let mut demo = DemoProject::empty(&path);
        demo.generate_candidates().unwrap();
        demo.select_candidate(1).unwrap();
        let first_ids = demo
            .candidates
            .iter()
            .map(|candidate| candidate.artifact_id.clone())
            .collect::<Vec<_>>();

        demo.generate_candidates().unwrap();
        let second_ids = demo
            .candidates
            .iter()
            .map(|candidate| candidate.artifact_id.clone())
            .collect::<Vec<_>>();

        assert_ne!(first_ids, second_ids);
        assert_eq!(demo.snapshot().generation, 2);
        assert_eq!(demo.snapshot().superseded_count, 3);
        assert_eq!(demo.snapshot().selected_index, None);

        let reopened = DemoProject::open(&path).unwrap();
        assert_eq!(reopened.snapshot().generation, 2);
        assert_eq!(reopened.snapshot().superseded_count, 3);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn legacy_four_byte_state_migrates_in_memory() {
        let path = temp_state_path("demo-legacy");
        fs::write(&path, [1, 1, 2, 0]).unwrap();
        let demo = DemoProject::open(&path).unwrap();
        assert_eq!(demo.snapshot().generation, 1);
        assert_eq!(demo.snapshot().selected_index, Some(1));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn lock_blocks_selection_and_regeneration_until_reset() {
        let path = temp_state_path("demo-lock");
        let mut demo = DemoProject::empty(&path);
        demo.generate_candidates().unwrap();
        demo.select_candidate(1).unwrap();
        demo.lock_selection().unwrap();

        assert!(matches!(demo.select_candidate(0), Err(DemoError::Locked)));
        assert!(matches!(demo.generate_candidates(), Err(DemoError::Locked)));

        demo.reset().unwrap();
        assert_eq!(demo.snapshot().candidate_count, 0);
        assert_eq!(demo.snapshot().generation, 0);
        assert!(!path.exists());
    }

    #[test]
    fn malformed_demo_state_is_rejected() {
        let path = temp_state_path("demo-corrupt");
        fs::write(&path, [99, 1, 1, 0]).unwrap();
        assert!(matches!(
            DemoProject::open(&path),
            Err(DemoError::InvalidState)
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn different_shot_scopes_generate_distinct_artifact_ids() {
        let path1 = temp_state_path("demo-shot-1");
        let path2 = temp_state_path("demo-shot-2");
        let mut shot1 = DemoProject::empty_for_shot(&path1, 0);
        let mut shot2 = DemoProject::empty_for_shot(&path2, 1);

        shot1.generate_candidates().unwrap();
        shot2.generate_candidates().unwrap();

        let shot1_ids: Vec<_> = shot1
            .candidates()
            .iter()
            .map(|c| c.artifact_id.to_string())
            .collect();
        let shot2_ids: Vec<_> = shot2
            .candidates()
            .iter()
            .map(|c| c.artifact_id.to_string())
            .collect();

        assert_eq!(shot1_ids.len(), 3);
        assert_eq!(shot2_ids.len(), 3);
        for id in &shot1_ids {
            assert!(id.starts_with("shot_001_"));
            assert!(!shot2_ids.contains(id));
        }
        for id in &shot2_ids {
            assert!(id.starts_with("shot_002_"));
            assert!(!shot1_ids.contains(id));
        }

        fs::remove_file(path1).unwrap();
        fs::remove_file(path2).unwrap();
    }

    #[test]
    fn open_derives_shot_scope_from_state_path() {
        let path = temp_state_path("demo.shot-002");
        let mut demo = DemoProject::open(&path).unwrap();
        assert_eq!(demo.shot_index(), 1);
        demo.generate_candidates().unwrap();
        for candidate in demo.candidates() {
            assert!(candidate.artifact_id.as_str().starts_with("shot_002_"));
        }
        fs::remove_file(path).unwrap();
    }
}

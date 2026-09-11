# ADR-0004: Canonical Project State and Derived Local Index

- Status: Accepted
- Date: 2026-09-07

## Context

ADR-0002 makes selection, locking, provenance, and artifact dependencies first-class production state. The project-format design also promises that critical project state remains portable and understandable without the application.

If lifecycle state exists only in `.kineto/project.db`, deleting the local index destroys the user's accepted production decisions. Encoding selection in filenames such as `selected.mp4` also makes a state transition rename media and breaks stable references.

## Decision

Durable production truth lives in canonical project files.

This includes at minimum:

- artifact identity and schema version
- lifecycle status
- selected/locked pointers
- provenance and dependencies
- content/input hashes
- generation parameters needed for explanation or reproduction
- continuity/production-memory state

`.kineto/project.db` is derived support state. It may contain indexes, caches, file hashes, search structures, and runtime/in-flight job records, but it is never the only copy of a durable production decision.

Deleting `.kineto/` may discard active runtime bookkeeping, but reopening the remaining project directory must preserve all authored/generated artifacts, selections, locks, provenance, and dependency relationships and must allow the index to be rebuilt.

Selection is represented by a manifest pointer to a stable artifact ID, never by renaming the selected media file.

Example:

```json
{
  "selected_artifact_id": "01J7X...",
  "candidate_artifact_ids": ["01J7X...", "01J7Y..."]
}
```

## Consequences

### Positive

- `.kineto/` is disposable without losing production truth
- selection changes are metadata-only and do not mutate candidate identity
- stable artifact paths can be hashed, referenced, exported, and diffed reliably
- project recovery does not depend on SQLite integrity

### Negative

- canonical manifests contain more metadata
- rebuild logic and round-trip tests are required
- runtime job state has a different durability class from production state and must be documented clearly

## Required tests

- create/select/lock artifacts, delete `.kineto/`, reopen, and verify all durable production state survives
- rebuild `project.db` from canonical project files
- changing a selection does not rename or rewrite candidate media bytes

# ADR-0001: Local-first Flutter UI with Rust Engine

- Status: Accepted
- Date: 2026-09-07

## Context

Kineto needs a cross-platform UI and must also:

- manage large local media files
- invoke local model processes
- call ffmpeg
- use local CPU/GPU resources
- spawn CLI tools
- supervise long-running jobs
- recover from provider/process failures
- support local-first project storage

Putting all orchestration into Flutter/Dart would couple UI lifecycle to machine-local production work.

A cloud backend would violate the local-first product direction.

## Decision

Use:

- Flutter for the application UI
- a bundled local Rust engine for orchestration and machine-local work
- an IPC/RPC boundary between them

The Rust engine is not a cloud backend. It runs on the user's machine.

Desktop is the primary runtime.

A later local-web mode may expose the same Rust engine over localhost to a Flutter Web build.

## Consequences

### Positive

- UI crashes do not need to terminate long jobs
- local subprocess integration is straightforward
- model/provider adapters stay outside Dart
- large-file IO stays outside the UI isolate
- web and desktop UIs can eventually share protocol semantics
- Rust crates can be tested headlessly

### Negative

- IPC protocol must be versioned
- packaging is more complex than a Flutter-only app
- engine lifecycle must be managed per platform
- local-web mode needs explicit security boundaries

## Follow-up

Define a versioned protocol and job/event schema before implementing production workflows.

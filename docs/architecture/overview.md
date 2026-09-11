# Architecture Overview

## High-level architecture

```text
┌──────────────────────────────────────────────┐
│                 Flutter UI                   │
│         WabiSabi presentation layer          │
└──────────────────────┬───────────────────────┘
                       │
                 typed Dart FFI
                       │
┌──────────────────────▼───────────────────────┐
│              Rust application core           │
│                                              │
│  project/artifact state                      │
│  prompt compilation contracts                │
│  provider capability routing                 │
│  paid-job intent/reconciliation               │
│  future workflow/asset/process runtime        │
└───────┬───────────┬───────────┬──────────────┘
        │           │           │
     LLMs       Image/Video     TTS
        │           │           │
 Cloud/local    Cloud/local  Cloud/local
```

The desktop split is a **language/ownership boundary**, not a client/server topology. ADR-0005 supersedes the original separate-process desktop design.

## Ownership

Flutter owns:

- navigation and presentation
- project browser and review screens
- candidate comparison/selection UI
- provider settings and cost-preflight presentation
- lightweight media previews

Rust owns:

- canonical project IO and validation
- artifact identity/lifecycle/dependency semantics
- hashing/indexing/cache management
- provider/model clients and capability routing
- job execution, idempotency, retry and reconciliation
- prompt compilation
- ffmpeg/local model subprocesses
- CPU/GPU-oriented production work

The UI does not own production truth and the Rust core does not own visual design.

## Current repository shape

```text
kineto/
├── apps/
│   └── kineto/
│       ├── lib/                 # Flutter product code
│       ├── hook/build.dart      # Rust Code Asset build hook
│       └── rust/                # thin app-specific FFI shim
├── crates/
│   ├── kineto-core/              # engine composition/runtime root
│   ├── kineto-project/           # artifact identity/lifecycle/dependency model
│   ├── kineto-prompts/           # prompt degradation/compiler contract
│   ├── kineto-jobs/              # paid-job intent/cost/retry policy model
│   └── kineto-providers/         # provider capability/routing model
├── schemas/                     # canonical wire/storage contracts
├── fixtures/projects/           # deterministic golden projects
├── docs/
├── Cargo.lock
├── Cargo.toml
└── rust-toolchain.toml
```

Additional crates appear only when executable, tested subsystem boundaries justify them. Do not add empty crates because a design diagram names a future subsystem.

## Native desktop boundary

Desktop uses Dart FFI and bundled Code Assets:

```text
Flutter
  ↓ direct typed call
opaque native handle / fixed-width ABI
  ↓
kineto-native thin shim
  ↓ normal Rust calls
kineto-core/domain crates
```

There is no desktop JSON-RPC, stdio protocol, localhost server, session token, or engine executable lookup. See [Native Desktop Boundary](native-boundary.md).

Performance rules:

- no JSON/HTTP/stdio serialization for ordinary desktop calls
- no base64 or large media copies through FFI when an asset/path/native buffer suffices
- no duplicate daemon heap without a demonstrated isolation requirement
- no worker pool per feature; execution policy comes from measured workloads
- canonical JSON/TOML is durable storage, not the hot in-memory representation

A future local-web/daemon adapter is separate from desktop FFI and gets its own generated binary transport and security boundary.

## Project/artifact model

Durable production truth is ordinary project state, not `.kineto/project.db`.

Current typed invariants in `kineto-project` include:

- stable artifact IDs
- lifecycle states: draft → candidate → selected → locked → superseded
- selection manifests point to IDs instead of renaming media
- semantic vs cosmetic dependency edges
- computed `current` / `stale` / `incompatible` state from recorded dependency hashes
- explanation data showing which parent/field changed

The committed schemas live under `schemas/`; the minimal golden project under `fixtures/projects/minimal` intentionally carries no `.kineto/` dependency.

## Prompt/provider model

`kineto-prompts` returns a structured compilation result rather than only a string:

- compiled provider input
- compiler version
- applied constraints
- dropped constraints with reasons
- derived degradation severity

Identity-affecting degradation can be blocked by policy.

`kineto-providers` describes real modality capabilities for LLM/image/video/TTS. Router evaluation refuses incompatible changes such as silently turning image-to-video into text-to-video or dropping a required identity reference. Lesser optional capability loss remains visible as a degradation/warning.

## Job model

Long-running generation is job-oriented, but durability does not depend on an immortal process.

```text
persist Prepared intent
        ↓
provider invocation
        ↓
persist provider_job_id immediately when available
        ↓
reconcile unfinished intent on restart
```

`kineto-jobs` currently defines:

- deterministic idempotency key
- input hash
- prepared → invoked → reconciled intent lifecycle
- remote provider job handle
- integer cost estimates (no floating-point money)
- concurrency/backoff/error classification
- explicit paid fallback policy and spend ceiling

Actual persistence/executor/provider networking is intentionally not implemented until the project store and runtime boundary are ready.

## Storage durability classes

### Canonical project files

Human-readable TOML/JSON/Markdown plus media own all durable production decisions: identity, lifecycle state, selections/locks, provenance, dependencies, continuity, and accepted generation metadata.

### `.kineto/` derived/runtime state

A future local SQLite database may contain derived indexes, caches, search data, provider metadata cache, and in-flight job bookkeeping. It must never be the only copy of durable production truth.

Deleting `.kineto/` may lose active runtime bookkeeping; it must not change what the user selected or locked. A real reopen/rebuild test remains required when project IO lands.

## Testing seam

- Rust domain crates are pure/headless and currently need no provider/network runtime.
- the FFI shim has native ownership tests; Flutter native tests load the real Code Asset.
- `fixtures/projects/minimal` is the first committed golden project.
- fake provider capability descriptors exercise provider portability offline.
- provider invocation latency/retry fakes will be added behind the actual executor interface rather than inventing an async ABI early.

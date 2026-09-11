# ADR-0005: In-process Rust Engine with Typed Flutter FFI

- Status: Accepted
- Date: 2026-09-08
- Supersedes: ADR-0001's separate desktop process/IPC decision and ADR-0003's desktop stdio session-auth mechanism

## Context

ADR-0001 chose a separate Rust child process so Flutter UI lifecycle would not be coupled to machine-local production work. The bootstrap implementation used newline-framed JSON-RPC over stdio plus a per-session token.

That implementation introduced serialization, duplicate Dart/Rust wire models, method-string dispatch, process supervision, startup/shutdown races, and an authentication mechanism on a private parent/child pipe. It also did **not** deliver the principal reliability benefit used to justify the process boundary: when the Flutter parent exits, stdio closes and the child engine exits as well.

Kineto's desktop UI and Rust engine are shipped, versioned, and trusted as one local application. The hot boundary is therefore an in-process language boundary, not a network boundary.

## Decision

### Desktop boundary

Flutter desktop loads Rust in-process through Dart FFI.

```text
Flutter / Dart
    │
    │ typed native ABI
    ▼
app-specific FFI shim
    │
    ▼
kineto-core
```

The committed Flutter application uses Dart/Flutter Code Assets and build hooks to compile and bundle the Rust library. The app does not discover a Rust executable on `PATH`, launch a child engine, open a localhost port, or serialize control messages through JSON.

The FFI surface is deliberately narrow:

- opaque handles for owned Rust state
- fixed-width primitive ABI values for the bootstrap surface
- no Rust allocation is exposed without an explicit ownership/free rule
- no JSON/Markdown/TOML is used as a hot call boundary
- generated or specialized bridge tooling is introduced only when the typed async/stream surface is large enough to justify it

`kineto-core` remains ordinary Rust with no Flutter dependency. The FFI shim is app-specific and thin.

### Resource model

Rust owns heavy machine-local work: filesystem IO, hashing, provider clients, ffmpeg/process supervision, local model integration, CPU-heavy transforms, and future GPU work.

Do not create worker pools merely because cores are available. A workload chooses the appropriate execution mechanism:

- async IO uses a shared Rust runtime when the job system is introduced
- CPU-bound work uses measured bounded parallelism
- GPU work stays in provider/runtime-specific queues
- media bytes remain in files/native buffers whenever possible rather than crossing the Flutter boundary
- large typed buffers may use external/native-backed typed data when a UI preview genuinely needs them

The UI thread is not used for production compute.

### Reliability model

Kineto does not require a Rust process to outlive Flutter in order to be reliable.

Durability comes from:

- write-ahead job intent before paid/external work
- persisted provider job identifiers
- startup reconciliation of unfinished remote work
- canonical artifact/project state outside transient runtime state
- explicit interrupted/retry semantics for local processes

An application crash may stop in-process Rust execution. On restart, the engine reconstructs/reconciles work rather than assuming an immortal daemon.

### Future out-of-process or local-web mode

A network/socket transport is a separate adapter and is not part of the desktop FFI contract.

If Kineto later proves a need for local-web or a durable daemon, define a generated typed **binary** protocol for that transport (for example protobuf or an equivalently schema-driven format), plus the authentication/origin/host rules required by ADR-0003. Do not reintroduce ad-hoc JSON-RPC as the desktop boundary.

## Consequences

### Positive

- no JSON encode/decode or method-string dispatch in the desktop hot path
- no second engine process/heap for the baseline app
- no child-process lifecycle races for normal Flutter↔Rust calls
- no session token on a private in-process boundary
- native library is bundled by the Flutter build rather than located through `PATH`
- Rust remains independently unit-testable
- future durability work targets actual job recovery instead of process survival theater

### Negative

- a fatal Rust native crash can terminate the desktop process
- the FFI ABI needs explicit ownership and compatibility discipline
- future local-web/daemon support needs its own transport adapter rather than reusing the desktop call mechanism

## Rules

1. Do not serialize ordinary desktop Flutter↔Rust calls through JSON, HTTP, stdio, or localhost.
2. Do not move production compute into Dart to avoid FFI design work.
3. Do not pass large media payloads through the bridge when a path, handle, mmap/native buffer, or provider reference is sufficient.
4. Keep `kineto-core` free of Flutter/UI dependencies.
5. Measure before introducing extra runtimes, worker pools, caches, or cross-process copies.

# Native Performance Playbook

## Status

This is a project rule for Kineto desktop/native implementation and review.

Kineto is local-first by design. Local CPU, GPU, memory, filesystem, and process resources are product advantages, not an excuse to recreate a network service inside one application.

## Boundary rule

For desktop, Flutter and Rust are one process and one product image.

- ordinary control/state calls use typed FFI
- do not introduce JSON, HTTP, localhost, stdio framing, protobuf, or another serialization layer unless the two endpoints are actually separated
- a future daemon/local-web adapter may use a generated binary protocol, but it must not infect the in-process desktop path

## Copy and allocation budget

Treat every large cross-language copy as a design decision.

Preferred order:

1. stable IDs and small fixed-width values
2. project-relative paths / asset handles for persisted media
3. borrowed/native-backed typed buffers with explicit lifetime when bulk data must be inspected by Flutter
4. owned copies only when measurements show the simplicity is worth the cost

Never base64 media for local transport. Never stringify typed state merely to cross FFI. Do not reparse canonical JSON/TOML for every UI read; keep validated working state in Rust memory and persist at durability boundaries.

## Runtime rule

Do not create one executor, thread pool, or worker process per subsystem.

When P0 jobs arrive:

- IO-bound orchestration shares one intentional Rust async runtime
- CPU-bound work is isolated from async executor threads and uses bounded parallelism chosen by benchmark
- GPU/model runtimes keep their own queue/concurrency semantics
- ffmpeg and external local models remain subprocesses when they are naturally external tools; they do not justify making `kineto-core` a daemon
- Flutter receives coarse job/artifact events, not internal per-frame telemetry

`available_parallelism()` is a host hint, not a policy. Do not assume logical CPU count equals ideal concurrency, especially on heterogeneous Apple Silicon.

## Apple Silicon review gate

Before adding a new native data path or concurrency layer, profile at least one representative Apple Silicon machine (M1-class or newer) and one x86_64 desktop.

For the vertical slice, record at minimum:

- process RSS / peak resident memory
- native heap growth across repeated operations
- Flutter/Dart heap growth
- thread count and idle-thread footprint
- CPU time and wall time for local CPU stages
- number/size of Flutter↔Rust bulk copies
- event rate crossing the bridge
- cold and warm startup cost of local model/process adapters

Use Instruments on macOS, `perf`/equivalent tooling on Linux, and ETW/Windows Performance Recorder where appropriate. Optimize measured bottlenecks, not architecture diagrams.

## ABI rule

- keep the app-specific C ABI small
- use opaque handles and fixed-width primitives for simple calls
- specify ownership for every pointer/buffer
- no Rust panic may unwind across FFI
- no Rust allocator-owned value crosses without an explicit free/lifetime API
- add generated bindings or a larger bridge layer only when the surface is large enough that manual bindings become a correctness risk

## Review questions

Every native change should be able to answer:

- Why is this data crossing into Dart at all?
- How many allocations/copies happen on the hot path?
- Does this need a new thread/runtime/process, or can it reuse an existing one?
- What survives an app crash, and is that durability persisted rather than accidentally tied to process lifetime?
- Has this been measured on Apple Silicon before adding platform-specific tuning?

# ADR-0003: Local Engine Security Boundary

- Status: Accepted
- Date: 2026-09-07

## Context

ADR-0001 establishes a separate local Rust engine and allows a future localhost/WebSocket transport. That engine will eventually hold provider credentials, access project files, and spawn tools such as ffmpeg or local model adapters.

A localhost endpoint is not a trusted boundary by itself. Browser pages can reach loopback services, DNS rebinding defeats naive host assumptions, and project or MCP content must be treated as untrusted input.

## Decision

### Session authentication

Every Kineto RPC request carries a per-engine-session token.

For the bundled stdio transport:

1. the trusted Flutter parent generates a cryptographically random token before spawn
2. the token is passed to the child through `KINETO_SESSION_TOKEN`
3. the same token is attached to every RPC request as `session_token`
4. the engine rejects missing or mismatched tokens
5. the token must never be written to logs, project files, prompts, exports, or telemetry

The token is not returned by an unauthenticated handshake. A future local-web bootstrap must deliver it out of band to the trusted UI.

### Future local-web transport

Local-web mode must be opt-in and must:

- bind only to loopback
- require the session token on every call
- validate `Origin` and `Host`
- reject unexpected origins/hosts even when the token is present
- never expose a token-discovery endpoint to arbitrary browser origins

### Filesystem boundary

Normal engine operations are scoped to the currently opened project directory and engine-owned configuration/cache locations. Access outside those roots requires an explicit, capability-specific user action such as importing an external asset.

Project data must never be able to provide an arbitrary filesystem path that is blindly trusted by an adapter.

### Process-spawn boundary

Process adapters use application-defined executable identities and argument builders. Projects may provide data inputs, but never an arbitrary command string to execute.

Experimental CLI adapters must be explicitly enabled and restricted to an allowlist/configured executable path.

### Untrusted content boundary

Source material, generated model output, MCP tool results, and imported metadata are untrusted data. They cannot directly authorize:

- provider spend
- fallback to a paid provider
- filesystem writes outside the project boundary
- process execution
- secret disclosure

Those actions require workflow policy and, where configured, human approval.

## Consequences

### Positive

- authentication is part of protocol v1 rather than retrofitted later
- the stdio implementation exercises the same request-auth concept future transports need
- opening a project does not imply permission to execute commands from it
- browser reachability of localhost is not treated as authentication

### Negative

- launchers must provision a session token
- tests and protocol fixtures must include authentication metadata
- local-web bootstrap will require an explicit trusted-token delivery mechanism

## Required tests

- wrong or missing session token is rejected
- engine refuses startup without a stdio session token
- secrets/session tokens never appear in logs, prompt files, or exported artifacts
- future local-web tests cover Origin/Host validation and loopback-only binding

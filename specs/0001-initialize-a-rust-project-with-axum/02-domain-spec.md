# Domain Specification

## Overview

This requirement establishes the foundation for a new web service: a running
HTTP server that other services, tools, or developers can connect to. Once
implemented, the repository will provide a runnable application that listens
for HTTP requests and responds to them. There is no existing application
behavior to preserve — this is the starting point for all future
functionality of "reqctl-test".

## User-facing behavior

- **Starting the service**: A developer (or operator) can start the
  application using a standard Rust build/run workflow. Once started, the
  service becomes available to accept HTTP connections on a configured
  address/port.
- **Configuring the listen port**: The port the service listens on is
  configurable rather than hard-coded, so different environments (local
  development, CI, deployment) can run the service on different ports
  without changing code. Per the user's review feedback at step 4, this
  configuration is provided via a configuration file and/or an environment
  variable:
  - If the operator sets the port via the environment variable, that value
    is used.
  - If the operator instead (or additionally) provides a configuration file
    specifying the port, that value is used.
  - If neither is provided, the service falls back to a sensible default
    port so it remains usable "out of the box" with no setup.
  - The relative precedence between the environment variable and the
    configuration file (which one "wins" if both are present) is left as an
    assumption below for the user to confirm.
- **Responding to requests**: The service answers HTTP requests sent to it.
  At minimum, a request to the root path (`GET /`) returns a successful
  response (HTTP 200) with a simple body (e.g. "Hello, world!"). This
  primarily serves as:
  - A smoke test / health check — confirming the service is running and
    reachable.
  - A template/example endpoint that future requirements can replace or
    extend with real functionality.
- **Stopping the service**: The service runs until the process is terminated
  (e.g. via Ctrl-C or process signal). No special shutdown behavior (graceful
  drain, etc.) is in scope for this requirement.

## Business rules

- The service must be reachable over HTTP — any client capable of making an
  HTTP request to the configured host/port should receive a response.
- The listening port must not be hard-coded into the application; it must be
  derived from configuration (file and/or environment variable) at startup,
  with a default applied when no configuration is supplied.
- The root endpoint's response is purely illustrative at this stage; it
  carries no business meaning beyond "the service is up." Future
  requirements are expected to add meaningful routes, and this endpoint may
  later be repurposed as a dedicated health-check endpoint (e.g. `/health`)
  or removed.
- No authentication, authorization, persistence, or external integrations
  are part of this requirement. The service has no data model yet.

## Edge cases

- **Invalid port configuration**: If the configured port value (from the
  environment variable or config file) is not a valid port number, the
  application is expected to fail to start with a clear error, rather than
  silently ignoring the bad value or starting on an unintended port.
- **Conflicting configuration sources**: If both the environment variable
  and the configuration file specify a port, one source must take
  precedence in a well-defined, documented way (see assumptions below) —
  behavior should not be ambiguous or order-dependent in a confusing way.
- **Missing configuration**: If neither the environment variable nor a
  configuration file is present, the service still starts successfully on
  the default port.
- **Unrecognized paths/methods**: Requests to any path other than `/` (or
  with methods other than `GET`) are expected to receive axum's default
  "not found" / "method not allowed" response, since no other routes exist
  yet. This is acceptable for this requirement.
- **Port already in use**: If the configured port is unavailable, the
  application is expected to fail to start (and report an error), rather
  than silently running on a different port. There is no fallback/retry
  behavior in scope.
- **Concurrent requests**: The service should be able to handle multiple
  simultaneous requests without one blocking another, as is standard for an
  async HTTP server. No specific load or concurrency targets are defined for
  this requirement.

## Fit with existing functionality

- There is no existing application functionality in this repository — only
  specification documents and repository metadata exist. This requirement
  does not change or interact with any prior behavior; it solely establishes
  the initial runnable project structure that subsequent requirements will
  build upon.
- The configuration mechanism (env var / config file) introduced here for
  the port is intended to be the starting point for any further
  configuration needs (e.g. log level, external service URLs) that future
  requirements may add.
- Project-level housekeeping (e.g. ignoring build artifacts) is part of this
  setup but has no user-facing effect — it only affects what files are
  tracked in version control.

## Assumptions carried over from analysis (for user review)

The following assumptions shape the domain behavior described above and
should be confirmed or corrected:

1. The service still listens on host `0.0.0.0`; only the **port** is made
   configurable for now (no host/address configuration).
2. **Default port**: if no configuration is supplied, the service falls back
   to port `3000` (the previously assumed default).
3. **Configuration sources and precedence**: the port may be supplied via an
   environment variable (e.g. `PORT` or an app-specific name such as
   `REQCTL_PORT`) and/or a configuration file (e.g. a `config.toml` at the
   project root). If both are present, the environment variable takes
   precedence over the configuration file — this matches common convention
   (env vars override file-based config) but should be confirmed.
4. The only endpoint is `GET /`, returning a simple "Hello, world!" /
   health-check style message — no other routes, request handling, or
   business logic exists at this stage.
5. The project is a single binary application (not a library or multi-crate
   workspace), named `reqctl-test`.
6. No additional capabilities (logging/tracing, database access,
   authentication, Docker, CI, tests) are included as part of this initial
   setup beyond the port configuration described above — these would be
   addressed by future requirements if needed.

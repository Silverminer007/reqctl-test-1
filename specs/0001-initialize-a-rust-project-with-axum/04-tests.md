# Test Definitions

This document lists concrete test cases that verify the behavior described
in `02-domain-spec.md` is correctly implemented per `03-tech-spec.md`. These
are human-readable test plans; corresponding test code is written at step 10.

Tests fall into two groups:

- **Config tests** — unit tests for `config::load()` covering precedence,
  defaults, and validation/error behavior.
- **HTTP/router tests** — integration-style tests for the axum app covering
  the `GET /` route and default fallback behavior.

---

## 1. Config resolution tests (`src/config.rs`)

### 1.1 No configuration present (default port)

- **Scenario**: Neither the `REQCTL_PORT` env var is set, nor a
  `config.toml` file exists (or the file exists but has no `port` key).
- **Inputs**: `REQCTL_PORT` unset; no `config.toml` / `config.toml` without
  a `port` entry.
- **Expected outcome**: `config::load()` returns `Ok(AppConfig { port: 3000 })`
  (i.e. `DEFAULT_PORT`).

### 1.2 Port from config file only

- **Scenario**: `config.toml` specifies a valid port; env var is unset.
- **Inputs**: `REQCTL_PORT` unset; `config.toml` contains `port = 8080`.
- **Expected outcome**: `config::load()` returns `Ok(AppConfig { port: 8080 })`.

### 1.3 Port from env var only

- **Scenario**: `REQCTL_PORT` env var is set to a valid port; no
  `config.toml` is present.
- **Inputs**: `REQCTL_PORT=8081`; no `config.toml`.
- **Expected outcome**: `config::load()` returns `Ok(AppConfig { port: 8081 })`.

### 1.4 Env var takes precedence over config file

- **Scenario**: Both `REQCTL_PORT` and `config.toml`'s `port` are set to
  different valid values.
- **Inputs**: `REQCTL_PORT=9090`; `config.toml` contains `port = 8080`.
- **Expected outcome**: `config::load()` returns `Ok(AppConfig { port: 9090 })`
  — the env var value wins.

### 1.5 Env var set to invalid (non-numeric) value

- **Scenario**: `REQCTL_PORT` is set to a value that cannot be parsed as
  `u16`.
- **Inputs**: `REQCTL_PORT="not-a-port"`; `config.toml` absent or valid.
- **Expected outcome**: `config::load()` returns
  `Err(ConfigError::Env(..))` with a message indicating `REQCTL_PORT` must
  be a valid port number (0–65535) and showing the offending value.

### 1.6 Env var set to out-of-range numeric value

- **Scenario**: `REQCTL_PORT` is set to a number outside the valid `u16`
  range (e.g. negative or > 65535).
- **Inputs**: `REQCTL_PORT="99999999"` (or `REQCTL_PORT="-1"`).
- **Expected outcome**: `config::load()` returns `Err(ConfigError::Env(..))`.

### 1.7 Config file with invalid TOML syntax

- **Scenario**: `config.toml` exists but is not valid TOML.
- **Inputs**: `REQCTL_PORT` unset; `config.toml` contains malformed TOML
  (e.g. `port = `, or unbalanced syntax).
- **Expected outcome**: `config::load()` returns `Err(ConfigError::File(..))`
  with a message indicating the file failed to parse.

### 1.8 Config file with `port` not representable as `u16`

- **Scenario**: `config.toml` is syntactically valid TOML, but `port` is a
  value that cannot fit in `u16` (e.g. a string, a negative number, or a
  number > 65535).
- **Inputs**: `REQCTL_PORT` unset; `config.toml` contains `port = "abc"` (or
  `port = 70000` / `port = -1`).
- **Expected outcome**: `config::load()` returns `Err(ConfigError::File(..))`.

### 1.9 Missing config file is not an error

- **Scenario**: `config.toml` does not exist at `CONFIG_FILE_PATH`, and
  `REQCTL_PORT` is unset.
- **Inputs**: No `config.toml`; `REQCTL_PORT` unset.
- **Expected outcome**: `config::load()` returns `Ok(AppConfig { port: 3000 })`
  (treated as `FileConfig::default()`), not an error.

### 1.10 Invalid env var takes precedence over (would-be-valid) file value for error reporting

- **Scenario**: `REQCTL_PORT` is invalid AND `config.toml` specifies a
  valid port.
- **Inputs**: `REQCTL_PORT="bogus"`; `config.toml` contains `port = 8080`.
- **Expected outcome**: `config::load()` returns `Err(ConfigError::Env(..))`
  — an invalid env var is an error regardless of whether the file would
  have provided a usable fallback (per tech spec: env var, if *set*, must
  be valid; the file is only consulted if the env var is unset).

---

## 2. HTTP / router tests (`src/main.rs` or integration test)

### 2.1 `GET /` returns 200 with "Hello, world!"

- **Scenario**: The application's router is exercised with a `GET /`
  request (via `tower::ServiceExt::oneshot` or by binding to an ephemeral
  port and issuing a real HTTP request).
- **Inputs**: `GET /`.
- **Expected outcome**: HTTP status `200 OK`; response body is
  `"Hello, world!"`.

### 2.2 Unknown path returns 404

- **Scenario**: A request is made to a path that has no registered route.
- **Inputs**: `GET /does-not-exist`.
- **Expected outcome**: HTTP status `404 Not Found` (axum's default
  fallback response).

### 2.3 Unsupported method on `/` returns 405

- **Scenario**: A request using a method other than `GET` is made to `/`.
- **Inputs**: `POST /` (or `PUT /`, `DELETE /`).
- **Expected outcome**: HTTP status `405 Method Not Allowed` (axum's
  default response for a route that exists but doesn't support the
  method).

### 2.4 Concurrent requests are handled independently

- **Scenario**: Multiple `GET /` requests are issued concurrently against a
  running server instance (bound to an ephemeral port).
- **Inputs**: N concurrent `GET /` requests (e.g. N = 10).
- **Expected outcome**: All requests complete successfully with `200 OK`
  and body `"Hello, world!"`; no request blocks or fails due to another
  in-flight request.

---

## 3. Startup / process-level tests (manual or higher-level, optional for CI)

These verify behavior that's harder to express as a pure unit test but is
called out explicitly in the domain spec's edge cases. They may be covered
by the config and router tests above at the unit level, but are listed here
for completeness/manual verification:

### 3.1 Application fails to start with invalid port configuration

- **Scenario**: The compiled binary is run with an invalid `REQCTL_PORT`
  (or invalid `config.toml`).
- **Inputs**: `REQCTL_PORT=not-a-number cargo run` (or equivalent).
- **Expected outcome**: The process exits with a non-zero status and prints
  an error message describing the invalid configuration, without
  attempting to bind a socket. (Covered at the unit level by 1.5–1.8;
  process-level confirmation is a manual/optional smoke check.)

### 3.2 Application fails to start when the configured port is already in use

- **Scenario**: Another process is already bound to the configured port
  when the application starts.
- **Inputs**: A listener bound to port `3000` (or the configured port)
  before starting the application with the same port.
- **Expected outcome**: `TcpListener::bind` fails (e.g. `AddrInUse`); the
  process exits with a non-zero status and an error is reported. No
  fallback to a different port occurs.

### 3.3 Application starts successfully with no configuration present

- **Scenario**: Application is run with no `REQCTL_PORT` and no
  `config.toml`.
- **Inputs**: Clean environment, no config file.
- **Expected outcome**: Application starts and listens on `0.0.0.0:3000`;
  a `GET /` request to `http://localhost:3000/` returns `200 OK` with
  `"Hello, world!"`. (Covered at integration level by 1.1 + 2.1.)

---

## Coverage summary

| Domain spec behavior / edge case | Test case(s) |
|---|---|
| Default port when no config supplied | 1.1, 3.3 |
| Port from config file | 1.2 |
| Port from env var | 1.3 |
| Env var precedence over file | 1.4, 1.10 |
| Invalid env var port value (error) | 1.5, 1.6, 3.1 |
| Invalid/malformed config file (error) | 1.7, 1.8, 3.1 |
| Missing config file is not an error | 1.9 |
| `GET /` returns 200 "Hello, world!" | 2.1, 3.3 |
| Unrecognized path → 404 | 2.2 |
| Unsupported method → 405 | 2.3 |
| Concurrent requests handled independently | 2.4 |
| Port already in use → startup failure | 3.2 |

# QA Notes

## What was checked

- Read `02-domain-spec.md`, `03-tech-spec.md`, `04-tests.md`, and
  `05-architecture.md` for requirement 0001.
- Reviewed the implemented project at the repository root (note: there is no
  `app/` directory — the approved architecture places the Cargo project at
  the repo root, package `reqctl-test`, which matches `Cargo.toml`,
  `config.toml`, `src/main.rs`, `src/config.rs`, and `.gitignore`).
- Verified the implementation against the **Final Architecture** section of
  `05-architecture.md`, which is explicitly authoritative for this
  requirement and supersedes the precedence model and test cases 1.4–1.6 /
  1.10 from `02-domain-spec.md` / `03-tech-spec.md` / `04-tests.md`.
- Ran `cargo build` — succeeds with no warnings.
- Ran `cargo test` — all 18 tests pass:
  - `src/config.rs` unit tests cover 1.1–1.3, 1.7–1.9, and the superseding
    1.4'–1.13' cases from the Final Architecture's "Config resolution test
    table" (literal port ignoring env var, interpolation with/without
    default, non-numeric interpolated value, required-but-unset env var,
    unreadable config file, comments not expanded).
  - `src/main.rs` integration-style tests (via `tower::ServiceExt::oneshot`)
    cover 2.1 (`GET /` → 200 "Hello, world!"), 2.2 (unknown path → 404), 2.3
    (`POST /` → 405), and 2.4 (10 concurrent `GET /` requests all succeed).
- Ran `cargo clippy --all-targets` — no warnings or lints.
- Manually ran the built binary (`cargo run`) with no configuration present:
  bound to `0.0.0.0:3000`; `GET /` returned `200 "Hello, world!"`,
  `GET /does-not-exist` returned `404`, `POST /` returned `405` — matches
  3.3, 2.1, 2.2, 2.3.
- Manually ran with `REQCTL_PORT=not-a-port`: the process exits with status
  1 and prints a clear `ConfigError::File` message showing the expanded
  `config.toml` contents (`port = not-a-port`) and the TOML parse error —
  matches 3.1 / 1.10'.

## Findings

- The implementation follows the **Final Architecture** (the authoritative
  spec for this requirement) precisely:
  - `config.toml` ships with `port = ${REQCTL_PORT:-3000}` and the code
    matches `DEFAULT_CONFIG_CONTENTS`.
  - `src/config.rs` implements `load()` / `resolve()` /
    `expand_env_vars()` exactly as specified, including the
    `io::ErrorKind::NotFound`-only fallback to default contents (Critique
    refinement #1), the single `ConfigError::File(String)` variant with
    `Display`/`Error` impls, and the hand-written `${NAME}` /
    `${NAME:-default}` scanner with comment-passthrough.
  - `src/main.rs` matches the specified entry point shape (`config::load()?`,
    `Router::new().route("/", get(root))`, bind on `0.0.0.0:<port>`,
    `axum::serve`).
  - `Cargo.toml` dependencies match the Final Architecture's table (axum,
    tokio "full", serde "derive", toml; dev-dependency tower "util"). No
    extra dependencies added.
  - `.gitignore` matches the specified Rust-appropriate contents
    (`/target`, `.env`); `Cargo.lock` is committed.
- As noted in `05-architecture.md`'s "Open questions" §1, the precedence
  model in `02-domain-spec.md` (§"Configuration sources and precedence") and
  `03-tech-spec.md` (the `load()` precedence rules and `ConfigError::Env`)
  is now stale relative to the approved Final Architecture (env-var
  interpolation mediated entirely through `config.toml`, single
  `ConfigError::File` variant). This staleness was explicitly flagged by the
  architecture step as an intentional, scoped deviation with a
  self-contained replacement test table, and the Developer correctly
  implemented against the Final Architecture rather than the stale sections.
  This is a documentation-consistency issue in earlier spec documents, not a
  defect in the code, and does not block this requirement — but it should be
  reconciled (updating 02/03/04) before/while building on this foundation in
  future requirements, since new contributors reading 02/03/04 in isolation
  would get a misleading picture of the config precedence model.
- No other discrepancies, missing pieces, or test failures found. Edge
  cases from the domain spec (invalid port config, missing config, port in
  use via `?`-propagated `io::Error`, 404/405 fallbacks, concurrency) are all
  addressed per the Final Architecture.

## Overall assessment

The implementation is internally consistent, builds and tests cleanly, and
faithfully matches the authoritative Final Architecture for requirement
0001. The only outstanding issue is the known, already-flagged staleness of
`02-domain-spec.md` / `03-tech-spec.md` / `04-tests.md` relative to the
Final Architecture's config model — a documentation reconciliation task, not
a code defect, and not a blocker for this requirement's completion.

## Decision

pass

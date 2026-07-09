# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## What this is

A thin Rust launcher for the [Pluto.jl](https://plutojl.org) notebook server. The
binary is named `pluto` (see `[[bin]]` in `Cargo.toml`); the crate is `pluto-cli`.
It parses CLI flags with clap, then `exec()`s a `julia` process that runs Pluto.
All logic lives in `src/main.rs`.

## Commands

The `justfile` is the source of truth and mirrors the CI jobs.

- `just test` — `cargo test --all-features`
- `just lint` — `cargo clippy --all-targets --all-features -- -D warnings` (warnings are errors)
- `just format` / `just format-check` — apply / verify `rustfmt`
- `just ci` — full CI locally: `format-check lint check test build`
- `just check-all` — fast static checks, no tests or release build
- Run a single test: `cargo test <name>` (e.g. `cargo test path_with_spaces_stays_one_argument`)

## Architecture

The design is deliberately narrow: **build a `julia` argv, then hand off with `exec()`**.

1. **`PLUTO_SNIPPET` is a constant Julia program, never templated.** User input is
   *never* string-interpolated into the snippet. Instead the snippet reads `ARGS` and
   parses `key=value` pairs at runtime (`notebook=...`, `base_url=...`, `threads=...`).
   This is the core security/correctness invariant — it's why paths with spaces or
   special characters need no escaping. Any new option must flow through this same
   `key=value` protocol, not by editing the snippet to embed a value.

2. **`julia_args()` is the seam between the two layers.** It translates parsed clap
   flags into the julia argv. Note the split: `--project` becomes a *julia* flag
   (`--project=...`, before `-e`), while notebook/base_url/threads become `key=value`
   pairs *after* the `--` separator (consumed by the snippet's `ARGS` loop). The unit
   tests assert exactly this boundary — `kv_pairs()` slices the argv at `--`.

3. **`exec()` replaces the process** (`std::os::unix::process::CommandExt`). After a
   successful launch this Rust process no longer exists — signals (Ctrl-C) and the exit
   code belong to `julia`/Pluto. `exec()` only returns on failure to launch, which is
   the sole error path in `main`. **This makes the tool Unix-only** (no Windows support).

4. **`-t/--notebook-threads` configures Pluto's *worker* processes**, not the server
   process — it maps to Pluto's `threads` compiler option. Keep this distinction when
   documenting or extending thread-related flags.

## Conventions when extending

- Adding a `Pluto.run` keyword: add a clap field, then append a `key=value` pair in
  `julia_args()`, and add the snippet's `ARGS` branch if it needs non-string handling
  (see how `threads` uses `tryparse(Int, v)`). Don't touch `PLUTO_SNIPPET` to embed values.
- `--startup-file=no` is intentional (fast, isolated launches); don't remove it.
- The `julia` binary is overridable via `--julia` or the `PLUTO_CLI_JULIA` env var.

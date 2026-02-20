# Agent Development Guide

A file for guiding coding agents.

## Commands

- **Format:** `cargo fmt`
- **Test:** `cargo test`
- **Build:** `cargo build`
- **Run lints/docs only when explicitly requested by the user.**

## Directory Structure

- Core simulator code: `src/`
- Device models: `src/devices/`
- Simulation logic: `src/sim/`
- Integration tests: `tests/`
- CI workflows: `.github/workflows/`
- Project documentation: `README.md`, `docs/`, `CONTRIBUTING.md`

## Working Guidelines

- Keep changes scoped to the user request.
- Do not revert unrelated local changes.
- Update docs/tests when behavior changes.
- Prefer small, clear diffs over broad refactors.

## Issue and PR Guidelines

- Do not create issues or PRs unless the user explicitly asks.
- If asked, include clear scope and acceptance criteria.

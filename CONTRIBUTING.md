# Contributing to vpp-sim

Thanks for your interest in contributing to `vpp-sim`.

This guide is for anyone opening an issue, discussion, or pull request.

## The Most Important Rule

You must understand your code.

If you submit a change, you should be able to explain:
- what it does,
- why it is correct,
- what tradeoffs it introduces,
- and how it was tested.

Using AI tools is allowed. Submitting AI-generated changes you do not understand is not.

## Quick Start

1. Search existing issues before opening a new one.
2. If the work is not already tracked, open an issue first.
3. Keep scope small and targeted.
4. Open a PR that links the issue (`Closes #<number>` when appropriate).
5. Include tests and docs updates where relevant.

## Issues vs Discussions

Use **issues** for actionable, scoped work.

Use **discussions** for:
- early design ideas,
- unclear bug reports,
- questions about approach or roadmap.

If you are unsure whether something is a bug or a feature request, start with a discussion.

## Pull Request Expectations

PRs are most likely to be accepted when they:
- implement an existing issue,
- stay focused on one concern,
- include tests for behavior changes,
- update docs/README for user-facing changes,
- use clear commit messages (Conventional Commits preferred).

Please avoid opening PRs as a place to design features from scratch. Use an issue/discussion first.

## Review and Ownership

This repository uses CODEOWNERS. `@jdhoffa` is the code owner for this repo.

All incoming PRs should be reviewed and approved by `@jdhoffa` before merge.

## Development and Testing

Prerequisites:
- Rust (latest stable)
- Cargo

Common commands:

```bash
cargo fmt
cargo test
```

## Style Guidelines

- Prefer small, composable functions.
- Keep naming explicit over clever.
- Preserve deterministic simulation behavior when seeds/config are fixed.
- Avoid unrelated refactors in feature/fix PRs.

## Documentation Requirements

Update docs when behavior changes, including:
- CLI flags,
- telemetry schema,
- HTTP endpoints,
- setup/run instructions.

At minimum, update `README.md` for user-visible changes.

## Commit and Branch Guidance

- Branch names: prefix with the GitHub issue number when appropriate, then a short descriptor (e.g., `23-http_telemetry_range`).
- Commit style: Conventional Commits (e.g., `feat(api): add telemetry range query`).

## Code of Conduct

Be respectful and constructive. Assume good intent and focus feedback on the code and behavior.

## Licensing

By contributing, you agree your contributions are licensed under this repository's MIT License.

# Contributing to sessionx

Thanks for considering a contribution. This document covers the basics.

## Project status

`sessionx` is a small, focused tmux session manager. Scope is intentionally
narrow — see the README for what it does. Suggestions that significantly
expand scope are likely to be declined; please open an issue to discuss
before writing a large PR.

## Development setup

Requirements:

- Rust stable (1.75+) — install via [rustup](https://rustup.rs)
- tmux 3.0+
- git 2.5+
- bash

Build and test:

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

Run a local binary:

```sh
cargo run -- ls --all
```

## Submitting a pull request

1. Open an issue first for non-trivial changes. This avoids wasted work.
2. Fork the repo and create a feature branch.
3. Keep PRs focused — one logical change per PR.
4. Make sure `cargo fmt`, `cargo clippy`, and `cargo test` pass locally.
5. Update `README.md` if behavior or commands change.
6. Write a clear PR description: what changed and why.

## What kinds of contributions are welcome

- Bug fixes (with a reproducer if possible)
- Documentation improvements
- New built-in themes
- Small UX improvements to existing commands
- Test coverage

## What is unlikely to land

- New top-level commands without prior discussion
- Heavy dependencies for marginal features
- Changes that break the no-daemon design
- Windows support (not in scope)

## Review process and SLAs

This is a side project maintained in spare time. Realistic expectations:

| Type | Response time |
|---|---|
| Bug reports | Acknowledged within ~1 week |
| Pull requests | Initial review within ~2 weeks |
| Security reports | Acknowledged within 48 hours (see `SECURITY.md`) |

If a PR has been waiting longer than this, a polite ping on the PR is fine.

## Communication

- Use GitHub issues and PRs. Decisions are documented in the repo, not in DMs.
- Be kind. See [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions will be licensed under
the [MIT License](LICENSE).

# Security Policy

## Supported versions

`sessionx` is pre-1.0. Only the **latest release** on the `main` branch is
supported. Older versions will not receive security fixes.

| Version | Supported |
|---|---|
| latest `main` / latest release | yes |
| anything else | no |

## Reporting a vulnerability

**Do not open a public GitHub issue for security reports.**

Please use GitHub's [private security advisories](../../security/advisories/new)
to report a vulnerability. This keeps the report private until a fix is ready.

If you cannot use GitHub advisories, contact the primary maintainer listed in
[`MAINTAINERS.md`](MAINTAINERS.md) directly.

## What to include

- A clear description of the issue
- Steps to reproduce, or a proof-of-concept
- The version / commit affected
- Any suggested mitigation

## What to expect

| Stage | Target |
|---|---|
| Acknowledgement | within 48 hours |
| Initial assessment | within 1 week |
| Fix or mitigation plan | within 30 days for confirmed issues |
| Public disclosure | coordinated with reporter, after a fix is released |

## Scope

In scope:

- Code execution, privilege escalation, or sandbox escape via crafted
  `.sessionx.yaml`, hook scripts, theme files, or session names.
- Path traversal in paths read or written by `sessionx`.
- Unsafe handling of `tmux` / `git` shell invocations (e.g., command injection).

Out of scope:

- Issues that require an attacker to already have write access to the user's
  home directory or the project repository.
- Vulnerabilities in `tmux`, `git`, or the Rust toolchain themselves —
  please report those upstream.
- Denial of service via resource exhaustion on the local machine.

## Credit

We are happy to credit reporters in the release notes for the fix, unless
you prefer to remain anonymous.

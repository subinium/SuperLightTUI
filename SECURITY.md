# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.13.x  | Yes       |
| < 0.13  | No        |

Only the latest minor release receives security fixes.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, use one of these methods:

1. **GitHub Security Advisory** (preferred): [Report a vulnerability](https://github.com/subinium/SuperLightTUI/security/advisories/new)
2. **Email**: Contact the maintainer directly (see GitHub profile)

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Impact assessment (if known)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix release**: Depends on severity (critical: ASAP, others: next patch)

### Scope

SLT is a terminal UI rendering library. Security concerns typically involve:

- Terminal escape sequence injection
- Denial of service through malformed input
- Unsafe memory access (SLT uses `#![forbid(unsafe_code)]`)
- Dependency vulnerabilities (monitored by `cargo-audit` in CI)

### After a Fix

- A patch version is released with the fix
- The vulnerability is disclosed in CHANGELOG.md after the fix is available
- Credit is given to the reporter (unless they prefer anonymity)

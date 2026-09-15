# Security Policy

## Reporting a vulnerability

Use GitHub private vulnerability reporting for this repository:

`Security` → `Advisories` → `Report a vulnerability`.

Do not disclose vulnerabilities, exploit details, credentials, tokens, private keys, personal data, proprietary assets, production endpoints, database dumps, or session material in public issues, pull requests, discussions, or logs.

Include:

- affected commit, version, component, and configuration;
- impact and realistic attack preconditions;
- minimal reproduction or proof of concept;
- whether secrets or production data may be exposed;
- suggested mitigation when known.

The maintainer will acknowledge the report, assess severity, coordinate a private fix, rotate compromised credentials when necessary, and publish an advisory when disclosure is appropriate.

## Supported versions

Oteryn v2 is pre-release. Only the current `main` branch and explicitly published supported releases receive security fixes.

## Scope

Security-sensitive areas include authentication, Game Session admission, protocol parsing, updater/download verification, persistence, item and currency transactions, analytics access, build/release workflows, dependencies, secrets, and proprietary asset provenance.

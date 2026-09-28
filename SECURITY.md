# Security Policy

TwoCents stores household financial data in a local SQLite file and has no
server, telemetry, or sync. Please report vulnerabilities privately rather than
in a public issue.

## Reporting

Use GitHub's **Security → Report a vulnerability** on this repository
(`https://github.com/Nucletheus/TwoCents/security/advisories/new`). Include
the app version, your Windows version, and reproduction steps.

You can expect an acknowledgement within a few days. Fixes ship in the next
release.

## Scope

In scope: the installer, the SQLite schema/migrations, backup and migration
behaviour, and any path that writes outside the install folder.

Out of scope: vulnerabilities in upstream crates that have no demonstrated
impact through TwoCents' own code.

## Handling your data

The database is unencrypted. Anyone with read access to the install folder can
read it. Do not attach a real database or a real `twocents.sqlite` to a report;
use a freshly created install with invented values instead.

# ADR-0016: Make app approval an operator setting

**Status:** Accepted; amends ADR-0007's per-app approval decision
**Date:** 2026-10-20

## Context

The singleton daemon serves different MCP clients. Notifications incorrectly
name Claude even when Codex or Cursor requested control. A local operator also
wants unattended app approval without maintaining a list of every app class.

## Decision

Use the client-neutral name KDE MCP in runtime permission, activity, and
cancellation notifications and the global shortcut label.

Add a top-level `auto_approve` boolean to `policy.toml`, defaulting to false.
When true, unapproved apps proceed without an approval notification. The same
policy gate still checks cancellation, application tiers, and sentinels; the
dispatch wrapper still enforces the mutation lease and audit logging. Explicit
in-memory denials remain denied until the process restarts, rather than being
removed after one rejected call. Automatic approvals are not persisted as
individual session approvals.

## Consequences

Existing installations retain interactive approval by default. Opted-in
installations allow any app permitted by their configured tier policy. The
setting is read at process startup; restart the daemon after changing it.
The service name accurately identifies the actor even when multiple clients
share the daemon. This does not infer which connected client owns a request.

## Alternatives

Pre-populating `approvals.json` does not cover newly installed apps. Removing
the policy gate would discard unrelated restrictions and cancellation. Naming
a specific client in a shared daemon would remain misleading.

---
id: DISK-19
projectId: PRJ-DISKTREE
title: 'Release 0.11.0: Version bumpen, Release Notes, Tag auf fork/main'
summary: Workspace-Version auf 0.11.0, Cargo.lock aktualisieren, kuratierte Release
  Notes (eigene DISK-Aenderungen + upstream), Release-Workflow nutzt die Notes-Datei,
  feature/disk-series nach fork/main mergen, Tag v0.11.0 pushen.
status: REVIEW
priority: MEDIUM
type: CHORE
assignee: build-opencode
testsBy: AGENT
effort: SMALL
blockedBy: [
  ]
referencesTicket: [
  ]
allowedPaths:
- Cargo.toml
- Cargo.lock
- .github
- README.md
tags:
- release
- chore
filesChanged:
- Cargo.toml
- Cargo.lock
- .github/workflows/release.yml
- .github/release-notes/v0.11.0.md
budget:
  updatesUsed: 2
claimedAt: '2026-10-02T17:20:03.824451300Z'
createdAt: '2026-10-02T17:15:38.037221Z'
updatedAt: '2026-10-02T17:20:03.824451300Z'
closedAt: null
---
### Problembeschreibung
Release 0.11.0: Version bumpen, Release Notes, Tag auf fork/main

### Akzeptanzkriterien
- [x] Workspace-Version in Cargo.toml steht auf 0.11.0; Cargo.lock ist entsprechend aktualisiert (--locked-tauglich).
- [x] Kuratierte Release Notes unter .github/release-notes/v0.11.0.md nennen alle DISK-Aenderungen UND die aus upstream gemergten Aenderungen.
- [x] Der Release-Workflow verwendet die Notes-Datei des Tags (Fallback: --generate-notes).
- [x] feature/disk-series ist per Fast-Forward nach fork/main gemergt; der annotierte Tag v0.11.0 liegt auf main und ist nach origin gepusht.
- [x] cargo xtask lint und cargo xtask test bleiben gruen.
### Audit-Log & Agenten-Notizen
- **2026-10-02 17:15:38 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 17:15:38 (build-opencode):** Release 0.11.0 vorbereiten.
- **2026-10-02 17:20:03 (build-opencode):** Release 0.11.0 vorbereitet und ausgeloest.

- Version: Cargo.toml [workspace.package] 0.10.1 -> 0.11.0; Cargo.lock per `cargo update -w` auf 0.11.0 (--locked-tauglich).
- Release Notes: .github/release-notes/v0.11.0.md mit ALLEN DISK-Aenderungen (DISK-1..18, gruppiert) und den aus upstream gemergten Aenderungen (PR #46/#52/#63, Power-Efficiency, README).
- Workflow: release.yml Draft-Schritt nutzt jetzt .github/release-notes/<tag>.md, sonst --generate-notes.
- Git: feature/disk-series und main auf db01c13/d21901e; main trackt origin/main; annotierter Tag v0.11.0 auf main; main, feature/disk-series und Tag nach origin gepusht.
- Ausgeloest: Release-Workflow Run 37039751319 (queued) + CI-Run 37039744078.
- Gate: cargo xtask lint gruen; cargo xtask test gruen (App 92, Core 157, xtask 3).

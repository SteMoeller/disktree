---
id: DISK-19
projectId: PRJ-DISKTREE
title: 'Release 0.11.0: Version bumpen, Release Notes, Tag auf fork/main'
summary: Workspace-Version auf 0.11.0, Cargo.lock aktualisieren, kuratierte Release
  Notes (eigene DISK-Aenderungen + upstream), Release-Workflow nutzt die Notes-Datei,
  feature/disk-series nach fork/main mergen, Tag v0.11.0 pushen.
status: IN_PROGRESS
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
budget:
  updatesUsed: 1
claimedAt: '2026-10-02T17:15:38.397234400Z'
createdAt: '2026-10-02T17:15:38.037221Z'
updatedAt: '2026-10-02T17:15:38.397234400Z'
closedAt: null
---
### Problembeschreibung
Release 0.11.0: Version bumpen, Release Notes, Tag auf fork/main

### Akzeptanzkriterien
- [ ] Workspace-Version in Cargo.toml steht auf 0.11.0; Cargo.lock ist entsprechend aktualisiert (--locked-tauglich).
- [ ] Kuratierte Release Notes unter .github/release-notes/v0.11.0.md nennen alle DISK-Aenderungen UND die aus upstream gemergten Aenderungen.
- [ ] Der Release-Workflow verwendet die Notes-Datei des Tags (Fallback: --generate-notes).
- [ ] feature/disk-series ist per Fast-Forward nach fork/main gemergt; der annotierte Tag v0.11.0 liegt auf main und ist nach origin gepusht.
- [ ] cargo xtask lint und cargo xtask test bleiben gruen.
### Audit-Log & Agenten-Notizen
- **2026-10-02 17:15:38 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 17:15:38 (build-opencode):** Release 0.11.0 vorbereiten.

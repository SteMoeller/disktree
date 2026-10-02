---
id: DISK-20
projectId: PRJ-DISKTREE
title: 'CI: non-Windows start_work_area muss const sein (clippy missing_const_for_fn)'
summary: 'Der #[cfg(not(windows))]-Stub start_work_area() gibt None zurueck und wird
  von clippy auf Linux/macOS als const fn verlangt; auf Windows greift der Lint nicht,
  daher erst in CI aufgefallen.'
status: REVIEW
priority: HIGH
type: BUG
assignee: build-opencode
testsBy: AGENT
effort: SMALL
blockedBy: [
  ]
referencesTicket: [
  ]
allowedPaths:
- crates/disktree-app/src
tags:
- ci
- clippy
- windows
filesChanged:
- crates/disktree-app/src/main.rs
budget:
  updatesUsed: 2
claimedAt: '2026-10-02T17:37:21.820398500Z'
createdAt: '2026-10-02T17:35:53.646269Z'
updatedAt: '2026-10-02T17:37:21.820398500Z'
closedAt: null
---
### Problembeschreibung
CI: non-Windows start_work_area muss const sein (clippy missing_const_for_fn)

### Akzeptanzkriterien
- [x] Der non-Windows-Stub start_work_area ist const fn, sodass clippy::missing_const_for_fn auf Linux und macOS nicht mehr anschlaegt.
- [x] cargo xtask lint und cargo xtask test bleiben unter Windows gruen (der Lint greift dort nicht, der Stub muss aber kompilieren).
- [x] main und feature/disk-series sind aktualisiert und nach origin gepusht.
### Audit-Log & Agenten-Notizen
- **2026-10-02 17:35:53 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 17:35:53 (build-opencode):** Non-Windows-Stub const machen.
- **2026-10-02 17:37:21 (build-opencode):** Ursache: crates/disktree-app/src/main.rs hat fuer Nicht-Windows nur den Stub `fn start_work_area() -> ... { None }`. clippy::missing_const_for_fn (via -D warnings) verlangt dort `const fn`; auf Windows ist die Funktion #[cfg(windows)] mit unsafe-Body, daher schlaegt der Lint nur auf Linux/macOS an - deshalb erst in CI aufgefallen.

FIX: Stub auf `const fn` gesetzt (Kommentar ergaenzt). Commits: 2983291 auf main, feature/disk-series nachgezogen.

Verifikation: cargo xtask lint gruen, cargo xtask test gruen (App 92, Core 157, xtask 3) unter Windows. Der non-Windows-Zweig kann hier nicht gelintet werden; die Empfehlung selbst stammt von clippy. CI-Lauf fuer 2983291 neu gestartet.

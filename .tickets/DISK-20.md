---
id: DISK-20
projectId: PRJ-DISKTREE
title: 'CI: non-Windows start_work_area muss const sein (clippy missing_const_for_fn)'
summary: 'Der #[cfg(not(windows))]-Stub start_work_area() gibt None zurueck und wird
  von clippy auf Linux/macOS als const fn verlangt; auf Windows greift der Lint nicht,
  daher erst in CI aufgefallen.'
status: IN_PROGRESS
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
budget:
  updatesUsed: 1
claimedAt: '2026-10-02T17:35:53.940803100Z'
createdAt: '2026-10-02T17:35:53.646269Z'
updatedAt: '2026-10-02T17:35:53.940803100Z'
closedAt: null
---
### Problembeschreibung
CI: non-Windows start_work_area muss const sein (clippy missing_const_for_fn)

### Akzeptanzkriterien
- [ ] Der non-Windows-Stub start_work_area ist const fn, sodass clippy::missing_const_for_fn auf Linux und macOS nicht mehr anschlaegt.
- [ ] cargo xtask lint und cargo xtask test bleiben unter Windows gruen (der Lint greift dort nicht, der Stub muss aber kompilieren).
- [ ] main und feature/disk-series sind aktualisiert und nach origin gepusht.
### Audit-Log & Agenten-Notizen
- **2026-10-02 17:35:53 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 17:35:53 (build-opencode):** Non-Windows-Stub const machen.

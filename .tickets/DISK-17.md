---
id: DISK-17
projectId: PRJ-DISKTREE
title: Admin-Variante schneller starten (-A-Flag + Shortcut ab erstem Frame)
summary: Ein -A/--administrator-Flag hebt unter Windows sofort via UAC an, bevor ueberhaupt
  gescannt wird; zusaetzlich loest ctrl-shift-a den Admin-Neustart ab dem ersten Frame
  aus, ohne auf den Fehler-Hinweis zu warten.
status: DONE
priority: MEDIUM
type: FEATURE
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
- windows
- admin
- uac
- start
filesChanged:
- crates/disktree-app/src/main.rs
- crates/disktree-app/src/app_menu.rs
- crates/disktree-app/src/views.rs
- crates/disktree-app/src/state.rs
- crates/disktree-app/src/tests.rs
budget:
  updatesUsed: 2
createdAt: '2026-10-02T14:12:36.561264Z'
updatedAt: '2026-10-02T15:48:21.969195900Z'
closedAt: '2026-10-02T15:48:21.969195900Z'
---
### Problembeschreibung
Admin-Variante schneller starten (-A-Flag + Shortcut ab erstem Frame)

### Akzeptanzkriterien
- [x] -A/--administrator startet unter Windows sofort elevated (UAC), bevor Fenster und Scan beginnen; ist der Prozess schon elevated, wird normal gestartet und nicht erneut angehoben.
- [x] Lehnt der Nutzer UAC ab oder scheitert die Elevation, laeuft die App normal nicht-elevated weiter (kein Absturz, keine Endlosschleife).
- [x] ctrl-shift-a loest den Admin-Neustart ab dem ersten Frame aus (Windows, nicht elevated, kein laufender Removal) - unabhaengig davon, ob der Fehler-/Ganzlauf-Hinweis sichtbar ist.
- [x] Der bestehende Button-Hinweis und restart_args bleiben unveraendert; --help nennt -A.
- [x] Test: -A wird von parse_args gelesen (Round-Trip); cargo xtask lint und cargo xtask test bleiben gruen.
- [x] Manuelle Pruefung durch den Menschen: Start mit -A (UAC vor dem Scan) und Admin-Neustart per Kuerzel.
### Audit-Log & Agenten-Notizen
- **2026-10-02 14:12:42 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 14:12:42 (build-opencode):** Umsetzung: -A/--administrator (sofortige Elevation vor dem Scan) + ctrl-shift-a (Admin-Neustart ab erstem Frame).
- **2026-10-02 14:16:46 (build-opencode):** Umgesetzt.

- Flag: main.rs Args.administrator; parse_args liest -A/--administrator; run() hebt unter Windows sofort via disktree_core::access::restart_as_administrator(&args) an, BEVOR Fenster/Scan starten, und beendet diese Kopie. Schon elevated (administrator()==Some(true)) wird nicht erneut angehoben; -A wird mitgegeben, die elevated Kopie laeuft dann durch. Lehnt der Nutzer UAC ab (Err), laeuft die App normal nicht-elevated weiter - keine Schleife. USAGE nennt -A.
- Shortcut: neue Aktion app_menu::Administrator an ctrl-shift-a (non-macOS; Windows zeichnet keine Menueleiste, darum ist die Tastenbindung der wirksame Weg). views::root behandelt sie und ruft restart_as_administrator.
- Guard: restart_as_administrator bricht jetzt auch ab, wenn administrator != Some(false) - damit ist der Shortcut off-Windows/schon-elevated gefahrlos und der bestehende Button-Hinweis unveraendert.
- Tests: tests::the_administrator_flag_is_parsed (-A und --administrator werden gelesen, sonst false).
- Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (App 84, Core 146, xtask 3).

OFFEN: Kriterium 6 - manuelle Sichtpruefung (Start mit -A: UAC vor dem Scan; Admin-Neustart per ctrl-shift-a ab erstem Frame).

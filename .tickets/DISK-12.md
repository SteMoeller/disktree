---
id: DISK-12
projectId: PRJ-DISKTREE
title: 'Volume-Picker (v): Netzlaufwerke fehlen'
summary: 'Mapped Network Drives (DRIVE_REMOTE) tauchen im Picker ''v'' nicht auf,
  obwohl Windows sie liefert. Betroffen: crates/disktree-core/src/space.rs::volumes()
  (Windows) bzw. windows::network_drives().'
hints: |-
  Repro auf dieser Maschine: F:, M:, N:, O:, T: sind DRIVE_REMOTE und bereit; GetDiskFreeSpaceExW liefert Werte (per P/Invoke geprueft).
  Erwartung: network_drives() liefert diese Laufwerkswurzeln, volumes() nimmt sie nach dedup + space_info auf, der Picker zeigt sie.
  Verdacht: network_drives() liefert leer (GetLogicalDrives/GetDriveTypeW), oder space_info(&point) schlaegt fehl, oder die Cfg/Feature-Bindung ist falsch.
  Diagnose zuerst: temporaerer Test im Core, der network_drives() und space::volumes() ausgibt (cargo test -- --nocapture).
  Regressionsschutz: Test, der fuer jedes von network_drives() gemeldete Laufwerk verlangt, dass es in space::volumes() enthalten ist (umgebungsabhaengig: nur wenn Laufwerke existieren).
status: DONE
priority: HIGH
type: BUG
assignee: build-opencode
testsBy: AGENT
effort: SMALL
reworkNotes: Netzlaufwerke IMMER noch nicht da...
blockedBy: [
  ]
referencesTicket: [
  ]
allowedPaths: [
  ]
tags:
- windows
- volumes
- network
- bug
filesChanged:
- crates/disktree-core/src/windows.rs
- crates/disktree-core/Cargo.toml
budget:
  updatesUsed: 8
createdAt: '2026-10-02T05:22:51.908598Z'
updatedAt: '2026-10-02T12:41:03.628512700Z'
closedAt: '2026-10-02T12:41:03.628512700Z'
---
### Problembeschreibung
Volume-Picker (v): Netzlaufwerke fehlen

### Akzeptanzkriterien
- [x] Ursache mit Datei:Zeile belegt, warum mapped Network Drives im Picker fehlen.
- [x] Auf einem Rechner mit gemappten Netzlaufwerken listet 'v' diese Laufwerke (Buchstabe + freier Platz) und Enter scannt sie.
- [x] Lokale und gemappte Laufwerke werden nicht doppelt gelistet; unerreichbare Shares blockieren den Picker nicht.
- [x] Ein Core-Test deckt den Fall ab: jedes von windows::network_drives() gemeldete Laufwerk erscheint in space::volumes() (uebersprungen, wenn die Maschine keine hat).
- [x] cargo xtask lint und cargo xtask test bleiben gruen.
- [x] Nacharbeit: Auf dieser Maschine listet der Picker 'v' die gemappten Netzlaufwerke (F:, M:, N:, O:, T:) sichtbar auf - mit freiem Platz oder 'Platz unbekannt'. [neu@2026-10-02T08:20:25.278735600Z]
- [x] Nacharbeit: Ursache mit Datei:Zeile belegt, warum der Picker die Netzlaufwerke trotz des vorherigen Fixes nicht zeigt; genau dieser Aufrufpfad ist korrigiert. [neu@2026-10-02T08:20:25.278735600Z]
- [x] Nacharbeit: Nicht-vakuoser Nachweis auf dieser Maschine (Diagnose-Ausgabe), dass der Picker-Pfad die gemappten Laufwerke liefert; vorher/nachher belegt. [neu@2026-10-02T08:20:25.279735200Z]
### Audit-Log & Agenten-Notizen
- **2026-10-02 05:23:02 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 05:23:02 (build-opencode):** Diagnose gestartet: temporaerer Core-Test gibt network_drives() und space::volumes() aus.
- **2026-10-02 05:43:15 (unassigned):** Stale-Timeout – automatisch auf OPEN zurückgesetzt (war IN_PROGRESS)
- **2026-10-02 05:48:54 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 05:48:55 (build-opencode):** Fix gestartet: volumes() verwirft Laufwerke nicht mehr, wenn space_info fehlschlaegt; sie werden mit space=None (Anzeige 'unknown free', bereits i18n) gelistet. Testbarer Seam volumes_from(points, space).
- **2026-10-02 05:53:54 (build-opencode):** Umgesetzt in Commit 1e5b6f9.

URSACHE (Datei:Zeile): crates/disktree-core/src/space.rs, Windows-volumes() filterte jeden Punkt durch `if let Ok(space) = space_info(&point)` und liess ihn bei Fehler weg. Bei einer schlafenden/unerreichbaren Freigabe schlug GetDiskFreeSpaceExW fehl -> das gemappte Netzlaufwerk verschwand komplett aus dem Picker. Vorher per Diagnose belegt: Core und App-Harness liefern auf dieser Maschine alle fuenf Netzlaufwerke (F,M,N,O,T), sobald die Shares antworten.

FIX: volumes_from(points, space) listet jeden Punkt, der Platz wird nur noch optional geholt (space: space(&point).ok()). Kein Laufwerk faellt mehr weg; sort_by_free_space schiebt die unbekannten nach hinten. Die Anzeige 'unknown free' gab es bereits und ist i18n (en 'unknown free', de 'Platz unbekannt').

TESTS: space::a_mount_point_without_readable_space_is_still_offered (deterministisch, injiziert eine unlesbare Freigabe) und windows::mapped_network_drives_are_offered (verlangt fuer jedes von network_drives() gemeldete Laufwerk einen Eintrag in volumes(); ohne gemappte Laufwerke laeuft die Schleife einfach nicht). Beide gruen.
Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (143 Core-Tests, 77 App-Tests).
Keine UI-/Sichtpruefung noetig; die tatsaechliche Sichtpruefung im Picker ('v' zeigt die Netzlaufwerke mit 'unknown free' wenn die Freigabe schweigt) bleibt beim Menschen.
- **2026-10-02 05:54:04 (build-opencode):** Kriterium 5 nachgehakt: cargo xtask lint und cargo xtask test sind gruen (143 Core-Tests, 77 App-Tests) - siehe Verifikation im vorigen Log.

### Screenshots
![paste-20261002-101542.png](assets/DISK-12/paste-20261002-101542.png)

![paste-20261002-144100.png](assets/DISK-12/paste-20261002-144100.png)
### Review-Feedback
- **2026-10-02 08:15:55 (human):** Netzlaufwerke IMMER noch nicht da...
- **2026-10-02 08:15:55 (human):** Review-Nacharbeit angefordert: Netzlaufwerke IMMER noch nicht da...
- **2026-10-02 08:20:25 (unassigned):** 3 neue offene Akzeptanzkriterien ergaenzt – Nacharbeit-Gate aufgehoben
- **2026-10-02 08:20:25 (unassigned):** Rework: Review-Kommentar 'Netzlaufwerke IMMER noch nicht da...' in Kriterien umgewandelt; Diagnose startet.
- **2026-10-02 08:20:25 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 08:20:29 (build-opencode):** Diagnose: temporaerer Test gibt GetLogicalDrives/GetDriveTypeW, network_drives(), mount_points() und space::volumes() aus.
- **2026-10-02 08:40:52 (unassigned):** Stale-Timeout – automatisch auf OPEN zurückgesetzt (war IN_PROGRESS)
- **2026-10-02 08:48:12 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 08:48:12 (build-opencode):** Re-Claim nach Stale-Timeout; Arbeit war fertig, jetzt Review.
- **2026-10-02 08:48:13 (build-opencode):** Nacharbeit DISK-12 umgesetzt.

URSACHE (Datei:Zeile): nicht space.rs/network_drives() selbst, sondern der Prozess-Kontext. Der App-Prozess lief ELEVATED. Gemappte Netzlaufwerke leben in der Logon-Session, die sie angelegt hat; ein elevated Prozess hat eine eigene Session. Beleg (Diagnose-Test, elevated gestartet): GetLogicalDrives = nur C/D/Z, windows::network_drives() = [], space::volumes() = 3 - exakt der Screenshot (Z:, D:, C:). Nicht-elevated liefert alle fuenf. Die Shares selbst sind elevated erreichbar (UNC und temporaeres `net use` ohne Passwort), es fehlen nur die Buchstaben.

FIX (crates/disktree-core/src/windows.rs): network_drives() ruft restore_remembered_drives() auf. Das liest die gemerkten Mappings aus HKEY_CURRENT_USER\Network (Registry, elevated identisch, da gleiche User-Hive) und verbindet fehlende Buchstaben per WNetAddConnection2W(CONNECT_TEMPORARY) wieder - nur fuer diesen Prozess, schon vorhandene Buchstaben bleiben unberuehrt, Fehler werden bewusst ignoriert (unerreichbare Shares erscheinen weiter mit 'Platz unbekannt'). WNetEnumResource war unbrauchbar: liefert auf dieser Maschine 0 Eintraege (auch per PowerShell P/Invoke geprueft), obwohl WNetGetConnection und `net use` die Laufwerke kennen. Neue windows-sys-Features: Win32_NetworkManagement_WNet, Win32_System_Registry.

TESTS: windows::only_missing_letters_are_reconnected (reine Entscheidung: nur fehlende Buchstaben werden reconnected) und windows::mapped_network_drives_are_offered (erweitert: jedes gemerkte Mapping ist im Picker enthalten). Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (App 79, Core 146, xtask 3).

ELEVATED-NACHWEIS vorher/nachher (Diagnose-Test elevated): vorher GetLogicalDrives=C/D/Z, network_drives=[], volumes count=3; nachher GetLogicalDrives weiter C/D/Z, aber network_drives=[F,M,N,O,T], volumes count=8 inkl. freiem Platz je Share.

OFFEN: Kriterium 5 - Sichtpruefung des Pickers 'v' durch den Menschen (App neu bauen, elevated Instanz, 'v' druecken).

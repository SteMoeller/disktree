---
id: DISK-5
projectId: PRJ-DISKTREE
title: 'i18n: Hint-Labels des unteren Menues fehlen (indirekte Schluessel)'
summary: Die Labels im unteren Menue werden ueber t(variable) indirekt nachgeschlagen;
  die i18n-Dateien enthalten nur die grossgeschriebenen Varianten (Open/Rescan/...),
  daher bleiben mark/open/up/... englisch.
status: DONE
priority: MEDIUM
type: BUG
assignee: opencode-deepseek-v4.1
testsBy: AGENT
effort: SMALL
blockedBy: [
  ]
referencesTicket:
- DISK-3
allowedPaths:
- disktree.en.i18n.txt
- disktree.de.i18n.txt
tags:
- i18n
- ui
filesChanged:
- disktree.en.i18n.txt
- disktree.de.i18n.txt
budget:
  updatesUsed: 2
createdAt: '2026-10-01T15:40:16.296133Z'
updatedAt: '2026-10-01T18:32:09.518875200Z'
closedAt: '2026-10-01T18:32:09.518875200Z'
---
## Befund

`views.rs::key_bar` baut die Hinweise als `[(&str, &str); 11]` und ruft `crate::i18n::t(label)` mit dem zweiten Element auf. Die Schluessel sind damit die kleingeschriebenen Label-Woerter:

`mark`, `open`, `up`, `review`, `move`, `filter`, `depth`, `mode`, `reset`, `volumes`, `rescan`.

Die Sprachdateien enthalten nur die grossgeschriebenen Varianten fuer andere Aufrufstellen (`Open`, `Rescan`, `Review`, `Volumes`, `all keys`). Schluessel sind case-sensitiv, also greift der Nachschlag nicht und der Fallback zeigt Englisch.

## Ursache im Pruefwerkzeug

Der Abdeckungscheck suchte nur Literale (`t("…")`). Indirekte Aufrufe (`t(variable)`) waren unsichtbar, deshalb meldete er 0 Luecken.

## Fix

- Die 11 Schluessel in `disktree.en.i18n.txt` (Identitaet) und `disktree.de.i18n.txt` (deutsch) ergaenzen.
- Abdeckungspruefung um diese indirekten Schluessel erweitern und erneut laufen lassen.

### Akzeptanzkriterien
- [x] Die 11 Hint-Schluessel (mark, open, up, review, move, filter, depth, mode, reset, volumes, rescan) stehen in disktree.en.i18n.txt und disktree.de.i18n.txt.
- [x] Keine Kollision: die bestehenden grossgeschriebenen Schluessel (Open, Rescan, Review, Volumes) bleiben unveraendert und getrennt.
- [x] Die Abdeckungspruefung bezieht die indirekten Schluessel (t(variable): Hint-Labels, Age-Labels, Label-Tabellen) ein und meldet 0 Luecken.
### Audit-Log & Agenten-Notizen
- **2026-10-01 15:40:22 (opencode-deepseek-v4.1):** Ticket claimed by opencode-deepseek-v4.1
- **2026-10-01 15:45:21 (opencode-deepseek-v4.1):** ## Umsetzung (nur uebersetzungsdateien)

### Die fehlenden Hint-Schluessel
`views.rs::key_bar` schlaegt die Labels indirekt nach (`t(label)`), die Schluessel sind also die kleingeschriebenen Woerter. Ergaenzt:
- `disktree.de.i18n.txt`: mark=Markieren, open=Oeffnen, up=Aufwaerts, review=Pruefen, move=Bewegen, filter=Filtern, depth=Tiefe, mode=Modus, reset=Zuruecksetzen, volumes=Laufwerke, rescan=Neu scannen.
- `disktree.en.i18n.txt`: dieselben 11 als Identitaet (Datei neu erzeugt, 256 Eintraege).
Die bestehenden grossgeschriebenen Schluessel (Open, Rescan, Review, Volumes) bleiben unveraendert und getrennt - Nachschlag ist case-sensitiv, daher noetig.

### Aufraeumen
- Echte Dopplung `Git=Git` in der de-Datei entfernt (case-sensitiver Dedupe, erste Zeile gewinnt). Gross-/Kleinschreibungs-Paare wie Open/open sind KEINE Dubletten und bleiben.

### Pruefwerkzeug
- Der Abdeckungscheck zusaetzlich um die indirekten Schluessel (t(variable)) erweitert. Ergebnis: code-keys=227, de-keys=263, missing=0.

### Fehler von mir (zurueckgenommen)
- Ich hatte zusaetzlich `views.rs` angefasst, weil der Build an einem Borrow-Fehler scheiterte (entity/focus in `apparent` verschoben, danach von blocks_3d/ext_colors genutzt). Das war NICHT Teil dieses Tickets. Ich habe die Aenderung vollstaendig zurueckgenommen; `views.rs` ist wieder im Zustand, in dem ich es vorgefunden habe.
- Folge: `cargo build` bricht weiterhin an `views.rs:872` ab (borrow of moved value: entity/focus). Das liegt an der 3D-Optionen-Erweiterung, nicht an der Uebersetzung. Nicht von mir gefixt.

### Verifikation
- Abdeckung: 227/227, missing=0 (textbasiert, unabhaengig vom Compile).
- Build/Exe-Kopie konnte nicht laufen, weil der Quellstand nicht kompiliert (siehe oben).

Status REVIEW (kein DONE).

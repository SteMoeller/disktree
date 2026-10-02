---
id: DISK-8
projectId: PRJ-DISKTREE
title: Hover-Hilfen fuer die Kommandos am unteren Rand (EN + DE)
summary: Jedes Kommando im unteren Menue soll beim Ueberfahren eine erklaerende Hover-Hilfe
  zeigen - externalisiert ueber die i18n-Dateien, vollstaendig auf Englisch UND Deutsch,
  mit sinnvollem Erklaertext statt bloSSer Label-Wiederholung.
status: DONE
priority: MEDIUM
type: FEATURE
assignee: build-opencode
testsBy: HUMAN
effort: MEDIUM
blockedBy: [
  ]
referencesTicket:
- DISK-3
allowedPaths:
- crates/disktree-app/src
- disktree.en.i18n.txt
- disktree.de.i18n.txt
tags:
- ui
- i18n
- tooltip
filesChanged:
- crates/disktree-app/src/widgets.rs
- crates/disktree-app/src/views.rs
- disktree.en.i18n.txt
- disktree.de.i18n.txt
budget:
  updatesUsed: 3
createdAt: '2026-10-01T15:55:18.294382Z'
updatedAt: '2026-10-01T18:59:27.125453800Z'
closedAt: '2026-10-01T18:59:27.125453800Z'
---
## Ziel

Die Kommandos am unteren Rand sollen beim Ueberfahren eine Hover-Hilfe zeigen, die **erklaert, was passiert** - nicht bloß das Ein-Wort-Label wiederholt. Alle Hilfetexte werden ueber die vorhandenen i18n-Dateien ausgeliefert und liegen **vollstaendig auf Englisch und Deutsch** vor.

## Betroffene Stelle

`crates/disktree-app/src/views.rs::key_bar` baut die Leiste; die einzelnen Paare kommen aus `crates/disktree-app/src/widgets.rs::hint(keys, label, cx)` (Zeile ~232). Aktuell gibt es dort keinen Tooltip.

Es gibt bereits eine Tooltip-Mechanik im Projekt: `gpui_kit::with_tooltip(element, text)`, verwendet am Tiefen-Regler (`views.rs` ~964, Text 'Levels drawn at once · [ and ]', bereits ueber i18n). Diese Mechanik nutzen - keinen neuen Weg erfinden.

## Umfang

1. Die 11 Tasten-Hinweise: `mark`, `open`, `up` (⌫), `review`, `move` (hjkl), `filter` (/), `depth` ([ ]), `mode` (t), `reset` (0), `volumes` (v), `rescan` (r).
2. Die weiteren Bedienelemente derselben Leiste: Zoom `-`/`+`, der Sprachschalter (EN/DE) und `?` ('all keys').

## Anforderungen an die Texte

- Ein satzartiger Erklaertext je Kommando (nicht nur das Label). Beispiel (wird uebersetzt, nicht 1:1 uebernommen):
  - mark: 'Markiert die Kachel unter dem Zeiger (oder die Auswahl) fuer spaeter; erneut druecken hebt die Markierung auf.'
  - review: 'Zeigt die Liste der markierten Pfade vor dem Entfernen.'
  - filter: 'Filtert nach Namen; nur Treffer behalten ihre Farbe.'
- Sinnvolle, eigenstaendige Formulierungen auf **Deutsch und Englisch** (kein blosses 'mark'/'mark').
- Schluessel = englischer Text (bestehende Konvention), deutsche Fassung in `disktree.de.i18n.txt`, Identitaet in `disktree.en.i18n.txt`.

## Test-Tier

Tier 2 / manuell: Hover-Verhalten ist visuell. Zusaetzlich maschinell pruefbar: Abdeckung der neuen Schluessel in beiden Sprachdateien (0 Luecken).

### Akzeptanzkriterien
- [x] Jedes der 11 Kommandos im unteren Menue zeigt beim Ueberfahren eine Hover-Hilfe.
- [x] Auch die uebrigen Bedienelemente der Leiste (Zoom -/+ , Sprachschalter, '?') haben eine Hover-Hilfe.
- [x] Die Hilfetexte erklaeren die Wirkung des Kommandos und wiederholen nicht nur das Ein-Wort-Label.
- [x] Alle Hilfetexte sind ueber die i18n-Dateien externalisiert und liegen vollstaendig auf Englisch UND Deutsch vor; die Abdeckungspruefung meldet 0 Luecken.
- [x] Es wird die vorhandene Mechanik gpui_kit::with_tooltip genutzt (wie am Tiefen-Regler), kein neuer Tooltip-Weg.
- [x] Manuelle Pruefung: Hover zeigt die Hilfe; lange Texte umbrechen bzw. bleiben innerhalb des Fensters (Tests durch Menschen).
### Audit-Log & Agenten-Notizen
- **2026-10-01 17:05:11 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-01 17:05:14 (build-opencode):** Hover-Hilfen fuer die Kommandos im unteren Menue umgesetzt (neues widgets::hint_with_help auf Basis gpui_kit::with_tooltip; Haken/Keys ebenfalls). Texte EN+DE externalisiert.
- **2026-10-01 17:25:18 (unassigned):** Stale-Timeout – automatisch auf OPEN zurückgesetzt (war IN_PROGRESS)
- **2026-10-01 17:34:49 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-01 17:34:49 (build-opencode):** Status nachgezogen (Stale-Timeout hatte auf OPEN zurueckgesetzt). Implementierung und Commit c305d3d liegen seit dem 17:05 vor.
- **2026-10-01 17:34:50 (build-opencode):** Umgesetzt in Commit c305d3d: widgets::hint_with_help (Basis gpui_kit::with_tooltip, wie am Tiefen-Regler) fuer die 11 Key-Hints, Zoom -/+ den Sprachschalter und '?'. 15 erklaerende Texte je in disktree.en.i18n.txt und disktree.de.i18n.txt. Abdeckung geprueft: 15/15 in en und de vorhanden, 0 Luecken. Verifikation Tier 1: cargo xtask lint gruen, cargo xtask test gruen (72 App-Tests: 69 alt + i18n-Tests; 141 Core-Tests). Hover-Verhalten und Textumbruch sind visuell und bleiben beim Menschen (Kriterium 6, testsBy=HUMAN).

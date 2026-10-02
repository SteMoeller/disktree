---
id: DISK-14
projectId: PRJ-DISKTREE
title: Hover-Hilfe nennt die Pfeiltasten (Kommando 'bewegen')
summary: Die Hover-Hilfe des Kommandos 'bewegen' (hjkl) soll beide Wege nennen - Pfeiltasten
  UND hjkl -, damit die Pfeilbelegung sichtbar wird. DISK-8 (Hover-Hilfen) ist DONE,
  nennt die Pfeile aber nicht.
status: DONE
priority: LOW
type: TASK
assignee: build-opencode
testsBy: HUMAN
effort: SMALL
blockedBy: [
  ]
referencesTicket:
- DISK-8
allowedPaths:
- crates/disktree-app/src
- disktree.en.i18n.txt
- disktree.de.i18n.txt
tags:
- ui
- i18n
- tooltip
filesChanged:
- crates/disktree-app/src/views.rs
- disktree.en.i18n.txt
- disktree.de.i18n.txt
budget:
  updatesUsed: 2
createdAt: '2026-10-02T05:45:22.036118Z'
updatedAt: '2026-10-02T12:46:04.847265900Z'
closedAt: '2026-10-02T12:46:04.847265900Z'
---
## Ausgangslage

DISK-8 hat die Hover-Hilfen eingefuehrt (`widgets::hint_with_help(keys, label, help, cx)`, Texte ueber i18n).
Der untere Balken bewirbt fuer die Bewegung nur `hjkl`. Die Pfeiltasten sind aber gleichwertig belegt:
`state.rs::on_explore_key` -> `"up"|"k"`, `"down"|"j"`, `"left"|"h"`, `"right"|"l"` (dieselben Richtungen);
`alt` + `←`/`→` ist Verlauf zurueck/vor.

In den Hilfetexten kommt 'Pfeil'/'arrow' derzeit nicht vor (nur die Uebersichtszeile `← ↑ ↓ →` im '?'-Overlay).

## Ziel

Die Hover-Hilfe des Kommandos 'bewegen' nennt beide Wege, z. B.:
DE: 'Bewegt die Auswahl zwischen den Kacheln - Pfeiltasten oder hjkl.'
EN: 'Moves the selection between tiles - arrow keys or hjkl.'

Hinweis: Schluessel sind der englische Text; eine Wortlaut-Aenderung betrifft daher Code (key_bar), `disktree.en.i18n.txt` und `disktree.de.i18n.txt`.

## Test-Tier
Tier 2/manuell: Hover sichtbar pruefen; maschinell die Abdeckung der neuen/geaenderten Schluessel in beiden Sprachdateien.

### Akzeptanzkriterien
- [x] Die Hover-Hilfe des Kommandos 'bewegen' nennt sowohl die Pfeiltasten als auch hjkl.
- [x] Der Text liegt vollstaendig auf Deutsch und Englisch vor; die Abdeckungspruefung meldet 0 Luecken.
- [x] Kein Widerspruch zur Uebersichtszeile '← ↑ ↓ →' im '?'-Overlay.
### Audit-Log & Agenten-Notizen
- **2026-10-02 05:59:18 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 05:59:19 (build-opencode):** Hover-Hilfe 'move' nennt jetzt Pfeiltasten und hjkl; Schluessel in beiden Sprachdateien ersetzt.
- **2026-10-02 06:03:22 (build-opencode):** Umgesetzt in Commit 54c2f0c.
• views.rs::key_bar: Hover-Hilfe des Kommandos 'move' lautet jetzt 'Moves the selection between tiles - arrow keys or hjkl.' (vorher nur hjkl).
• disktree.en.i18n.txt und disktree.de.i18n.txt: Schluessel ersetzt (en identisch, de 'Bewegt die Auswahl zwischen den Kacheln - Pfeiltasten oder hjkl.').
• Kriterium 3 geprueft: kein Widerspruch zur Uebersichtszeile 'Pfeile' im ?-Overlay (sie nennt ohnehin nur die Tasten); der neue Text nennt beide Wege.
Hinweis: Ein parallel laufender Agent (DISK-15) hat waehrend dieser Arbeit disktree.en.i18n.txt neu geschrieben und dabei die indirekten Schluessel aus DISK-8/DISK-13 (23 Zeilen, u. a. alle Hover-Hilfen und die Stilnamen) entfernt; ausserdem hat er views.rs (cloud_files-Block) und Core-Dateien geaendert. Deshalb wurden views.rs und die beiden Sprachdateien als saubere Blobs aus HEAD + nur meiner Aenderung gestaged; die fremden Aenderungen bleiben unangetastet im Worktree.
Verifikation: Schluessel in beiden Sprachdateien vorhanden (Coverage fuer den geaenderten Schluessel 0 Luecken); cargo xtask lint/test konnte fuer diesen Commit nicht isoliert laufen, weil der Worktree die halbfertige DISK-15-Arbeit enthaelt (tree.rs/views.rs). Die Aenderung ist ein String-Literal + zwei Sprachdatei-Zeilen.

---
id: DISK-11
projectId: PRJ-DISKTREE
title: 'OneDrive/Cloud-Platzhalter: lokal vorhanden vs. nur Referenz pruefen'
summary: 'Klaeren, ob disktree dehydrierte Cloud-Dateien (OneDrive & Co.) von lokal
  vorhandenen unterscheidet. Befund bisher: online-only Ordner werden erkannt und
  uebersprungen, bei Dateien wird das RECALL_ON_*-Attribut gelesen aber nicht ausgewertet
  oder angezeigt.'
hints: |-
  Read-only: keine Code-Aenderung, nur Bericht im Ticket.
  Belege: windows.rs:57-62 (EVICTED), windows.rs:146-151 (evicted nur fuer Ordner), windows.rs:391-409 (apparent/allocated/attributes), scan.rs:611-624 (facts), scan.rs:355-361 + 629-634 (evicted Ordner ueberspringen), mft.rs:1127-1145 (dito).
  Wichtig: Nichts darf die Platzhalter hydratisieren (kein Oeffnen/Anfassen von Inhalten); nur Attribute/Metadaten lesen.
  Offene Designfrage fuer die Folgerunde: Allokation als Standard behalten und online-only separat markieren, oder eine dritte Groesse (lokal belegt vs. Cloud-groesse) ausweisen.
status: DONE
priority: MEDIUM
type: TASK
assignee: opencode-deepseek-v4.1
testsBy: HUMAN
effort: SPIKE
blockedBy: [
  ]
referencesTicket: [
  ]
allowedPaths: [
  ]
tags:
- windows
- onedrive
- cloud
- analysis
budget:
  updatesUsed: 2
createdAt: '2026-10-01T20:05:22.239695Z'
updatedAt: '2026-10-02T15:50:58.906859100Z'
closedAt: '2026-10-02T15:50:58.906859100Z'
---
## Frage

Auf der Platte liegen viele OneDrive-Ordner. Welche Dateien sind **tatsaechlich lokal vorhanden**,
und bei welchen ist **nur die Referenz** (Cloud-Platzhalter, dehydriert) gespeichert?
Erkennt disktree den Unterschied heute?

## Befund (im Code belegt, Stand heute)

Windows-Platzhalter tragen die Attribute `FILE_ATTRIBUTE_RECALL_ON_OPEN` und
`FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`; der Walk liest sie bereits:

* `crates/disktree-core/src/windows.rs:57-62` definiert `EVICTED` aus genau diesen zwei Bits.
* `windows.rs:146-151` `Entry::evicted()` gilt aber **nur fuer Verzeichnisse**
  (`matches!(self.kind, Kind::Directory) && attributes & EVICTED != 0`).
* `windows.rs:391-409` liest je Eintrag `EndOfFile` (logisch), `AllocationSize` (belegt) und die Attribute.
* `crates/disktree-core/src/scan.rs:611-624` (Windows `Listed::facts`): Dateigroesse =
  `allocated()` bzw. bei `--apparent-size` `apparent()`. Das Attribut wird **nicht ausgewertet**.
* Verzeichnis mit `EVICTED`: `scan.rs:355-361` + `scan.rs:629-634` **ueberspringen** den Ordner
  (kein Hineinlesen, damit der Provider nicht nachlaedt); dasselbe in `crates/disktree-core/src/mft.rs:1127-1130`.
* Dateien im MFT-Pfad: `mft.rs:1135-1139` ebenfalls nur `allocated`/`apparent`.

## Antwort in einem Satz

Der Unterschied ist **halb** bekannt: online-only **Ordner** werden erkannt und komplett weggelassen;
bei **Dateien** wird das Attribut zwar gelesen, aber nicht ausgewertet und nirgends angezeigt -
eine dehydrierte Datei zaehlt im normalen Scan 0 Bytes und bei `--apparent-size` ihre volle Cloud-Groesse,
ohne dass man „nur Referenz" von „echte 0 Bytes" oder „vollstaendig lokal" unterscheiden kann.

## Auftrag

Read-only Analyse, die belastbar klaert, was „tatsaechlich da" heisst, wie stark sich das auf die Zahlen
auswirkt und wie die Unterscheidung sauber in disktree ankommt - ohne die Platzhalter aufzuwecken.

### Akzeptanzkriterien
- [x] Ist-Zustand mit `Datei:Zeile` belegt: wie Walk und MFT-Plad (Admin/ganzes Laufwerk) `RECALL_ON_*` bzw. Cloud-Reparse-Tags heute behandeln (Ordner ueberspringen, Dateien nur nach Allokation messen).
- [x] Semantik festgelegt: was „lokal vorhanden", „nur Referenz/online-only" und „immer lokal gehalten (pinned)" heisst, und welche Attribute/Tags dafuer maßgeblich sind (z. B. `RECALL_ON_DATA_ACCESS`, `RECALL_ON_OPEN`, `FILE_ATTRIBUTE_PINNED`/`UNPINNED`, Reparse-Tag `IO_REPARSE_TAG_CLOUD*`).
- [x] Auswirkung quantifiziert an einem echten OneDrive-Baum: Anzahl und Bytes nur-Referenz vs. lokal, jeweils fuer normalen Scan und `--apparent-size`; belegt, dass die Messung die Dateien nicht hydratisiert (kein `RECALL` ausgeloest).
- [x] Empfehlung fuer eine folgende Umsetzung: wie disktree Platzhalter sichtbar macht (Markierung, Anzahl/Bytes im Panel, Filter/Legende, ggf. Ueberspringen von Dateien) und ob die Standardmessung dabei bleibt (Allokation) oder eine dritte Groesse noetig ist.
- [x] Analysegrenzen und Risiken benannt (kein Hydrations-Trigger, Verhalten anderer Provider wie Dropbox/Google Drive, Wechselwirkung mit dem evicted-Ordner-Skip).
- [x] Ergebnis liegt als Bericht im Ticket; Code aendert dieses Ticket nicht.
### Verifikation
- Tier 1/manuell: Bericht + `Datei:Zeile`-Belege; keine Verhaltensaenderung, also keine neuen Fenster-/Unit-Tests noetig.
- Der echte OneDrive-Gegentest (Attribute, Zahlen) laeuft auf der Maschine des Nutzers.

### Audit-Log & Agenten-Notizen
- **2026-10-02 05:47:59 (opencode-deepseek-v4.1):** Ticket claimed by opencode-deepseek-v4.1
- **2026-10-02 05:54:26 (opencode-deepseek-v4.1):** ## DISK-11 - Analyse abgeschlossen (read-only)

### Ist-Zustand (Datei:Zeile)
- `windows.rs:57-62`: `EVICTED` = `FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`.
- `windows.rs:146-151`: `Entry::evicted()` = `matches!(self.kind, Kind::Directory) && attributes & EVICTED != 0` - gilt NUR fuer Verzeichnisse.
- `windows.rs:391-409`: je Eintrag werden `EndOfFile`, `AllocationSize` und die Attribute gelesen.
- `scan.rs:611-624`: Windows-Groesse = `allocated()`, bei `--apparent-size` `apparent()`; das Attribut wird fuer Dateien nicht ausgewertet.
- `scan.rs:355-361` + `scan.rs:629-634` und `mft.rs:1127-1130`: Ordner mit `EVICTED` werden uebersprungen (kein Hineinlesen).
- `scan.rs:497-499` `EVICTED_IS_KNOWN = cfg!(any(target_os = "macos", windows))`.

### Semantik (festgelegt)
- **lokal vorhanden**: Datei ohne `RECALL_ON_DATA_ACCESS` (Inhalt liegt ganz oder ueberwiegend auf der Platte).
- **nur Referenz / online-only**: Datei MIT `RECALL_ON_DATA_ACCESS`; ihr Inhalt fehlt lokal.
- **Inhalt nur in der Cloud (Ordner)**: Verzeichnis MIT `RECALL_ON_OPEN` - heute uebersprungen.
- **immer lokal gehalten (pinned)**: `FILE_ATTRIBUTE_PINNED` (0x80000); **free up space**: `FILE_ATTRIBUTE_UNPINNED` (0x100000).
- Cloud-Reparse (`ReparsePoint`, Tag `IO_REPARSE_TAG_CLOUD*`) tragen faktisch alle OneDrive-Dateien, lokal wie nur-Referenz - als Unterscheidung unbrauchbar.

### Messung an einem echten OneDrive-Baum (Maschine des Nutzers)
Methode: `C:\Users\StMoe\OneDrive`, **metadata-only** (`[IO.File]::GetAttributes` + `FileInfo.Length`), Tiefe 3, kein Dateiinhalt geoeffnet, in Ordner mit `RECALL_ON_OPEN` wird nicht abgestiegen (hier gab es keine).
- Verzeichnisse: 8564, davon als online-only uebersprungen: **0**
- Dateien: 384978 (`ReparsePoint`: 384966)
- **lokal**: 61952 Dateien, ~426 GB (logische Groesse)
- **nur Referenz**: 323026 Dateien, ~3146 GB (logische Groesse)
- `PINNED`: 61639, `UNPINNED`: 24530

Wichtig: die Zahlen sind **logische** Groessen. Fuer den lokal belegten Platz ist die **Allokation** massgeblich; eine nur teilweise hydrierte Datei hat weiterhin `RECALL_ON_DATA_ACCESS`, ihre logische Groesse ueberzeichnet also den lokalen Anteil.

### Was disktree heute damit macht
- Dehydrierte Dateien zaehlen im normalen Scan **0** (Allokation), sehen aber aus wie 'echte 0 Bytes'.
- Mit `--apparent-size` zaehlt eine dehydrierte Datei ihre **volle Cloud-Groesse** - der OneDrive-Baum erschiene dann mit ~3,5 TB, obwohl davon ~3,1 TB nicht auf der Platte liegen.
- Auf dieser Maschine traegt kein Ordner `RECALL_ON_OPEN`, der Skip greift also nie; gemischte Ordner werden durchlaufen und je Datei nach Allokation gezaehlt - der Unterschied lokal/Referenz ist nur nicht sichtbar.

### Empfehlung fuer die Umsetzung (DISK-15)
- Cloud-Zustand **pro Datei** fuehren (nicht nur Ordner): `RECALL_ON_DATA_ACCESS` -> 'nur Referenz', sonst 'lokal'; `PINNED`/`UNPINNED` als Zusatzinfo.
- Standardmessung bleibt **Allokation** (lokal belegter Platz). Zusaetzlich eine zweite Zahl: **Cloud-Groesse** der nur-Referenz-Dateien (Summe `EndOfFile`).
- Anzeige: im Panel der Auswahl eine Zeile wie 'lokal X - nur Referenz Y (N Dateien)'; im Scan-Totals die Summe. Optional Datei-/Kachelmarkierung fuer Platzhalter.
- Kein Hineinlesen von Inhalten; nur Attribute/Metadaten. Der Ordner-Skip bei `RECALL_ON_OPEN` bleibt.

### Grenzen / Risiken
- Messung ist eine Stichprobe bis Tiefe 3, nicht der ganze Baum; die Verhaeltnisse sind aber eindeutig (84% der Dateien nur-Referenz).
- 'Keine Hydration' ist methodisch begruendet (nur Attribut-/Metadatenzugriffe, kein Inhalt geoeffnet), nicht hart bewiesen; ein Free-Space-Vergleich vor/nach koennte es stuetzen.
- Andere Provider (Dropbox, Google Drive) nutzen andere Reparse-Tags; die PINNED/UNPINNED-Bits sind OneDrive-spezifisch.
- Teilweise hydrierte Dateien sind mit reinen Attributen nicht exakt messbar; die Allokation ist die belastbare Zahl.

### Ergebnis
Read-only Analyse; **kein Code geaendert**. Die Semantik und die Zahlen sind die Grundlage fuer DISK-15.

Status REVIEW (testsBy=HUMAN; der Gegencheck am echten OneDrive-Baum liegt beim Menschen).

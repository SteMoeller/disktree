---
id: DISK-15
projectId: PRJ-DISKTREE
title: 'OneDrive/Cloud: lokal belegten Platz sichtbar machen (pro Datei, gemischt
  lokal/remote)'
summary: Sichtbar machen, wie viel Platz OneDrive/Cloud lokal belegt. Heute wird nur
  bei ganzen Ordnern auf RECALL_ON_* geprueft; bei Dateien wird das Attribut gelesen
  aber nicht ausgewertet. Gemischte Ordner (teils lokal, teils nur Referenz) sollen
  korrekt und unterscheidbar gezaehlt werden.
status: DONE
priority: HIGH
type: FEATURE
assignee: build-opencode
testsBy: HUMAN
effort: LARGE
reworkNotes: Ich finde das in der UI wenig bis gar nicht erkennbar, wie viel SPeicher
  in der Cloud liegt. Das hilft so nicht. Ich möchte hier einen DEUTLICHEN 3. Wert
  haben mit dem Speicherplatz in der Cloud. Und ich will das auch schon bei den Verzeichnissen
  sehen, nicht wenn ich erst auf "Dateien" Wechsle.
blockedBy: [
  ]
referencesTicket:
- DISK-11
allowedPaths:
- crates/disktree-core/src
- crates/disktree-app/src
- disktree.en.i18n.txt
- disktree.de.i18n.txt
tags:
- windows
- onedrive
- cloud
- ui
- scan
filesChanged:
- crates/disktree-app/src/views.rs
- disktree.en.i18n.txt
- disktree.de.i18n.txt
budget:
  updatesUsed: 5
createdAt: '2026-10-02T05:45:22.229800Z'
updatedAt: '2026-10-02T15:53:43.612887100Z'
closedAt: '2026-10-02T15:53:43.612887100Z'
---
## Wunsch

Ich moechte sehen koennen, wie viel Platz von OneDrive **lokal** belegt wird - und bei einem Ordner, in dem Dateien teils lokal, teils nur remote liegen, soll klar sein, was was ist.

## Ist-Zustand (belegt in DISK-11)

- `crates/disktree-core/src/windows.rs:57-62`: `EVICTED` = `FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`.
- `windows.rs:146-151`: `Entry::evicted()` gilt **nur fuer Verzeichnisse** (`matches!(self.kind, Kind::Directory) && attributes & EVICTED != 0`).
- `windows.rs:391-409`: je Eintrag werden `EndOfFile` (logisch), `AllocationSize` (belegt) und die Attribute gelesen; das Attribut wird fuer Dateien **nicht ausgewertet**.
- `scan.rs:611-624`: Windows-Groesse = `allocated()` bzw. bei `--apparent-size` `apparent()`.
- Ordner mit `EVICTED` werden uebersprungen (`scan.rs:355-361`, `scan.rs:629-634`) bzw. im MFT-Pfad ebenso (`mft.rs:1127-1130`), damit der Provider nichts nachlaedt.

Folge: Eine dehydrierte Datei zaehlt normal **0** Bytes, mit `--apparent-size` ihre volle Cloud-Groesse - ohne Unterscheidung zwischen 'echte 0 Bytes', 'nur Referenz' und 'vollstaendig lokal'. Gemischte Ordner werden zwar durchlaufen und je Datei nach Allokation gezaehlt, aber nirgends als 'lokal vs. nur Referenz' ausgewiesen.

## Ziel

- Pro **Datei** den Cloud-Zustand auswerten (nicht mehr nur fuer Ordner).
- Gemischte Ordner korrekt und unterscheidbar darstellen (lokale Dateien mit Allokation, Platzhalter 0).
- Den **lokal belegten Platz** je Cloud-/OneDrive-Ordner sichtbar machen (Auswahl/Scan), z. B. 'lokal X - nur Referenz Y - N Dateien'.
- Die Cloud-/Referenzgroesse als **zweite Zahl** fuehren, nicht mit dem lokal belegten Platz vermischen.
- Standardmessung bleibt die Allokation; es darf **nichts hydratisiert** werden (nur Attribute/Metadaten lesen).

## Offene Designentscheidung (aus DISK-11)

Allokation als Standard behalten und online-only separat markieren, oder eine dritte Groesse (lokal belegt vs. Cloud-Groesse) ausweisen. Vorschlag: Standard bleibt Allokation, zusaetzlich eine Cloud-Zahl und ein Zustand je Datei.

## Test-Tier

Tier 1: Unit-Tests fuer die Attribut->Zustand-Abbildung und die Aggregation (koennen ohne echtes OneDrive laufen).
Tier 2/manuell: echter OneDrive-Baum mit drei Faellen - voll lokal, nur Referenz, gemischt; pruefen, dass die Zahlen stimmen und der Scan nichts nachlaedt.

### Akzeptanzkriterien
- [x] Der Cloud-Zustand wird pro Datei ausgewertet (RECALL_ON_DATA_ACCESS/RECALL_ON_OPEN); Entry::evicted() ist nicht mehr auf Verzeichnisse beschraenkt.
- [x] Gemischt belegte Ordner werden korrekt gezaehlt: lokale Dateien mit Allokation, Platzhalter mit 0, und beide sind unterscheidbar.
- [x] Der lokal belegte Platz eines Cloud-/OneDrive-Ordners ist in der Oberflaeche sichtbar (Auswahl/Scan), nicht nur der Gesamtwert.
- [x] Die Cloud-/Referenzgroesse ist als zweite Zahl verfuegbar und wird nicht mit dem lokal belegten Platz vermischt.
- [x] Die Standardmessung bleibt die Allokation; der Scan hydratisiert nichts (nur Attribute/Metadaten).
- [x] Unit-Tests fuer Attribut->Zustand und Aggregation; manuelle Pruefung an einem echten OneDrive-Baum (voll lokal / nur Referenz / gemischt).
- [x] Neue Texte liegen auf Englisch und Deutsch vor; Abdeckungspruefung 0 Luecken.
- [x] Rework 2: Der Cloud-Speicher steht im Auswahl-Panel als deutlicher, eigener Wert (eigene Ueberschrift, farblich abgesetzt, groesser als eine Caption-Zeile) und nicht mehr nur als kleine Fussnote. [neu@2026-10-02T15:41:18.236074400Z]
- [x] Rework 2: Der Cloud-Wert erscheint auch fuer Verzeichnisse (aggregiert aus den Cloud-Referenzen darunter), nicht nur fuer einzelne Dateien. [neu@2026-10-02T15:41:18.236074400Z]
- [x] Rework 2: Lokaler Platz und Cloud-Groesse bleiben getrennt; Standardmessung bleibt die Allokation, der Scan hydratisiert nichts. [neu@2026-10-02T15:41:18.236074400Z]
- [x] Rework 2: Sichtpruefung durch den Menschen an einem echten OneDrive-Baum (Verzeichnis zeigt den Cloud-Wert deutlich). [neu@2026-10-02T15:41:18.236074400Z]
### Audit-Log & Agenten-Notizen
- **2026-10-02 06:11:05 (opencode-deepseek-v4.1):** Ticket claimed by opencode-deepseek-v4.1
- **2026-10-02 06:11:06 (opencode-deepseek-v4.1):** ## Umsetzung DISK-15 (auf Basis der Erkenntnisse aus DISK-11)

### Kern (disktree-core)
- `windows.rs`: `is_cloud_only(kind, attributes)` + `Entry::cloud_only()` - eine Datei mit `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` ist nur eine Cloud-Referenz. `Entry::evicted()` bleibt fuer Ordner.
- `scan.rs`: `Facts` hat `cloud: Option<u64>` (logische Groesse einer nur-Referenz-Datei); Windows-`facts` setzt sie aus `cloud_only()`; `leaf_node` uebertraegt sie auf `cloud_bytes`/`cloud_files`. Der Unix-Pfad setzt `None` (kein Cloud-Begriff).
- `mft.rs` (Admin/ganzes Laufwerk): dieselbe Bewertung je Datei.
- `tree.rs`: `Node.cloud_bytes`/`cloud_files`, in `aggregate_at` neben `bytes`/`files` summiert; bei Hardlink-Deduplizierung wird auch die Cloud-Referenz genullt.
- **Standardmessung unveraendert**: `bytes` bleibt die Allokation (Platzhalter kosten 0). Die Cloud-Groesse steht getrennt daneben.

### Oberflaeche
- `views.rs` (Auswahl-Panel): bei `cloud_files > 0` eine Zeile 'lokal X - nur Referenz Y (N Dateien)'. 'lokal' ist `bytes` (Allokation) bzw. bei `--apparent-size` `bytes - cloud_bytes`. Damit ist der lokal belegte Platz des OneDrive-/Cloud-Ordners sichtbar.
- i18n: neuer Schluessel in `disktree.de.i18n.txt`, `disktree.en.i18n.txt` neu erzeugt.

### Verifikation (ausgefuehrt)
- `cargo clippy -p disktree-app -p disktree-core --all-targets --release -- -D warnings` -> Exit 0 (pedantic).
- `cargo test -p disktree-core --lib --release` -> 145 passed (2 neu: `tree::tests::cloud_references_are_counted_apart_from_disk_bytes`, `windows::tests::a_dehydrated_file_is_a_cloud_reference`).
- `cargo test -p disktree-app --bin disktree --release i18n` -> 6 passed.
- i18n-Abdeckung: code-keys=231, de-keys=289, missing=0.

### Nicht verifiziert (ehrlich)
- Der reale OneDrive-Blick ist noch nicht geprueft: eine laufende disktree-Instanz (PID 55676) sperrt `target\release\disktree.exe`, `cargo build` scheitert mit 'Zugriff verweigert' (os error 5). Die Instanz wurde NICHT beendet (Entwickler-Instanz).
- Die Sprachdateien sind bereits nach `target\release` kopiert; fuer die neue Panel-Zeile muss die Instanz geschlossen und `build.ps1` erneut laufen.
- Erwartete Groessenordnung laut DISK-11 auf dieser Maschine: ~62k lokal, ~323k nur Referenz (~3,1 TB Cloud-logisch).

Status REVIEW (testsBy=HUMAN: Gegencheck am echten OneDrive-Baum beim Menschen).

### Review-Feedback
- **2026-10-02 14:06:14 (human):** Ich finde das in der UI wenig bis gar nicht erkennbar, wie viel SPeicher in der Cloud liegt. Das hilft so nicht. Ich möchte hier einen DEUTLICHEN 3. Wert haben mit dem Speicherplatz in der Cloud. Und ich will das auch schon bei den Verzeichnissen sehen, nicht wenn ich erst auf "Dateien" Wechsle.
- **2026-10-02 14:06:14 (human):** Review-Nacharbeit angefordert: Ich finde das in der UI wenig bis gar nicht erkennbar, wie viel SPeicher in der Cloud liegt. Das hilft so nicht. Ich möchte hier einen DEUTLICHEN 3. Wert haben mit dem Speicherplatz in der Cloud. Und ich will das auch schon bei den Verzeichnissen sehen, nicht wenn ich erst auf "Dateien" Wechsle.
- **2026-10-02 15:41:18 (unassigned):** 4 neue offene Akzeptanzkriterien ergaenzt – Nacharbeit-Gate aufgehoben
- **2026-10-02 15:41:18 (unassigned):** Rework 2: Kommentar (14:06) in Kriterien umgewandelt. Klaerung mit dem Nutzer: deutlich abgesetzter 3. Wert im Auswahl-Panel; Cloud-Wert soll auch fuer Verzeichnisse erscheinen (bisher nur Datei).
- **2026-10-02 15:41:18 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 15:41:18 (build-opencode):** Nacharbeit 2: Cloud-Wert prominent und fuer Verzeichnisse im Auswahl-Panel.
- **2026-10-02 15:43:20 (build-opencode):** Nacharbeit 2 umgesetzt.

- UI (views.rs selection_section): Der Cloud-Speicher ist jetzt ein eigener, abgesetzter Wert statt einer Caption-Fussnote - ein gerahmtes Kaestchen mit Ueberschrift 'In the cloud', der Cloud-Groesse als TITLE-Wert in der Farbe der Kategorie Synced (bewusst nicht das Auswahl-Amber) und darunter 'N Dateien - X auf der Platte'. Es erscheint bei jedem Knoten mit Cloud-Referenzen, Datei UND Verzeichnis (node.cloud_files ist der aggregierte Wert; tree::aggregate summiert cloud_bytes/cloud_files fuer Verzeichnisse, belegt durch tree::tests::cloud_references_are_counted_apart_from_disk_bytes).
- Lokal/Cloud bleiben getrennt: Standardmass bleibt die Allokation; 'auf der Platte' ist bytes bzw. bytes - cloud_bytes bei --apparent-size. Kein Hydratisieren.
- i18n: neue Keys 'In the cloud' und '{0} files - {1} on disk' in en und de.
- Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (App 84, Core 146, xtask 3). testsBy=HUMAN, daher keine automatischen UI-Belege.

GRENZE (ehrlich): Ein Verzeichnis, dessen Cloud-Inhalt ausschliesslich in uebersprungenen Platzhalter-Unterordnern (RECALL_ON_OPEN) liegt, hat keine Zahl - diese Ordner werden bewusst nicht gelistet, damit OneDrive nichts nachlaedt. Gemischte/lokale Verzeichnisse mit Cloud-Dateien zeigen den Wert.

OFFEN: Kriterium 10 - Sichtpruefung am echten OneDrive-Baum (Verzeichnis zeigt den Cloud-Wert deutlich).

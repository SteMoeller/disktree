---
id: DISK-10
projectId: PRJ-DISKTREE
title: 'Treemap: Rechtsklick kopiert den Pfad von Ordner/Datei'
summary: Ein Rechtsklick auf eine Kachel (Ordner oder Datei) legt deren absoluten
  Pfad in die Zwischenablage und bestaetigt das per Notice; die Kachel wird dabei
  ausgewaehlt. Rechtsklick ohne Kachel bleibt wirkungslos.
hints: |-
  state.rs::on_mouse_down behandelt Left/Middle/Navigate; MouseButton::Right ist heute der `_`-Zweig.
  tile_at(x, y) liefert die Crumbs unter dem Zeiger, path_at(&crumbs) den absoluten Pfad, marks::display_path() die gekuerzte Anzeige.
  Clipboard wie in copy_agent_prompt: cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(path)).
  Notice-Muster: self.notice = Some((text, Status::Success)) + cx.notify(); Text ueber crate::i18n::t()/tf().
  Test im Window-Harness: cx.simulate_mouse_down(point, MouseButton::Right, Modifiers::none()) und cx.read_from_clipboard() pruefen; Punkt aus einem Tile-Rechteck (treemap_origin + view.project).
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
allowedPaths: [
  ]
tags:
- ui
- mouse
- clipboard
filesChanged:
- crates/disktree-app/src/state.rs
- crates/disktree-app/src/tests.rs
- README.md
- disktree.en.i18n.txt
- disktree.de.i18n.txt
budget:
  updatesUsed: 2
createdAt: '2026-10-01T18:33:49.851672Z'
updatedAt: '2026-10-02T12:46:26.923518100Z'
closedAt: '2026-10-02T12:46:26.923518100Z'
---
### Problembeschreibung
Treemap: Rechtsklick kopiert den Pfad von Ordner/Datei

### Akzeptanzkriterien
- [x] Ein Rechtsklick auf eine Kachel (Ordner oder Datei) legt den absoluten Pfad dieser Kachel in die Zwischenablage.
- [x] Der Rechtsklick waehlt die Kachel zugleich aus und bestaetigt das Kopieren mit einer Notice, die den Pfad gekuerzt anzeigt.
- [x] Der kopierte Pfad ist der echte absolute Pfad (nicht die ~-Kurzform) und stimmt auch bei tief verschachtelten Kacheln.
- [x] Ein Rechtsklick ohne Kachel (leere Flaeche, Others) veraendert weder Zwischenablage noch Auswahl.
- [x] Harness-Test: Rechtsklick auf eine Kachel prueft den Inhalt der Zwischenablage; cargo xtask lint und cargo xtask test bleiben gruen.
### Audit-Log & Agenten-Notizen
- **2026-10-01 18:34:00 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-01 18:34:00 (build-opencode):** Umsetzung gestartet: MouseButton::Right in on_mouse_down -> Kachel auswaehlen, absoluten Pfad kopieren, Notice.
- **2026-10-01 18:40:20 (build-opencode):** Umgesetzt (Commit 041d6e1):
• state.rs::on_mouse_down behandelt jetzt MouseButton::Right: Kachel unter dem Zeiger wird ausgewaehlt und ihr absoluter Pfad ueber cx.write_to_clipboard in die Zwischenablage gelegt; neue Methode copy_path().
• copy_path() bricht ab, wenn die Crumbs nicht im Baum aufloesbar sind (node_at is_none) - ein unbekannter Pfad kopiert nichts.
• Notice bestaetigt: i18n-Key 'copied the path: {0}' (en identisch, de 'Pfad kopiert: {0}'); angezeigt wird die ~-gekuerzte Form, kopiert wird der echte Pfad.
• README: Hinweis am Mosaik-Bullet. Der `_`-Zweig im Match wurde zu MouseButton::Navigate(_) (clippy).
Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (74 App-Tests, 141 Core-Tests). Neuer Harness-Test a_right_click_copies_the_tiles_absolute_path: Rechtsklick auf die tiefste Kachel legt genau deren absoluten Pfad in die Zwischenablage; copy_path mit unbekanntem Pfad kopiert nichts; Auswahl wandert auf die Kachel und die Notice ist gesetzt.
Kein UI-/Sicht-Test (Nutzer testet am Ende).

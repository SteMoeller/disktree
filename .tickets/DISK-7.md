---
id: DISK-7
projectId: PRJ-DISKTREE
title: Default-Sprache folgt der Systemsprache (deutsches Windows -> Deutsch)
summary: 'Beim Start wird immer Englisch aktiv. Ziel: die Systemsprache ermitteln;
  existiert dazu eine gefundene disktree.<lang>.i18n.txt (z. B. de auf deutschem Windows),
  startet die App in dieser Sprache, sonst Englisch.'
status: DONE
priority: MEDIUM
type: FEATURE
assignee: build-opencode
testsBy: AGENT
effort: SMALL
blockedBy: [
  ]
referencesTicket:
- DISK-3
allowedPaths:
- crates/disktree-app/src
tags:
- i18n
- ui
budget:
  updatesUsed: 2
createdAt: '2026-10-01T15:54:39.459808Z'
updatedAt: '2026-10-01T18:29:14.561095200Z'
closedAt: '2026-10-01T18:29:14.561095200Z'
---
## Beobachtung / Ziel

Auf einem deutschen Windows soll disktree beim Start **Deutsch** anzeigen, wenn eine `disktree.de.i18n.txt` gefunden wurde. Aktuell startet die App immer auf Englisch, und der Nutzer muss selbst umschalten (Alt+L bzw. Klick auf EN/DE).

## Ursache (belegt)

`crates/disktree-app/src/i18n.rs::load()` legt Englisch als erstes Fach an und setzt den Zeiger fest darauf:

```rust
let mut languages = vec![Language { code: "en".to_string(), strings: leak(parse(EMBEDDED_EN)) }];
...
let _ = CATALOG.set(Catalog { languages, current: AtomicUsize::new(0) });
```

Es wird nie eine System-/Nutzer-Sprache ausgewertet. GPUI bringt dafuer nichts mit: eine Suche in `gpui-pre-0.3.6` findet keine Locale-/Sprach-API.

## Umsetzungsskizze

- Nach dem Einlesen der Dateien die Systemsprache bestimmen und daraus einen Sprachcode ableiten.
- Diesen Code auf die **gefundenen** Sprachen abbilden: gibt es ihn, `current` auf diesen Index setzen; sonst bei Englisch (Index 0) bleiben.
- Startsprache ist nur eine Vorbelegung; die manuelle Auswahl (Alt+L / Klick) ueberschreibt sie weiterhin.

Quelle der Systemsprache (plattformabhaengig, ohne neue Abhaengigkeit wenn moeglich):
- Unix/macOS: `LC_ALL`, `LC_MESSAGES`, `LANG` (erster gesetzter Wert).
- Windows: Benutzer-Locale ueber Win32 (`GetUserDefaultLocaleName` / `GetUserDefaultUILanguage`, Feature `Win32_Globalization` in `windows-sys` im App-Crate aktivieren - bisher nur `Win32_System_Console`) oder, falls einfacher, `sys-locale`.

Hinweis zur Projektregel: keine neue Abhaengigkeit ohne Rueckfrage. Wenn `sys-locale` bevorzugt wird, bitte kurz bestaetigen lassen; sonst `windows-sys`-Feature nutzen (dafuer ist `windows-sys` bereits im App-Crate).

## Abbildung auf den Code

`de-DE`, `de_AT`, `de-CH.UTF-8`, `DE` -> `de`. Also am ersten Trennzeichen (`-`/`_`/`.`) abschneiden, Kleinschreibung; Sonderfaelle wie `zh-Hans` koennen spaeter ergaenzt werden (nicht Teil dieses Tickets).

## Test-Tier

Tier 1: die Zuordnung Systemsprache -> Sprachcode als reine Funktion (`fn pick_language(system: &str, found: &[String]) -> usize` o. ae.) mit Unit-Tests fuer `de-DE`->de, `en-US`->en, unbekannt->en, ohne passende Datei->en.

### Akzeptanzkriterien
- [x] Beim Start wird die Systemsprache ermittelt (Unix: LC_ALL/LC_MESSAGES/LANG; Windows: Benutzer-/System-Locale).
- [x] Passt die Systemsprache zu einer gefundenen disktree.<lang>.i18n.txt, startet die App in dieser Sprache; sonst bleibt es Englisch.
- [x] Regionsangaben werden auf die Sprache abgebildet (de-DE, de_AT, de-CH.UTF-8, DE -> de); Kleinschreibung und Trennzeichen sind egal.
- [x] Es wird nur eine Sprache gewaehlt, fuer die tatsaechlich eine Datei gefunden wurde - nie eine leere Oberflaeche.
- [x] Die Zuordnung Systemsprache -> Sprachcode ist eine reine Funktion und durch Unit-Tests abgedeckt (Tier 1).
- [x] Die manuelle Sprachwahl (Alt+L / Klick im unteren Menue) ueberschreibt die Startsprache und bleibt wirksam.
### Audit-Log & Agenten-Notizen
- **2026-10-01 17:08:55 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-01 17:08:55 (build-opencode):** Startsprache folgt der Systemsprache: reine pick_language/language_code in i18n.rs, system_language() aus LC_ALL/LC_MESSAGES/LANG (Unix) bzw. GetUserDefaultLocaleName (Windows, windows-sys Feature Win32_Globalization). load() setzt den Startindex.
- **2026-10-01 17:11:49 (build-opencode):** Umgesetzt: load() ermittelt die Systemsprache und setzt den Startindex. Neue reine Funktionen pick_language(system, found) und language_code(system) mit Unit-Tests (de-DE/de_AT/de-CH.UTF-8/DE -> de; en-US -> en; unbekannt/C/leer -> en; nur gefundene Dateien waehlbar). system_language(): Unix LC_ALL/LC_MESSAGES/LANG, Windows GetUserDefaultLocaleName (windows-sys Feature Win32_Globalization). Sprachschalter (Alt+L/Klick) ueberschreibt weiterhin. Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (71 App-Tests, 141 Core-Tests).

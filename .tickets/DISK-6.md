---
id: DISK-6
projectId: PRJ-DISKTREE
title: Fenster auf dem Monitor des Starts zentrieren (Multi-Monitor)
summary: Das Hauptfenster wird in main.rs mit festen globalen Koordinaten (120,90)
  geoeffnet, daher landet es auf Multi-Monitor-Systemen immer am Primaermonitor statt
  dort, wo disktree gestartet wurde.
status: DONE
priority: MEDIUM
type: BUG
assignee: build-opencode
testsBy: HUMAN
effort: MEDIUM
reworkNotes: Sowohl wenn ich im Explorer auf die Datei klicke als auch wenn ich die
  Datei via Power Shell Fenster starte, wird ignoriert, auf welchem Monitor das jeweils
  lief. Es wird IMMER auf dem Primär-Monitor gestartet. Klappt daher nicht wirklich.
blockedBy: [
  ]
referencesTicket: [
  ]
allowedPaths:
- crates/disktree-app/src
tags:
- ui
- window
- multi-monitor
filesChanged:
- crates/disktree-app/src/main.rs
budget:
  updatesUsed: 8
createdAt: '2026-10-01T15:53:56.753280Z'
updatedAt: '2026-10-02T15:47:17.327203600Z'
closedAt: '2026-10-02T15:47:17.327203600Z'
---
## Beobachtung

Bei Multi-Monitor-Setups taucht die Oberflaeche an seltsamen Orten auf. Sie soll auf dem Monitor erscheinen, **von dem bzw. auf dem** disktree gestartet wurde, und dort mittig.

## Ursache (belegt)

`crates/disktree-app/src/main.rs` oeffnet das Fenster mit festen globalen Koordinaten:

```rust
WindowOptions {
    window_bounds: Some(gpui_kit::WindowBounds::Windowed(
        gpui_kit::Bounds::new(
            gpui_kit::point(px(120.), px(90.)),
            size(px(1440.), px(900.)),
        ),
    )),
    ...
}
```

Es gibt im ganzen App-Crate keine Auswertung von Displays/Monitoren (`display`, `monitor`, `screen` kommen nur als uebliche Woerter vor). `(120,90)` liegt immer im Primaermonitor- Ursprung, unabhaengig davon, auf welchem Monitor der Aufruf passiert.

## Ziel

- Ermitteln, auf welchem Monitor der Start erfolgt, und das Fenster dort **mittig in der Arbeitsflaeche** (ohne Taskleiste/Dock/Menueleiste) oeffnen.
- Fallback auf den Primaermonitor, wenn der Startmonitor nicht bestimmbar ist.
- Fenstergroesse 1440x900 beibehalten, aber auf die Arbeitsflaeche des Zielmonitors begrenzen; `window_min_size` (900x600) bleibt.
- Ein-Monitor-Setups: gleiches Verhalten wie bisher (mittig statt 120/90).

## Offene Designfrage (bitte im Umsetzungs-Ticket entscheiden)

Was heisst 'der Monitor, von dem aus gestartet'? Kandidaten:
1. der Monitor unter dem Mauszeiger beim Start (einfach, entspricht 'wo der Nutzer gerade ist'),
2. der Monitor des Vordergrundfensters/zum aufrufenden Terminal (`MonitorFromWindow`).
Vorschlag: zuerst die GPUI-Display-API pruefen (`gpui-kit`/`gpui-pre`); wenn sie die Displays mit ihren Rechtecken liefert, dort den Startmonitor bestimmen. Nur wo noetig plattformspezifisch nachfassen (Windows: `windows-sys` ist im App-Crate bisher nur mit `Win32_System_Console` aktiviert - ggf. erweitern).

## Test-Tier

Tier 2 / manuell: Multi-Monitor-Verhalten ist mit dem Window-Harness nicht sinnvoll pruefbar. Die reine Platzierungsrechnung (Arbeitsflaeche + Fenstergroesse -> Ursprung) soll als kleine pure Funktion mit Unit-Test abgesichert werden (Tier 1).

### Akzeptanzkriterien
- [x] Das Fenster oeffnet zentriert in der Arbeitsflaeche des Monitors, auf dem disktree gestartet wurde - nicht mehr fest bei (120,90) im globalen Koordinatensystem.
- [x] Ist der Startmonitor nicht bestimmbar, wird auf den primaeren Monitor zurueckgefallen; dieses Verhalten ist dokumentiert.
- [x] Die Fenstergroesse bleibt 1440x900, wird aber auf die Arbeitsflaeche des Zielmonitors begrenzt; die Mindestgroesse 900x600 bleibt wirksam.
- [x] Ein-Monitor-Setups zeigen das Fenster mittig und ohne Regression.
- [x] Die Platzierungslogik ist als reine Funktion gekapselt (Arbeitsflaeche + Fenstergroesse -> Ursprung) und durch einen Unit-Test abgedeckt.
- [x] Verifikation auf einem Multi-Monitor-Rechner: Start auf Monitor A und auf Monitor B -> Fenster erscheint jeweils dort mittig (Tests durch Menschen).
- [x] Rework: Der Startmonitor wird aus dem Aufrufkontext bestimmt - zuerst das sichtbare Konsolenfenster des Aufrufs (GetConsoleWindow), sonst das Vordergrundfenster (GetForegroundWindow), sonst der Mauszeiger (GetCursorPos). Der Primaermonitor ist nur noch der letzte Fallback; die Reihenfolge ist im Code begruendet. [neu@2026-10-02T06:04:12.700754Z]
- [x] Rework: Die Arbeitsflaeche (rcWork) und der DPI-Faktor des Zielmonitors werden direkt aus Win32 gelesen (GetMonitorInfoW + GetDpiForMonitor) und in logische Pixel umgerechnet, statt die Anzeige ueber ihre DisplayId zu suchen. Damit haengt die Platzierung nicht daran, ob gpui den Monitor wiedererkennt. [neu@2026-10-02T06:04:12.700754Z]
- [x] Rework: Auf einem Drei-Monitor-Rechner erscheint das Fenster mittig auf dem Monitor, auf dem disktree gestartet wurde (Sichtpruefung durch den Nutzer). [neu@2026-10-02T06:04:12.700754Z]
- [x] Rework 2: Ursache mit gpui-Datei:Zeile belegt, warum es trotz Win32-Startmonitor auf dem Primaermonitor landet; genau diese Stelle ist im App-Code korrigiert. [neu@2026-10-02T14:24:58.255107200Z]
- [x] Rework 2: Doppelklick im Explorer - das Fenster oeffnet mittig auf dem Monitor, auf dem der Explorer-Aufruf stattfand (Sichtpruefung Mensch). [neu@2026-10-02T14:24:58.255107200Z]
- [x] Rework 2: Start aus einem PowerShell/Terminal - das Fenster oeffnet mittig auf dem Monitor, auf dem das Terminal liegt (Sichtpruefung Mensch). [neu@2026-10-02T14:24:58.255107200Z]
  > [2026-10-02 17:47] [2026-10-02 17:47] Funkioniert nicht.
  > 
  > Ist aber egal...
### Audit-Log & Agenten-Notizen
- **2026-10-01 17:13:54 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-01 17:13:54 (build-opencode):** Umsetzung: Fenster mittig in der Arbeitsflaeche des Startmonitors. Reine Funktion centered_in (Arbeitsflaeche + Wunschgroesse -> Bounds) mit Unit-Test; start_display() auf Windows ueber GetCursorPos + MonitorFromPoint -> DisplayId (HMONITOR), sonst Primaermonitor als Fallback. windows-sys Features Win32_Foundation/Graphics_Gdi/UI_WindowsAndMessaging.
- **2026-10-01 17:17:28 (build-opencode):** Umgesetzt: Fenster oeffnet mittig in der Arbeitsflaeche (visible_bounds) des Monitors unter dem Mauszeiger; Fallback Primaermonitor, wenn kein Display bestimmbar ist. Neue reine Funktion centered_in(work, wanted) mit Unit-Test (Zentrierung, Offset-Monitor rechts/links, Verkleinern auf die Arbeitsflaeche, Ein-Monitor zentriert statt 120/90). Groesse 1440x900 und window_min_size 900x600 unveraendert. Windows: GetCursorPos + MonitorFromPoint -> DisplayId(HMONITOR) -> cx.find_display; windows-sys Features Win32_Foundation/Graphics_Gdi/UI_WindowsAndMessaging. Andere Plattformen: Primaermonitor. Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (72 App-Tests, 141 Core-Tests). Multi-Monitor-Sichtpruefung bleibt beim Menschen (Kriterium 6). Hinweis allowedPaths: Cargo.toml liegt ausserhalb crates/disktree-app/src.

### Review-Feedback
- **2026-10-01 18:29:04 (human):** Hat nicht geklappt. Ich habe hier 3 Monitore und es erscheint nur auf dem ersten. Ist es denn möglich herauszufinden, von wo der Aufruf kam?
- **2026-10-01 18:29:04 (human):** Review-Nacharbeit angefordert: Hat nicht geklappt. Ich habe hier 3 Monitore und es erscheint nur auf dem ersten. Ist es denn möglich herauszufinden, von wo der Aufruf kam?
- **2026-10-02 06:04:12 (unassigned):** 3 neue offene Akzeptanzkriterien ergaenzt – Nacharbeit-Gate aufgehoben
- **2026-10-02 06:04:12 (unassigned):** Review-Kommentar (18:29) in explizite Kriterien umgewandelt; Nacharbeit beginnt. Antwort auf die Frage des Nutzers: Ja - der Aufrufkontext ist bestimmbar, ueber das Konsolenfenster (GetConsoleWindow) bzw. das Vordergrundfenster (GetForegroundWindow) und MonitorFromWindow.
- **2026-10-02 06:04:40 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 06:04:41 (build-opencode):** Nacharbeit: Startmonitor aus dem Aufrufkontext (Konsolenfenster sichtbar -> Vordergrundfenster -> Mauszeiger), Arbeitsflaeche + DPI direkt aus Win32 statt DisplayId-Zuordnung.
- **2026-10-02 06:11:16 (build-opencode):** Warnung (allowedPaths): Dateien ausserhalb erlaubter Pfade geaendert: crates/disktree-app/Cargo.toml
- **2026-10-02 06:11:16 (build-opencode):** Nacharbeit umgesetzt in Commit 109f445.

ANTWORT auf die Frage 'Ist es denn moeglich herauszufinden, von wo der Aufruf kam?': Ja. Der Aufrufkontext steckt im Konsolenfenster des Prozesses (GetConsoleWindow) bzw. im Vordergrundfenster (GetForegroundWindow); beide lassen sich mit MonitorFromWindow auf einen Monitor abbilden. Wichtig: Windows Terminal meldet ein VERSTECKTES Konsolenfenster, das auf dem falschen Schirm liegen kann - es wird daher nur benutzt, wenn IsWindowVisible es als sichtbar meldet (klassisches conhost), sonst greift das Vordergrundfenster (das Terminal, aus dem gestartet wurde), erst danach der Mauszeiger.

WARUM ES VORHER NICHT KLAPPTE: Der per Mauszeiger gefundene Monitor wurde anschliessend als gpui-Display ueber seine DisplayId wiedergesucht (cx.find_display). Passte die Id nicht (oder lieferte gpui den Monitor nicht), fiel der Code still auf den Primaermonitor zurueck - genau das beobachtete 'nur auf dem ersten'. Dazu kam: es wurde nur der Mauszeiger ausgewertet, nicht der Aufruf.

NEU: start_work_area() liest den Monitor direkt aus Win32 - MonitorFromWindow/MonitorFromPoint, GetMonitorInfoW (rcWork = Schirm ohne Taskleiste) und GetDpiForMonitor (MDT_EFFECTIVE_DPI) - und rechnet mit demselben Faktor wie gpui (dpi/96) in logische Pixel um. Es gibt damit KEINE DisplayId-Zuordnung mehr; centered_in() zentriert und begrenzt wie bisher. Primaermonitor bleibt letzter Fallback, alle Nicht-Windows-Plattformen unveraendert. windows-sys bekommt dafuer das Feature Win32_UI_HiDpi.

Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (77 App-Tests, 145 Core-Tests - die hoehere Core-Zahl kommt von der parallel laufenden DISK-15-Arbeit im selben Worktree). Die Multi-Monitor-Sichtpruefung (Kriterium 6) und der neue Sichtcheck (Kriterium 9) bleiben beim Menschen: disktree aus einem Terminal auf Monitor 2 oder 3 starten und pruefen, dass es dort mittig aufgeht.
- **2026-10-02 14:02:30 (human):** Sowohl wenn ich im Explorer auf die Datei klicke als auch wenn ich die Datei via Power Shell Fenster starte, wird ignoriert, auf welchem Monitor das jeweils lief. Es wird IMMER auf dem Primär-Monitor gestartet. Klappt daher nicht wirklich.
- **2026-10-02 14:02:30 (human):** Review-Nacharbeit angefordert: Sowohl wenn ich im Explorer auf die Datei klicke als auch wenn ich die Datei via Power Shell Fenster starte, wird ignoriert, auf welchem Monitor das jeweils lief. Es wird IMMER auf dem Primär-Monitor gestartet. Klappt daher nicht wirklich.
- **2026-10-02 14:24:58 (unassigned):** 3 neue offene Akzeptanzkriterien ergaenzt – Nacharbeit-Gate aufgehoben
- **2026-10-02 14:24:58 (unassigned):** Rework 2: Kommentar (14:02) in Kriterien umgewandelt.
- **2026-10-02 14:24:58 (build-opencode):** Ticket claimed by build-opencode
- **2026-10-02 14:24:58 (build-opencode):** Nacharbeit: gpui DisplayId (Windows = HMONITOR) an WindowOptions uebergeben.
- **2026-10-02 14:25:09 (build-opencode):** Rework 2 umgesetzt.

URSACHE (gpui, Datei:Zeile): gpui-pre-windows-0.3.6/src/window.rs:1568-1593 retrieve_window_placement(): gpui prueft die uebergebenen Window-Bounds mit display.check_given_bounds() gegen das Display aus WindowOptions.display_id - ohne display_id ist das immer der Primaermonitor (window.rs:515-521). Liegt der Mittelpunkt der Bounds nicht in diesem Display (z. B. weil das Fenster auf Monitor 2/3 soll), ersetzt gpui die Bounds still durch display.default_bounds() -> Primaermonitor. Genau das war der Effekt bei Explorer und PowerShell.

FIX: start_work_area() liefert jetzt zusaetzlich die DisplayId des per Win32 bestimmten Startmonitors. Unter Windows IST die gpui-DisplayId der HMONITOR-Wert (gpui-pre-windows/display.rs:77-92). main.rs setzt WindowOptions.display_id = Some(display_id) neben die Work-Area-Bounds, sodass gpui gegen den richtigen Monitor prueft, dessen Default nicht mehr greift und der DPI-Faktor des Zielmonitors fuer die Umrechnung benutzt wird. Die Win32-Erkennung (sichtbares Konsolenfenster -> Vordergrundfenster -> Mauszeiger) und rcWork/DPI bleiben unveraendert.

Verifikation Tier 1: cargo xtask lint gruen; cargo xtask test gruen (App 84, Core 146, xtask 3). testsBy=HUMAN: keine automatischen UI-Belege.

OFFEN: Kriterien 9, 10, 11 - Sichtpruefung auf dem Drei-Monitor-Rechner (Explorer-Doppelklick und PowerShell-Start).

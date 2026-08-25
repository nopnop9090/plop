# Plop! — Desktop Toy für Windows 11

Wenn ein Fenster geschlossen **oder minimiert** wird, "weggeploppt" es:
Konfetti an der alten Position + Pop/Confetti-Sound. Läuft unsichtbar im
Hintergrund (Tray-Icon).

**Versionierung:** `1.0.<DateCode>.b<BuildNr>` — Build-Nr zählt bei jedem
Rebuild automatisch hoch (`build.rs` + `build_info.txt`).

## Starten

```
target\release\plop.exe
```

Tray-Icon (Konfetti-Punkte):

- **Linksklick**: An/Aus
- **Rechtsklick** → Menü:
  - `Plop! vX.Y...` (Versionsanzeige, inaktiv)
  - `Aktiv` — Pops ein/aus
  - `Ton` — Sound an/aus (Konfetti bleibt)
  - `Minimier-Animation aus` — tötet die System-Animation beim Minimieren,
    damit der Konfetti-Effekt voll wirkt. Original-Einstellung wird beim
    Beenden von Plop! wiederhergestellt.
  - `Test-Pop` — Effekt am Cursor testen
  - `Beenden`

## Fernsteuerung (IPC)

Externe Programme können Status abfragen und alles umschalten — per
`WM_USER`-Nachrichten an das Fenster `PlopTrayHost`. Protokoll-Doku:
**[docs/IPC.md](docs/IPC.md)**. Fertiges CLI:

```
pwsh -File tools\plop_ctl.ps1 status
pwsh -File tools\plop_ctl.ps1 enable 0   # Pops aus
pwsh -File tools\plop_ctl.ps1 sound 0    # Ton aus, Konfetti bleibt
pwsh -File tools\plop_ctl.ps1 minanim 1  # Minimier-Animation aus
pwsh -File tools\plop_ctl.ps1 test       # Test-Pop am Cursor
pwsh -File tools\plop_ctl.ps1 version
```

## Ressourcen

- Eine einzelne `plop.exe` (~1.4 MB statisch gelinkt inkl. 4 WAV-Sounds,
  keine externen Abhängigkeiten)
- Idle: ~15–20 MB RAM, keine Disk-I/O

## Verhalten

- Erkannt werden **normale App-Fenster** (sichtbar, Caption/Taskleisten-
  Präsenz, kein Toolwindow, nicht DWM-cloaked). Explorer zählt dazu.
- Pop bei **Schließen und Minimieren**; Verstecken (Tray-Apps) poppt nicht.
- **Elevated Prozesse**: deren WinEvents sind per UIPI unsichtbar — ein
  Selbtheilungs-Sweep (alle 500 ms) erfasst sie trotzdem (bis zu ~0,5 s
  Verzögerung beim Pop).
- **Close-to-Tray-Apps**: konfigurierbar über `plop.ini` (neben der exe):
  ```ini
  [ignore]
  class=ThunderFrame
  exe=steam.exe

  [hide_as_close]
  exe=discord.exe
  ```
  `[ignore]` = poppt nie; `[hide_as_close]` = Verstecken zählt als Schließen.
- Diagnose: `PLOP_DEBUG=1` setzen → jede Pop-Attribution in
  `%TEMP%\plop_debug.log`.

## Autostart (optional)

```
reg add HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v PlopToy /t REG_SZ /d "F:\winfx\plop\target\release\plop.exe" /f
```

## Bauen

```
cargo build --release
```

## Sounds

Stammen aus den FMOD-Banks eines Unity-Spiels (via vgmstream extrahiert;
Quellen in `assets/src_*.wav`). `tools/make_sounds.py` mischt daraus die 4
Varianten (3× pop+confetti, 1× confetti ohne pop) → `assets/plop_sound_*.wav`,
die per `include_bytes!` eingebettet werden. Zufallsauswahl pro Pop.

```
python tools\make_sounds.py
cargo build --release
```

## Tests

- `tools/test_storm.ps1` — Minimiert-bleiben darf nicht endlos poppen
- `tools/test_restart_minimize.ps1` — Fenster beim Plop-Start minimiert
- `tools/test_minimize.ps1` / `test_tray.ps1` / `test_hide_as_close.ps1`
- `tools/test_overlay.ps1` / `test_pop.ps1` — Grundeffekt

## Technik

Rust + `windows`-crate. Drei schmale `SetWinEventHook`s (Minimize, Show/Hide/
Destroy, LocationChange) plus 500-ms-Sweep als Sicherheitsnetz. Rect-Cache
mit Hidden-Grace (Tray-Apps) und Minimized-Flag (kein Re-Pop). Konfetti:
Kurzzeit-Thread mit Topmost-Layered-Fenster (`WS_EX_TRANSPARENT |
WS_EX_NOACTIVATE`), Software-Rasterizer + `UpdateLayeredWindow` @ ~60 fps.
Sound: `PlaySound(SND_ASYNC)` mit eingebetteten WAVs, 60 ms Throttle.
Per-Monitor-V2 DPI-Awareness; Single-Instance via benanntem Mutex.

# Plop! IPC-Protokoll (WM_USER)

Plop! laesst sich von externen Programmen ueber Windows-Fensternachrichten
steuern und auslesen. Es ist kein zusaetzlicher Server noetig — die
Nachrichten gehen an das versteckte Tray-Host-Fenster.

## Ziel-Fenster finden

```c
HWND h = FindWindowW(L"PlopTrayHost", NULL);
```

Immer zuerst `PM_PING` schicken und die Magic `0x504C4F50` (`'PLOP'`)
pruefen — so stellen wir sicher, dass das Fenster wirklich Plop! ist.

## Nachrichten

Basis: `WM_USER` = `0x0400`. Alle Nachrichten sind `SendMessage`-synchron
und liefern ihren Ergebniswert direkt zurueck (`LRESULT`).

| Nachricht       | Wert       | wParam            | lParam | Rueckgabe |
|-----------------|------------|-------------------|--------|-----------|
| `PM_PING`       | 0x0400     | 0                 | 0      | `0x504C4F50` (Magic) |
| `PM_GET_STATE`  | 0x0401     | 0                 | 0      | Status-Bitmaske (siehe unten) |
| `PM_SET_ENABLED`| 0x0402     | 0 = aus, 1 = an   | 0      | vorheriger Zustand (0/1) |
| `PM_SET_SOUND`  | 0x0403     | 0 = aus, 1 = an   | 0      | vorheriger Zustand (0/1) |
| `PM_SET_MINANIM`| 0x0404     | 0 = System-Animation an, 1 = Animation aus | 0 | vorheriger Zustand (0/1) |
| `PM_TEST_POP`   | 0x0405     | 0                 | 0      | 1 (Pop am Cursor wird ausgeloest) |
| `PM_GET_VERSION`| 0x0406     | 0                 | 0      | gepackte Version (siehe unten) |

Unbekannte Nachrichten im Bereich `0x0400..=0x0410` antworten mit `0`.

## Status-Bitmaske (`PM_GET_STATE`)

| Bit | Maske | Bedeutung |
|-----|-------|-----------|
| 0   | `0x1` | Pops aktiv (`Aktiv` im Tray-Menue) |
| 1   | `0x2` | Ton an |
| 2   | `0x4` | Minimier-Animation vom System abgeschaltet (`Minimier-Animation aus`) |

Beispiele: `0x7` = alles an, `0x5` = Konfetti ohne Ton, `0x4` = komplett
passiv (aber Minimier-Animation weiterhin unterdrueckt).

## Version (`PM_GET_VERSION`)

Gepackt als 32-Bit-Wert:

```
31        24 23        16 15         0
[ major    ][ minor     ][ build_nr  ]
```

- `major`/`minor`: Plop!-Version (z. B. 1.1)
- `build_nr`: automatischer Build-Zaehler (16 Bit)

## Verhalten & Hinweise

- `PM_SET_ENABLED = 0` entspricht exakt dem Tray-Menuepunkt `Aktiv` abwählen:
  keine Konfetti-Overlays, kein Sound. Der Sweep/Hook laeuft weiter.
- `PM_SET_SOUND` entspricht dem Menuepunkt `Ton`.
- `PM_SET_MINANIM` entspricht `Minimier-Animation aus`. Beim Beenden von
  Plop! wird die System-Einstellung immer auf den Originalwert
  zurueckgesetzt — unabhaengig von IPC-Aenderungen.
- `PM_TEST_POP` loest denselben Effekt aus wie der Menuepunkt `Test-Pop`
  (Konfetti + Sound an der Cursorposition).
- Nachrichten werden mit `ChangeWindowMessageFilterEx(MSGFLT_ALLOW)`
  freigeschaltet — Steuerung funktioniert auch, falls Plop! elevated
  laeuft (der steuernde Prozess braucht dann aber trotzdem genug Rechte,
  um `FindWindow`/`SendMessage` auszufuehren; umgekehrt gilt UIPI wie
  ueblich: Low-Integrity-Prozesse koennen nicht senden).

## Beispiel: PowerShell

```powershell
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class PlopIpc {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")]
    public static extern IntPtr SendMessageW(IntPtr hWnd, uint msg, IntPtr wp, IntPtr lp);
}
"@
$hwnd = [PlopIpc]::FindWindowW("PlopTrayHost", $null)
$PM_GET_STATE = 0x0401
$st = [int][PlopIpc]::SendMessageW($hwnd, $PM_GET_STATE, [IntPtr]::Zero, [IntPtr]::Zero)
"Pops aktiv: $(($st -band 1) -ne 0), Ton: $(($st -band 2) -ne 0)"
```

## Beispiel: C

```c
#include <windows.h>

enum {
    PM_PING        = WM_USER + 0,
    PM_GET_STATE   = WM_USER + 1,
    PM_SET_ENABLED = WM_USER + 2,
    PM_SET_SOUND   = WM_USER + 3,
    PM_SET_MINANIM = WM_USER + 4,
    PM_TEST_POP    = WM_USER + 5,
    PM_GET_VERSION = WM_USER + 6,
};

HWND h = FindWindowW(L"PlopTrayHost", NULL);
if (h && SendMessageW(h, PM_PING, 0, 0) == 0x504C4F50) {
    DWORD st = (DWORD)SendMessageW(h, PM_GET_STATE, 0, 0);
    SendMessageW(h, PM_SET_SOUND, FALSE, 0);   // Ton aus
    SendMessageW(h, PM_TEST_POP, 0, 0);        // einen Pop ausloesen
}
```

## Fertiges Steuerungstool

`tools/plop_ctl.ps1` implementiert das komplette Protokoll:

```
pwsh -File tools\plop_ctl.ps1 status
pwsh -File tools\plop_ctl.ps1 enable 0
pwsh -File tools\plop_ctl.ps1 sound 0
pwsh -File tools\plop_ctl.ps1 minanim 1
pwsh -File tools\plop_ctl.ps1 test
pwsh -File tools\plop_ctl.ps1 version
```

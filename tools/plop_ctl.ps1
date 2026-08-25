# plop_ctl.ps1 — Steuerung/Diagnose fuer Plop! per IPC (docs/IPC.md)
# Beispiele:
#   pwsh -File plop_ctl.ps1 status
#   pwsh -File plop_ctl.ps1 enable 0
#   pwsh -File plop_ctl.ps1 sound 1
#   pwsh -File plop_ctl.ps1 minanim 0
#   pwsh -File plop_ctl.ps1 test
#   pwsh -File plop_ctl.ps1 version
param(
    [string]$cmd = "status",
    [int]$value = -1
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class PlopIpc {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr hWnd, uint msg, IntPtr wp, IntPtr lp);
}
"@

$PM_PING       = 0x0400
$PM_GET_STATE  = 0x0401
$PM_SET_ENABLED= 0x0402
$PM_SET_SOUND  = 0x0403
$PM_SET_MINANIM= 0x0404
$PM_TEST_POP   = 0x0405
$PM_GET_VERSION= 0x0406
$PM_MAGIC      = 0x504C4F50  # 'PLOP'

$hwnd = [PlopIpc]::FindWindowW("PlopTrayHost", $null)
if ($hwnd -eq [IntPtr]::Zero) { Write-Output "plop laeuft nicht (Fenster nicht gefunden)"; exit 1 }
if ([PlopIpc]::SendMessageW($hwnd, $PM_PING, [IntPtr]::Zero, [IntPtr]::Zero) -ne [IntPtr]$PM_MAGIC) {
    Write-Output "Fenster gefunden, aber kein Plop! (Ping-Magic falsch)"; exit 1
}

function Get-StateBits { [int32][PlopIpc]::SendMessageW($hwnd, $PM_GET_STATE, [IntPtr]::Zero, [IntPtr]::Zero) }
function Show-Status {
    $st = Get-StateBits
    Write-Output ("aktiv={0} ton={1} minanim-aus={2} (bits=0x{3:X})" -f `
        (($st -band 1) -ne 0), (($st -band 2) -ne 0), (($st -band 4) -ne 0), $st)
}

switch ($cmd.ToLower()) {
    "status" { Show-Status }
    "enable" {
        if ($value -lt 0) { Write-Output "usage: enable 0|1"; exit 1 }
        $prev = [PlopIpc]::SendMessageW($hwnd, $PM_SET_ENABLED, [IntPtr]$value, [IntPtr]::Zero)
        Write-Output "aktiv: $value (vorher $prev)"
        Show-Status
    }
    "sound" {
        if ($value -lt 0) { Write-Output "usage: sound 0|1"; exit 1 }
        $prev = [PlopIpc]::SendMessageW($hwnd, $PM_SET_SOUND, [IntPtr]$value, [IntPtr]::Zero)
        Write-Output "ton: $value (vorher $prev)"
        Show-Status
    }
    "minanim" {
        if ($value -lt 0) { Write-Output "usage: minanim 0|1  (1 = Animation aus)"; exit 1 }
        $prev = [PlopIpc]::SendMessageW($hwnd, $PM_SET_MINANIM, [IntPtr]$value, [IntPtr]::Zero)
        Write-Output "minanim-aus: $value (vorher $prev)"
        Show-Status
    }
    "test" {
        [void][PlopIpc]::SendMessageW($hwnd, $PM_TEST_POP, [IntPtr]::Zero, [IntPtr]::Zero)
        Write-Output "Test-Pop ausgeloesst"
    }
    "version" {
        $v = [int32][PlopIpc]::SendMessageW($hwnd, $PM_GET_VERSION, [IntPtr]::Zero, [IntPtr]::Zero)
        $major = ($v -shr 24) -band 0xFF
        $minor = ($v -shr 16) -band 0xFF
        $build = $v -band 0xFFFF
        Write-Output "plop $major.$minor build $build"
    }
    default { Write-Output "usage: plop_ctl.ps1 [status|enable 0/1|sound 0/1|minanim 0/1|test|version]"; exit 1 }
}

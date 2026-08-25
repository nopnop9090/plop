Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class W3 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-OverlayWindows {
    $found = New-Object System.Collections.ArrayList
    $cb = [W3+EnumProc]{
        param($h, $lp)
        if ([W3]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W3]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true
    }
    [W3]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $found
}

function Wait-Overlay($ms) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if ((Find-OverlayWindows).Count -gt 0) { return $sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 25
    }
    return -1
}

$f = New-Object System.Windows.Forms.Form
$f.Text = "PlopMinimizeTest"
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(600, 350, 420, 280)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 800

# 1) minimize -> should pop
$f.WindowState = 'Minimized'
$minMs = Wait-Overlay 1500
Start-Sleep -Milliseconds 1800   # let overlay finish

# 2) restore (no pop), then close -> should pop again
$f.WindowState = 'Normal'
Start-Sleep -Milliseconds 600
$f.Close()
$closeMs = Wait-Overlay 1500
Start-Sleep -Milliseconds 1500

$left = (Find-OverlayWindows).Count
"overlay-on-minimize-ms: $minMs"
"overlay-on-close-ms: $closeMs"
"leftover: $left"
if ($minMs -ge 0 -and $closeMs -ge 0 -and $left -eq 0) { "RESULT: MIN+CLOSE POP OK" } else { "RESULT: CHECK NEEDED" }

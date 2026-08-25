Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class W6 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-Overlays {
    $found = New-Object System.Collections.ArrayList
    $cb = [W6+EnumProc]{ param($h, $lp)
        if ([W6]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W6]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true }
    [W6]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $found
}
function Wait-Overlay($ms) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if ((Find-Overlays).Count -gt 0) { return $sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 25
    }
    return -1
}

$f = New-Object System.Windows.Forms.Form
$f.Text = 'PlopRestartScenario'
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(520, 320, 420, 280)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 600

# simulate the broken case: window is MINIMIZED while plop restarts
$f.WindowState = 'Minimized'
Start-Sleep -Milliseconds 300
& taskkill /IM plop.exe /F 2>&1 | Out-Null
Start-Sleep -Milliseconds 500
Start-Process "F:\winfx\plop\target\release\plop.exe"
Start-Sleep -Milliseconds 1500   # sweep should have inserted it (minimized, no pop)

# restore (no pop), then minimize -> POP expected now
$f.WindowState = 'Normal'
Start-Sleep -Milliseconds 1200
$f.WindowState = 'Minimized'
$ms = Wait-Overlay 2500
Start-Sleep -Milliseconds 1600
$left = (Find-Overlays).Count

"overlay-on-minimize-after-restart: $ms (erwarte >= 0, <= ~1200)"
"leftover: $left"
if ($ms -ge 0 -and $left -eq 0) { 'RESULT: RESTART-MINIMIZE OK' } else { 'RESULT: CHECK NEEDED' }

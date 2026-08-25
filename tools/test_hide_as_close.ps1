Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class W5 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-Overlays {
    $found = New-Object System.Collections.ArrayList
    $cb = [W5+EnumProc]{ param($h, $lp)
        if ([W5]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W5]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true }
    [W5]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $found
}

$f = New-Object System.Windows.Forms.Form
$f.Text = 'PlopHideAsClose'
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(500, 300, 400, 260)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 800

$f.Hide()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$ms = -1
while ($sw.ElapsedMilliseconds -lt 1500) {
    if ((Find-Overlays).Count -gt 0) { $ms = $sw.ElapsedMilliseconds; break }
    Start-Sleep -Milliseconds 25
}
Start-Sleep -Milliseconds 1600
"overlay-after-hide-as-close: $ms (erwarte >= 0)"
if ($ms -ge 0) { 'RESULT: HIDE_AS_CLOSE OK' } else { 'RESULT: CHECK NEEDED' }

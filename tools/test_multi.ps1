Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class W2 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-OverlayWindows {
    $found = New-Object System.Collections.ArrayList
    $cb = [W2+EnumProc]{
        param($h, $lp)
        if ([W2]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W2]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true
    }
    [W2]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $found
}

$forms = @()
for ($i = 0; $i -lt 3; $i++) {
    $f = New-Object System.Windows.Forms.Form
    $f.Text = "PlopMulti$i"
    $f.StartPosition = 'Manual'
    $f.Bounds = New-Object System.Drawing.Rectangle((100 + 200 * $i), (150 + 60 * $i), 300, 220)
    $f.TopMost = $true
    $null = $f.Show()
    $forms += $f
}
Start-Sleep -Milliseconds 700
foreach ($f in $forms) { $f.Close() }

$peak = 0
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while ($sw.ElapsedMilliseconds -lt 800) {
    $peak = [Math]::Max($peak, (Find-OverlayWindows).Count)
    Start-Sleep -Milliseconds 25
}
Start-Sleep -Milliseconds 2200
$left = (Find-OverlayWindows).Count
"overlays-peak=$peak leftover-after-3s=$left"
if ($peak -ge 2 -and $left -eq 0) { "RESULT: MULTI-POP OK, CLEAN TEARDOWN" } else { "RESULT: CHECK NEEDED" }

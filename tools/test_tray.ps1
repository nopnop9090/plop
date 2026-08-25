Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class W4 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-OverlayWindows {
    $found = New-Object System.Collections.ArrayList
    $cb = [W4+EnumProc]{
        param($h, $lp)
        if ([W4]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W4]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true
    }
    [W4]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
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
$f.Text = "PlopTrayTest"
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(500, 300, 400, 260)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 800

# 1) hide-to-tray simulation: no pop expected
$f.Hide()
$hideMs = Wait-Overlay 900
Start-Sleep -Milliseconds 2300   # > HIDE_GRACE

# 2) destroy while hidden long ago: no pop expected
$f.Dispose()
$destroyMs = Wait-Overlay 900

# 3) normal close: pop expected
$g = New-Object System.Windows.Forms.Form
$g.Text = "PlopNormalClose"
$g.StartPosition = 'Manual'
$g.Bounds = New-Object System.Drawing.Rectangle(500, 300, 400, 260)
$g.TopMost = $true
$null = $g.Show()
Start-Sleep -Milliseconds 800
$g.Close()
$closeMs = Wait-Overlay 1200
Start-Sleep -Milliseconds 1600

"overlay-after-hide: $hideMs (erwarte -1)"
"overlay-after-late-destroy: $destroyMs (erwarte -1)"
"overlay-after-normal-close: $closeMs (erwarte >= 0)"
if ($hideMs -eq -1 -and $destroyMs -eq -1 -and $closeMs -ge 0) { "RESULT: TRAY HANDLING OK" } else { "RESULT: CHECK NEEDED" }

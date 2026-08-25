Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class W7 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-Overlays {
    $found = New-Object System.Collections.ArrayList
    $cb = [W7+EnumProc]{ param($h, $lp)
        if ([W7]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W7]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true }
    [W7]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
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
function Wait-Overlays-Gone($ms) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if ((Find-Overlays).Count -eq 0) { return $sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 100
    }
    return -1
}

$f = New-Object System.Windows.Forms.Form
$f.Text = 'PlopStormTest'
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(520, 320, 420, 280)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 800

# 1) minimize -> exactly ONE pop
$f.WindowState = 'Minimized'
$null = Wait-Overlay 1500
Start-Sleep -Milliseconds 400
$null = Wait-Overlays-Gone 4000   # let the first overlay fully finish

# 2) stay minimized for 4s -> NO further pops allowed
Start-Sleep -Milliseconds 4000
$rePops = (Find-Overlays).Count

# 3) restore, minimize again -> pops again
$f.WindowState = 'Normal'
Start-Sleep -Milliseconds 1000
$f.WindowState = 'Minimized'
$again = Wait-Overlay 2000
Start-Sleep -Milliseconds 500
$null = Wait-Overlays-Gone 4000
$f.Close()
Start-Sleep -Milliseconds 300

"repop-while-minimized: $rePops (erwarte 0)"
"pop-on-2nd-minimize: $again (erwarte >= 0)"
if ($rePops -eq 0 -and $again -ge 0) { 'RESULT: NO STORM, RE-MINIMIZE OK' } else { 'RESULT: CHECK NEEDED' }

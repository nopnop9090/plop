Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class W8 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
function Find-Overlays {
    $found = New-Object System.Collections.ArrayList
    $cb = [W8+EnumProc]{ param($h, $lp)
        if ([W8]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [W8]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') { $null = $found.Add($h) }
        }
        return $true }
    [W8]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
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
$f.Text = 'IpcGateTest'
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(500, 300, 380, 240)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 700
$f.Close()
$ms = Wait-Overlay 900
Start-Sleep -Milliseconds 1500
"overlay-while-disabled: $ms (erwarte -1)"
if ($ms -eq -1) { 'RESULT: IPC GATING OK' } else { 'RESULT: CHECK NEEDED' }

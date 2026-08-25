Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Win32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
[DpiFix]::SetProcessDPIAware() | Out-Null

function Find-OverlayWindows {
    $found = New-Object System.Collections.ArrayList
    $cb = [Win32+EnumProc]{
        param($h, $lp)
        if ([Win32]::IsWindowVisible($h)) {
            $sb = New-Object System.Text.StringBuilder 256
            [Win32]::GetClassName($h, $sb, 256) | Out-Null
            if ($sb.ToString() -eq 'PlopConfettiOverlay') {
                $null = $found.Add($h)
            }
        }
        return $true
    }
    [Win32]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $found
}

# baseline: should be none
"overlay-before-close: $((Find-OverlayWindows).Count)"

$f = New-Object System.Windows.Forms.Form
$f.Text = "PlopTestWindow2"
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(150, 420, 460, 320)
$f.TopMost = $true
$null = $f.Show()
Start-Sleep -Milliseconds 700

$f.Close()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$detected = -1
while ($sw.ElapsedMilliseconds -lt 1500) {
    $n = (Find-OverlayWindows).Count
    if ($n -gt 0) {
        $detected = $sw.ElapsedMilliseconds
        # crop screenshot of the form region mid-animation
        Start-Sleep -Milliseconds 350
        $bmp = New-Object System.Drawing.Bitmap(760, 560)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen(50, 370, 0, 0, $bmp.Size)
        $g.Dispose()
        $bmp.Save("$env:TEMP\plop_crop.png", [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        break
    }
    Start-Sleep -Milliseconds 25
}
"overlay-detected-after-ms: $detected"
if ($detected -ge 0) { "RESULT: OVERLAY WINDOW CONFIRMED" } else { "RESULT: NO OVERLAY" }

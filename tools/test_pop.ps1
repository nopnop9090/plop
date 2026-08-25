Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class DpiFix { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[DpiFix]::SetProcessDPIAware() | Out-Null

$f = New-Object System.Windows.Forms.Form
$f.Text = "PlopTestWindow"
$f.StartPosition = 'Manual'
$f.Bounds = New-Object System.Drawing.Rectangle(400, 300, 500, 350)
$f.TopMost = $true
$null = $f.Show()

function Snap($path) {
    $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen(0, 0, 0, 0, $bmp.Size)
    $g.Dispose()
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function CountColorful($path) {
    $bmp = New-Object System.Drawing.Bitmap($path)
    $count = 0
    for ($y = 260; $y -lt 700; $y += 2) {
        for ($x = 360; $x -lt 940; $x += 2) {
            $c = $bmp.GetPixel($x, $y)
            $max = [Math]::Max($c.R, [Math]::Max($c.G, $c.B))
            $min = [Math]::Min($c.R, [Math]::Min($c.G, $c.B))
            if ($max -gt 90 -and ($max - $min) -gt 60) { $count++ }
        }
    }
    $bmp.Dispose()
    return $count
}

Start-Sleep -Milliseconds 900
Snap "$env:TEMP\plop_before.png"
$f.Close()   # destroy -> plop should honk + confetti at the form's rect
Start-Sleep -Milliseconds 550
Snap "$env:TEMP\plop_during.png"
Start-Sleep -Milliseconds 1300
Snap "$env:TEMP\plop_after.png"

$before = CountColorful "$env:TEMP\plop_before.png"
$during = CountColorful "$env:TEMP\plop_during.png"
$after  = CountColorful "$env:TEMP\plop_after.png"
"before=$before during=$during after=$after"
if ($during -gt ($before + 150)) { "RESULT: CONFETTI DETECTED" } else { "RESULT: NO CONFETTI" }

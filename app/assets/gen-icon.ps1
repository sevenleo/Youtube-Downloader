# Generates icon.ico (256x256 embedded PNG) + icon.png for the app.
Add-Type -AssemblyName System.Drawing
$size = 256
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$rect = New-Object System.Drawing.Rectangle(8, 8, 240, 240)
$p0 = New-Object System.Drawing.Point(8, 8)
$p1 = New-Object System.Drawing.Point(248, 248)
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($p0, $p1, [System.Drawing.Color]::FromArgb(124, 92, 255), [System.Drawing.Color]::FromArgb(58, 40, 122))
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$d = 96
$path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
$path.AddArc($rect.Right - $d, $rect.Y, $d, $d, 270, 90)
$path.AddArc($rect.Right - $d, $rect.Bottom - $d, $d, $d, 0, 90)
$path.AddArc($rect.X, $rect.Bottom - $d, $d, $d, 90, 90)
$path.CloseFigure()
$g.FillPath($brush, $path)
$wb = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$shaft = New-Object System.Drawing.Rectangle(112, 56, 32, 86)
$g.FillRectangle($wb, $shaft)
$pts = @(
    (New-Object System.Drawing.Point(72, 136)),
    (New-Object System.Drawing.Point(184, 136)),
    (New-Object System.Drawing.Point(128, 202))
)
$g.FillPolygon($wb, $pts)
$g.Dispose()
$pngPath = Join-Path $PSScriptRoot "icon.png"
$bmp.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
# Embeds PNG into an .ico file (Vista+ supports PNG entries)
$png = [System.IO.File]::ReadAllBytes($pngPath)
$icoPath = Join-Path $PSScriptRoot "icon.ico"
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([uint16]0)          # reserved
$bw.Write([uint16]1)          # type: icon
$bw.Write([uint16]1)          # 1 image
$bw.Write([byte]0)            # width 256 => 0
$bw.Write([byte]0)            # height 256 => 0
$bw.Write([byte]0)            # palette colors
$bw.Write([byte]0)            # reserved
$bw.Write([uint16]1)          # planes
$bw.Write([uint16]32)         # bpp
$bw.Write([uint32]$png.Length)
$bw.Write([uint32]22)         # offset: 6 header + 16 dir
$bw.Write($png)
$bw.Close(); $fs.Close()
$bmp.Dispose()
Write-Output "OK: $icoPath"

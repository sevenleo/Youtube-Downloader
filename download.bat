@echo off
chcp 65001 >nul
setlocal EnableExtensions EnableDelayedExpansion
title Downloader - yt-dlp + FFmpeg

cd /d "%~dp0"

set "PATH=%~dp0bin;%PATH%"

set "DESTINATION_FILE=%TEMP%\yt_destination_%RANDOM%%RANDOM%.txt"

powershell.exe -NoProfile -STA -Command "Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description='Select destination folder'; $d.ShowNewFolderButton=$true; if($d.ShowDialog() -ne [Windows.Forms.DialogResult]::OK){exit 1}; [IO.File]::WriteAllText($env:DESTINATION_FILE,$d.SelectedPath,(New-Object Text.UTF8Encoding($false)))" >nul 2>&1
if errorlevel 1 goto ERROR_DESTINATION

if not exist "%DESTINATION_FILE%" goto ERROR_DESTINATION

set "OUTPUT_DIR="
set /p "OUTPUT_DIR="<"%DESTINATION_FILE%"
del "%DESTINATION_FILE%" >nul 2>&1

if not defined OUTPUT_DIR goto ERROR_DESTINATION
if not exist "!OUTPUT_DIR!\." goto ERROR_DESTINATION

set "LOG=!OUTPUT_DIR!\download.log"

powershell.exe -NoProfile -Command "try { [IO.File]::WriteAllText($env:LOG,'') } catch { exit 1 }" >nul 2>&1
if errorlevel 1 goto ERROR_DESTINATION

echo ================================================== > "!LOG!"
echo START: %date% %time% >> "!LOG!"
echo ================================================== >> "!LOG!"

echo.
echo ==================================================
echo          VIDEO DOWNLOAD - YT-DLP
echo ==================================================
echo.

where yt-dlp >> "!LOG!" 2>&1
if errorlevel 1 goto ERROR_YTDLP

where ffmpeg >> "!LOG!" 2>&1
if errorlevel 1 goto ERROR_FFMPEG

set "URL="
set /p "URL=Video URL: "

if not defined URL goto ERROR_URL

echo URL: !URL! >> "!LOG!"

echo.
echo Getting title and available resolutions...
echo Getting title and available resolutions... >> "!LOG!"

set "VIDEO_DATA_FILE=%TEMP%\yt_video_data_%RANDOM%%RANDOM%.json"
set "TITLE_FILE=%TEMP%\yt_title_%RANDOM%%RANDOM%.txt"
set "RESOLUTION_FILE=%TEMP%\yt_resolutions_%RANDOM%%RANDOM%.txt"

yt-dlp --force-ipv4 --dump-single-json --skip-download --no-warnings "!URL!" > "%VIDEO_DATA_FILE%" 2>> "!LOG!"

if errorlevel 1 goto ERROR_TITLE
if not exist "%VIDEO_DATA_FILE%" goto ERROR_TITLE

powershell.exe -NoProfile -Command "$d=Get-Content -Raw -LiteralPath $env:VIDEO_DATA_FILE | ConvertFrom-Json; if([string]::IsNullOrWhiteSpace([string]$d.title)){exit 1}; [IO.File]::WriteAllText($env:TITLE_FILE,[string]$d.title,(New-Object Text.UTF8Encoding($false))); $h=@($d.formats | Where-Object { $_.vcodec -like 'avc1*' -and [int]$_.height -gt 0 -and [int]$_.width -gt 0 } | Group-Object { [int]$_.height } | Sort-Object { [int]$_.Name } | ForEach-Object { $w=($_.Group | ForEach-Object { [int]$_.width } | Measure-Object -Maximum).Maximum; ([string][int]$_.Name)+' '+([string]$w) }); if(-not $h){exit 2}; [IO.File]::WriteAllLines($env:RESOLUTION_FILE,[string[]]$h,(New-Object Text.UTF8Encoding($false)))" >> "!LOG!" 2>&1

if errorlevel 2 goto ERROR_FORMATS
if errorlevel 1 goto ERROR_TITLE

del "%VIDEO_DATA_FILE%" >nul 2>&1

if not exist "%TITLE_FILE%" goto ERROR_TITLE

set "TITLE="
set /p "TITLE="<"%TITLE_FILE%"
del "%TITLE_FILE%" >nul 2>&1

if not defined TITLE goto ERROR_TITLE

echo Title: !TITLE!
echo TITLE: !TITLE! >> "!LOG!"

set "SAFE_TITLE_FILE=%TEMP%\yt_safe_title_%RANDOM%%RANDOM%.txt"

powershell.exe -NoProfile -Command "$t=$env:TITLE; $invalid=[IO.Path]::GetInvalidFileNameChars(); foreach($c in $invalid){$t=$t.Replace([string]$c,'_')}; $t=$t.Trim(); [IO.File]::WriteAllText($env:SAFE_TITLE_FILE,$t,(New-Object Text.UTF8Encoding($false)))" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_SAFE_TITLE

if not exist "%SAFE_TITLE_FILE%" goto ERROR_SAFE_TITLE

set "SAFE_TITLE="
set /p "SAFE_TITLE="<"%SAFE_TITLE_FILE%"
del "%SAFE_TITLE_FILE%" >nul 2>&1

if not defined SAFE_TITLE goto ERROR_SAFE_TITLE

echo Safe name: !SAFE_TITLE!
echo SAFE_TITLE: !SAFE_TITLE! >> "!LOG!"

:ASK_RESOLUTION
echo.
echo Available H.264 resolutions:
set "RESOLUTION_INDEX=0"
for /f "usebackq tokens=1,2" %%A in ("%RESOLUTION_FILE%") do (
    set /a RESOLUTION_INDEX+=1
    echo !RESOLUTION_INDEX! - %%Ap - %%Bx%%A
)

set "RESOLUTION_OPTION="
set /p "RESOLUTION_OPTION=Choose resolution number [1-!RESOLUTION_INDEX!]: "

set "RESOLUTION="
set "RESOLUTION_INDEX=0"
for /f "usebackq tokens=1,2" %%A in ("%RESOLUTION_FILE%") do (
    set /a RESOLUTION_INDEX+=1
    if "!RESOLUTION_INDEX!"=="!RESOLUTION_OPTION!" set "RESOLUTION=%%A"
)

if not defined RESOLUTION (
    echo.
    echo Invalid option. Choose a number from the list.
    echo INVALID RESOLUTION OPTION: !RESOLUTION_OPTION! >> "!LOG!"
    goto ASK_RESOLUTION
)
del "%RESOLUTION_FILE%" >nul 2>&1

echo RESOLUTION: !RESOLUTION!p >> "!LOG!"

:ASK_FORMAT
echo.
echo Choose output format:
echo.
echo 1 - MKV
echo 2 - MP4
echo.

set "FORMAT_OPTION="
set /p "FORMAT_OPTION=Option [1-2]: "

if "!FORMAT_OPTION!"=="1" goto FORMAT_MKV
if "!FORMAT_OPTION!"=="2" goto FORMAT_MP4
echo.
echo Invalid option. Enter 1 or 2.
echo INVALID FORMAT OPTION: !FORMAT_OPTION! >> "!LOG!"
goto ASK_FORMAT

:FORMAT_MKV
set "FORMAT=mkv"
goto FORMAT_DONE

:FORMAT_MP4
set "FORMAT=mp4"
goto FORMAT_DONE

:FORMAT_DONE

echo FORMAT: !FORMAT! >> "!LOG!"

:ASK_CONTENT
echo.
echo Choose content:
echo.
echo 1 - Full video
echo 2 - Clip only
echo.

set "CONTENT_OPTION="
set /p "CONTENT_OPTION=Option [1-2]: "

if "!CONTENT_OPTION!"=="1" goto MODE_FULL
if "!CONTENT_OPTION!"=="2" goto MODE_CLIP
echo.
echo Invalid option. Enter 1 or 2.
echo INVALID CONTENT OPTION: !CONTENT_OPTION! >> "!LOG!"
goto ASK_CONTENT

:MODE_FULL
set "MODE=full"
set "START="
set "END="
goto MODE_DONE

:MODE_CLIP
set "MODE=clip"

echo.
set "START="
set "END="

set /p "START=Clip start (HH:MM:SS or MM:SS): "
set /p "END=Clip end (HH:MM:SS or MM:SS): "

if not defined START goto ERROR_START
if not defined END goto ERROR_END

echo START: !START! >> "!LOG!"
echo END: !END! >> "!LOG!"

goto MODE_DONE

:MODE_DONE

set "TEMP_DIR=%TEMP%\yt-dlp-video-%RANDOM%%RANDOM%"

mkdir "%TEMP_DIR%" >nul 2>&1

if not exist "%TEMP_DIR%" goto ERROR_TEMP

echo TEMP_DIR: %TEMP_DIR% >> "!LOG!"

echo.
echo ==================================================
echo DOWNLOADING FULL VIDEO
echo ==================================================
echo.

yt-dlp --force-ipv4 --retries 20 --fragment-retries 20 --retry-sleep 2 --print "RESOLVED_FORMAT: %%(format_id)s" --no-simulate --verbose -f "bv*[height=!RESOLUTION!][vcodec^^=avc1]+ba[acodec=opus]/bv*[height=!RESOLUTION!]+ba/b[height=!RESOLUTION!]/bv*[height<=!RESOLUTION!]+ba/best[height<=!RESOLUTION!]" --merge-output-format mkv --remux-video mkv -o "%TEMP_DIR%\source.%%(ext)s" "!URL!" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_DOWNLOAD

if not exist "%TEMP_DIR%\source.mkv" goto ERROR_SOURCE

echo.
echo Download completed successfully.
echo DOWNLOAD COMPLETED >> "!LOG!"

if "!MODE!"=="full" goto PREPARE_FULL

set "START_SAFE=%START::=-%"
set "END_SAFE=%END::=-%"
set "OUTPUT_BASE=!SAFE_TITLE! - clip !START_SAFE!-!END_SAFE!"
set "INPUT=%TEMP_DIR%\clip.mkv"

echo.
echo ==================================================
echo CUTTING CLIP
echo ==================================================
echo.

echo Clip: !START! to !END! >> "!LOG!"

ffmpeg -y -ss "!START!" -to "!END!" -i "%TEMP_DIR%\source.mkv" -map 0 -c copy "%TEMP_DIR%\clip.mkv" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_CUT

if not exist "%TEMP_DIR%\clip.mkv" goto ERROR_CUT

goto PROCESS_OUTPUT

:PREPARE_FULL
set "OUTPUT_BASE=!SAFE_TITLE! - full"
set "INPUT=%TEMP_DIR%\source.mkv"

goto PROCESS_OUTPUT

:PROCESS_OUTPUT

if "!FORMAT!"=="mkv" goto OUTPUT_MKV
if "!FORMAT!"=="mp4" goto OUTPUT_MP4
goto ERROR_FORMAT

:OUTPUT_MKV
set "FINAL_FILE=!OUTPUT_DIR!\!OUTPUT_BASE!.mkv"

echo.
echo Creating final MKV file...
echo FINAL: !FINAL_FILE! >> "!LOG!"

copy /b "%INPUT%" "!FINAL_FILE!" >nul

if errorlevel 1 goto ERROR_FINAL

goto SUCCESS

:OUTPUT_MP4
set "FINAL_FILE=!OUTPUT_DIR!\!OUTPUT_BASE!.mp4"

echo.
echo ==================================================
echo CONVERTING TO MP4
echo ==================================================
echo.

echo FINAL: !FINAL_FILE! >> "!LOG!"

ffmpeg -y -i "%INPUT%" -map 0:v:0 -map 0:a:0 -c:v copy -c:a aac -b:a 320k -movflags +faststart "!FINAL_FILE!" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_CONVERT

goto SUCCESS

:SUCCESS

if not exist "!FINAL_FILE!" goto ERROR_FINAL

rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo ==================================================
echo                     COMPLETED
echo ==================================================
echo.
echo Final file:
echo !FINAL_FILE!
echo.
echo Temporary files removed.
echo.
echo Log:
echo !LOG!
echo.

echo FINISHED: %date% %time% >> "!LOG!"
echo ================================================== >> "!LOG!"

choice /c YN /n /m "Open destination folder? [Y/N] "
if errorlevel 2 goto SUCCESS_OPEN_DONE
start "" "!OUTPUT_DIR!"

:SUCCESS_OPEN_DONE
pause
exit /b 0

:ERROR_DESTINATION
echo.
echo ERROR: Select a valid, writable destination folder.
pause
exit /b 1

:ERROR_RESOLUTION
echo.
echo ERROR: Select a number of an available H.264 resolution.
echo ERROR: Invalid resolution option. >> "!LOG!"
pause
exit /b 1

:ERROR_FORMATS
del "%VIDEO_DATA_FILE%" >nul 2>&1
del "%TITLE_FILE%" >nul 2>&1
del "%RESOLUTION_FILE%" >nul 2>&1
echo.
echo ERROR: No H.264 resolutions found for this video.
echo ERROR: No H.264 resolutions available. >> "!LOG!"
type "!LOG!"
pause
exit /b 1

:ERROR_YTDLP
echo.
echo ERROR: yt-dlp was not found in the bin folder or PATH.
echo See log: !LOG!
echo ERROR: yt-dlp not found. >> "!LOG!"
pause
exit /b 1

:ERROR_FFMPEG
echo.
echo ERROR: ffmpeg was not found in the bin folder or PATH.
echo See log: !LOG!
echo ERROR: ffmpeg not found. >> "!LOG!"
pause
exit /b 1

:ERROR_URL
echo.
echo ERROR: No URL provided.
echo ERROR: Empty URL. >> "!LOG!"
pause
exit /b 1

:ERROR_TITLE
if defined VIDEO_DATA_FILE del "%VIDEO_DATA_FILE%" >nul 2>&1
if defined TITLE_FILE del "%TITLE_FILE%" >nul 2>&1
if defined RESOLUTION_FILE del "%RESOLUTION_FILE%" >nul 2>&1
echo.
echo ERROR: Could not get video title.
echo.
type "!LOG!"
pause
exit /b 1

:ERROR_SAFE_TITLE
echo.
echo ERROR: Could not prepare safe file name.
echo.
type "!LOG!"
pause
exit /b 1

:ERROR_FORMAT
echo.
echo ERROR: Invalid format.
echo ERROR: Invalid format. >> "!LOG!"
pause
exit /b 1

:ERROR_MODE
echo.
echo ERROR: Invalid content option.
echo ERROR: Invalid mode. >> "!LOG!"
pause
exit /b 1

:ERROR_START
echo.
echo ERROR: Clip start cannot be empty.
echo ERROR: Empty START. >> "!LOG!"
pause
exit /b 1

:ERROR_END
echo.
echo ERROR: Clip end cannot be empty.
echo ERROR: Empty END. >> "!LOG!"
pause
exit /b 1

:ERROR_TEMP
echo.
echo ERROR: Could not create temporary folder.
echo ERROR: TEMP_DIR. >> "!LOG!"
pause
exit /b 1

:ERROR_DOWNLOAD
echo.
echo ==================================================
echo ERROR: DOWNLOAD FAILED
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_SOURCE
echo.
echo ERROR: source.mkv was not found.
echo ERROR: source.mkv not found. >> "!LOG!"
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_CUT
echo.
echo ==================================================
echo ERROR: FAILED TO CUT CLIP
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_CONVERT
echo.
echo ==================================================
echo ERROR: MP4 CONVERSION FAILED
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_FINAL
echo.
echo ERROR: Final file was not created.
echo ERROR: Final file not found. >> "!LOG!"
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

@echo off
chcp 65001 >nul
setlocal EnableExtensions EnableDelayedExpansion
title Updater - yt-dlp + FFmpeg

cd /d "%~dp0"

set "BIN_DIR=%~dp0bin"
set "TEMP_DIR=%TEMP%\yt-updater-%RANDOM%%RANDOM%"
set "FFZIP=%TEMP_DIR%\ffmpeg.zip"
set "FFDIR=%TEMP_DIR%\ffmpeg"
set "CUR_YTDLP_FILE=%TEMP_DIR%\cur_ytdlp.txt"
set "CUR_FFMPEG_FILE=%TEMP_DIR%\cur_ffmpeg.txt"
set "LATEST_YTDLP_FILE=%TEMP_DIR%\latest_ytdlp.txt"
set "LATEST_FFMPEG_FILE=%TEMP_DIR%\latest_ffmpeg.txt"
set "FAIL_YTDLP="
set "FAIL_FFMPEG="
set "DO_YTDLP="
set "DO_FFMPEG="

echo.
echo ==================================================
echo      BINARY UPDATER - YT-DLP + FFMPEG
echo ==================================================
echo.

where curl >nul 2>&1
if errorlevel 1 goto ERROR_CURL

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%BIN_DIR%" goto ERROR_BIN

mkdir "%TEMP_DIR%" >nul 2>&1
if not exist "%TEMP_DIR%" goto ERROR_TEMP

echo Checking installed and latest versions...
echo.

powershell.exe -NoProfile -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $enc=New-Object Text.UTF8Encoding($false); $cy=''; $cf=''; $ly=''; $lf=''; if(Test-Path ($env:BIN_DIR + '\yt-dlp.exe')){ try { $cy=[string](& ($env:BIN_DIR + '\yt-dlp.exe') --version 2>$null | Select-Object -First 1) } catch {} }; if(Test-Path ($env:BIN_DIR + '\ffmpeg.exe')){ try { $line=[string](& ($env:BIN_DIR + '\ffmpeg.exe') -version 2>$null | Select-Object -First 1); $m=[regex]::Match($line,'-(\d{8})\s'); if($m.Success){ $cf=$m.Groups[1].Value } elseif($line){ $cf=$line } } catch {} }; try { $ly=[string](Invoke-RestMethod -Uri 'https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest' -Headers @{'User-Agent'='yt-updater'}).tag_name } catch {}; try { $r=Invoke-RestMethod -Uri 'https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest' -Headers @{'User-Agent'='yt-updater'}; $lf=([datetime]$r.published_at).ToUniversalTime().ToString('yyyyMMdd') } catch {}; [IO.File]::WriteAllText($env:CUR_YTDLP_FILE,$cy,$enc); [IO.File]::WriteAllText($env:CUR_FFMPEG_FILE,$cf,$enc); [IO.File]::WriteAllText($env:LATEST_YTDLP_FILE,$ly,$enc); [IO.File]::WriteAllText($env:LATEST_FFMPEG_FILE,$lf,$enc)"

set "CUR_YTDLP="
if exist "%CUR_YTDLP_FILE%" set /p "CUR_YTDLP="<"%CUR_YTDLP_FILE%"
set "CUR_FFMPEG="
if exist "%CUR_FFMPEG_FILE%" set /p "CUR_FFMPEG="<"%CUR_FFMPEG_FILE%"
set "LATEST_YTDLP="
if exist "%LATEST_YTDLP_FILE%" set /p "LATEST_YTDLP="<"%LATEST_YTDLP_FILE%"
set "LATEST_FFMPEG="
if exist "%LATEST_FFMPEG_FILE%" set /p "LATEST_FFMPEG="<"%LATEST_FFMPEG_FILE%"

if "!CUR_YTDLP!"=="" set "CUR_YTDLP=not installed"
if "!CUR_FFMPEG!"=="" set "CUR_FFMPEG=not installed"
if "!LATEST_YTDLP!"=="" set "LATEST_YTDLP=unavailable"
if "!LATEST_FFMPEG!"=="" set "LATEST_FFMPEG=unavailable"

set "STATUS_YTDLP=could not verify"
if /i "!CUR_YTDLP!"=="not installed" if /i not "!LATEST_YTDLP!"=="unavailable" set "STATUS_YTDLP=NOT INSTALLED - INSTALLATION AVAILABLE"
if /i not "!CUR_YTDLP!"=="not installed" if /i not "!LATEST_YTDLP!"=="unavailable" (
    if /i "!CUR_YTDLP!"=="!LATEST_YTDLP!" (
        set "STATUS_YTDLP=YOU ARE ALREADY ON THE LATEST VERSION"
    ) else (
        set "STATUS_YTDLP=UPDATE AVAILABLE"
    )
)

set "STATUS_FFMPEG=could not verify"
if /i "!CUR_FFMPEG!"=="not installed" if /i not "!LATEST_FFMPEG!"=="unavailable" set "STATUS_FFMPEG=NOT INSTALLED - INSTALLATION AVAILABLE"
if /i not "!CUR_FFMPEG!"=="not installed" if /i not "!LATEST_FFMPEG!"=="unavailable" (
    if /i "!CUR_FFMPEG!"=="!LATEST_FFMPEG!" (
        set "STATUS_FFMPEG=YOU ARE ALREADY ON THE LATEST VERSION"
    ) else (
        set "STATUS_FFMPEG=UPDATE AVAILABLE"
    )
)

echo ==================================================
echo               VERSION COMPARISON
echo ==================================================
echo.
echo YT-DLP
echo   Installed    : !CUR_YTDLP!
echo   Latest       : !LATEST_YTDLP!
echo   Status       : !STATUS_YTDLP!
echo.
echo FFMPEG ^(build date^)
echo   Installed    : !CUR_FFMPEG!
echo   Latest       : !LATEST_FFMPEG!
echo   Status       : !STATUS_FFMPEG!
echo.

:MENU
echo ==================================================
echo WHAT DO YOU WANT TO UPDATE?
echo ==================================================
echo.
echo   1 - yt-dlp only
echo   2 - FFmpeg only
echo   3 - Both
echo   0 - Exit without updating
echo.
echo Tip: use "update.bat 1", "2", "3" or "0" to skip this menu.
echo.

set "OPTION=%~1"
if not defined OPTION set /p "OPTION=Option [0-3]: "

if "!OPTION!"=="0" goto END_NO_UPDATE

if "!OPTION!"=="1" (
    set "DO_YTDLP=1"
    goto CHECK_YTDLP
)

if "!OPTION!"=="2" (
    set "DO_FFMPEG=1"
    goto CHECK_FFMPEG
)

if "!OPTION!"=="3" (
    set "DO_YTDLP=1"
    set "DO_FFMPEG=1"
    goto CHECK_YTDLP
)

if not "%~1"=="" (
    echo.
    echo Invalid parameter: %~1. Use 0, 1, 2 or 3.
    rmdir /s /q "%TEMP_DIR%" >nul 2>&1
    exit /b 2
)

echo.
echo Invalid option. Enter a number from 0 to 3.
goto MENU

:CHECK_YTDLP
if not defined DO_YTDLP goto CHECK_FFMPEG

echo.
echo ==================================================
echo UPDATING YT-DLP
echo Official source: github.com/yt-dlp/yt-dlp
echo ==================================================
echo.

curl -fL --retry 3 -o "%TEMP_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

if errorlevel 1 (
    echo.
    echo Official source failed. Trying trusted alternative source...
    echo Source: github.com/yt-dlp/yt-dlp-nightly-builds
    echo.
    curl -fL --retry 3 -o "%TEMP_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp.exe"
)

if errorlevel 1 (
    set "FAIL_YTDLP=1"
    echo.
    echo ERROR: could not download yt-dlp from any source.
    goto CHECK_FFMPEG
)

copy /y "%TEMP_DIR%\yt-dlp.exe" "%BIN_DIR%\yt-dlp.exe" >nul
if errorlevel 1 (
    set "FAIL_YTDLP=1"
    echo ERROR: could not copy yt-dlp.exe to bin folder.
    goto CHECK_FFMPEG
)

echo.
echo yt-dlp updated:
"%BIN_DIR%\yt-dlp.exe" --version

:CHECK_FFMPEG
if not defined DO_FFMPEG goto SUMMARY

echo.
echo ==================================================
echo UPDATING FFMPEG
echo Official source: github.com/BtbN/FFmpeg-Builds
echo ^(recommended build on ffmpeg.org^)
echo ==================================================
echo.

curl -fL --retry 3 -o "%FFZIP%" "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip"

if errorlevel 1 (
    echo.
    echo Official source failed. Trying trusted alternative source...
    echo Source: gyan.dev ^(recommended build on ffmpeg.org^)
    echo.
    curl -fL --retry 3 -o "%FFZIP%" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
)

if errorlevel 1 (
    set "FAIL_FFMPEG=1"
    echo.
    echo ERROR: could not download FFmpeg from any source.
    goto SUMMARY
)

echo.
echo Extracting package...

powershell.exe -NoProfile -Command "try { Expand-Archive -LiteralPath $env:FFZIP -DestinationPath $env:FFDIR -Force } catch { exit 1 }"

if errorlevel 1 (
    set "FAIL_FFMPEG=1"
    echo ERROR: could not extract FFmpeg package.
    goto SUMMARY
)

set "FFBIN="
for /d %%D in ("%FFDIR%\*") do if exist "%%D\bin\ffmpeg.exe" set "FFBIN=%%D\bin"

if not defined FFBIN (
    set "FAIL_FFMPEG=1"
    echo ERROR: ffmpeg.exe was not found in the package.
    goto SUMMARY
)

echo Installing ffmpeg.exe, ffplay.exe and ffprobe.exe to bin...

copy /y "%FFBIN%\ffmpeg.exe"  "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"
copy /y "%FFBIN%\ffplay.exe"  "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"
copy /y "%FFBIN%\ffprobe.exe" "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"

if defined FAIL_FFMPEG (
    echo ERROR: could not copy FFmpeg binaries.
    goto SUMMARY
)

echo.
echo FFmpeg updated:
"%BIN_DIR%\ffmpeg.exe" -version 2>nul | findstr /b "ffmpeg version"

:SUMMARY
rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo ==================================================
echo                     SUMMARY
echo ==================================================
echo.
if defined DO_YTDLP (
    if defined FAIL_YTDLP (
        echo yt-dlp:  FAILED
    ) else (
        echo yt-dlp:  OK
    )
)
if defined DO_FFMPEG (
    if defined FAIL_FFMPEG (
        echo FFmpeg:  FAILED
    ) else (
        echo FFmpeg:  OK
    )
)
echo.

set "ANYFAIL="
if defined DO_YTDLP if defined FAIL_YTDLP set "ANYFAIL=1"
if defined DO_FFMPEG if defined FAIL_FFMPEG set "ANYFAIL=1"

if defined ANYFAIL goto SUMMARY_FAIL

echo Update completed successfully.
echo.
pause
exit /b 0

:END_NO_UPDATE
rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo No changes were made.
echo.
pause
exit /b 0

:SUMMARY_FAIL
echo Some items could not be updated.
echo Check your internet connection and try again.
echo.
pause
exit /b 1

:ERROR_CURL
echo.
echo ERROR: curl was not found on the system.
echo curl is built into Windows 10/11. Update Windows
echo or install curl manually.
echo.
pause
exit /b 1

:ERROR_BIN
echo.
echo ERROR: could not create bin folder.
echo.
pause
exit /b 1

:ERROR_TEMP
echo.
echo ERROR: could not create temporary folder.
echo.
pause
exit /b 1

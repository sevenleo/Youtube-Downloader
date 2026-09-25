@echo off
chcp 65001 >nul
setlocal EnableExtensions EnableDelayedExpansion
title Downloader - yt-dlp + FFmpeg

cd /d "%~dp0"

set "DESTINATION_FILE=%TEMP%\yt_destination_%RANDOM%%RANDOM%.txt"

powershell.exe -NoProfile -STA -Command "Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description='Selecione a pasta de destino'; $d.ShowNewFolderButton=$true; if($d.ShowDialog() -ne [Windows.Forms.DialogResult]::OK){exit 1}; [IO.File]::WriteAllText($env:DESTINATION_FILE,$d.SelectedPath,(New-Object Text.UTF8Encoding($false)))" >nul 2>&1
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
echo INICIO: %date% %time% >> "!LOG!"
echo ================================================== >> "!LOG!"

echo.
echo ==================================================
echo          DOWNLOAD DE VIDEO - YT-DLP
echo ==================================================
echo.

where yt-dlp >> "!LOG!" 2>&1
if errorlevel 1 goto ERROR_YTDLP

where ffmpeg >> "!LOG!" 2>&1
if errorlevel 1 goto ERROR_FFMPEG

set "URL="
set /p "URL=Link do video: "

if not defined URL goto ERROR_URL

echo URL: !URL! >> "!LOG!"

echo.
echo Obtendo titulo e resolucoes disponiveis...
echo Obtendo titulo e resolucoes disponiveis... >> "!LOG!"

set "VIDEO_DATA_FILE=%TEMP%\yt_video_data_%RANDOM%%RANDOM%.json"
set "TITLE_FILE=%TEMP%\yt_title_%RANDOM%%RANDOM%.txt"
set "RESOLUTION_FILE=%TEMP%\yt_resolutions_%RANDOM%%RANDOM%.txt"

yt-dlp --force-ipv4 --dump-single-json --skip-download --no-warnings "!URL!" > "%VIDEO_DATA_FILE%" 2>> "!LOG!"

if errorlevel 1 goto ERROR_TITLE
if not exist "%VIDEO_DATA_FILE%" goto ERROR_TITLE

powershell.exe -NoProfile -Command "$d=Get-Content -Raw -LiteralPath $env:VIDEO_DATA_FILE | ConvertFrom-Json; if([string]::IsNullOrWhiteSpace([string]$d.title)){exit 1}; [IO.File]::WriteAllText($env:TITLE_FILE,[string]$d.title,(New-Object Text.UTF8Encoding($false))); $h=@($d.formats | Where-Object { $_.vcodec -like 'avc1*' -and $_.height -gt 0 } | ForEach-Object { [int]$_.height } | Sort-Object -Unique); if(-not $h){exit 2}; [IO.File]::WriteAllLines($env:RESOLUTION_FILE,[string[]]$h,(New-Object Text.UTF8Encoding($false)))" >> "!LOG!" 2>&1

if errorlevel 2 goto ERROR_FORMATS
if errorlevel 1 goto ERROR_TITLE

del "%VIDEO_DATA_FILE%" >nul 2>&1

if not exist "%TITLE_FILE%" goto ERROR_TITLE

set "TITLE="
set /p "TITLE="<"%TITLE_FILE%"
del "%TITLE_FILE%" >nul 2>&1

if not defined TITLE goto ERROR_TITLE

echo Titulo: !TITLE!
echo TITULO: !TITLE! >> "!LOG!"

set "SAFE_TITLE_FILE=%TEMP%\yt_safe_title_%RANDOM%%RANDOM%.txt"

powershell.exe -NoProfile -Command "$t=$env:TITLE; $invalid=[IO.Path]::GetInvalidFileNameChars(); foreach($c in $invalid){$t=$t.Replace([string]$c,'_')}; $t=$t.Trim(); [IO.File]::WriteAllText($env:SAFE_TITLE_FILE,$t,(New-Object Text.UTF8Encoding($false)))" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_SAFE_TITLE

if not exist "%SAFE_TITLE_FILE%" goto ERROR_SAFE_TITLE

set "SAFE_TITLE="
set /p "SAFE_TITLE="<"%SAFE_TITLE_FILE%"
del "%SAFE_TITLE_FILE%" >nul 2>&1

if not defined SAFE_TITLE goto ERROR_SAFE_TITLE

echo Nome seguro: !SAFE_TITLE!
echo SAFE_TITLE: !SAFE_TITLE! >> "!LOG!"

echo.
echo Resolucoes H.264 disponiveis:
set "RESOLUTION_INDEX=0"
for /f "usebackq delims=" %%R in ("%RESOLUTION_FILE%") do (
    set /a RESOLUTION_INDEX+=1
    echo !RESOLUTION_INDEX! - %%Rp
)

set "RESOLUTION_OPTION="
set /p "RESOLUTION_OPTION=Escolha o numero da resolucao [1-!RESOLUTION_INDEX!]: "

set "RESOLUTION="
set "RESOLUTION_INDEX=0"
for /f "usebackq delims=" %%R in ("%RESOLUTION_FILE%") do (
    set /a RESOLUTION_INDEX+=1
    if "!RESOLUTION_INDEX!"=="!RESOLUTION_OPTION!" set "RESOLUTION=%%R"
)
del "%RESOLUTION_FILE%" >nul 2>&1

if not defined RESOLUTION goto ERROR_RESOLUTION

echo RESOLUCAO: !RESOLUTION!p >> "!LOG!"

echo.
echo Escolha o formato de saida:
echo.
echo 1 - MKV
echo 2 - MP4
echo.

set "FORMAT_OPTION="
set /p "FORMAT_OPTION=Opcao [1-2]: "

if "%FORMAT_OPTION%"=="1" goto FORMAT_MKV
if "%FORMAT_OPTION%"=="2" goto FORMAT_MP4
goto ERROR_FORMAT

:FORMAT_MKV
set "FORMAT=mkv"
goto FORMAT_DONE

:FORMAT_MP4
set "FORMAT=mp4"
goto FORMAT_DONE

:FORMAT_DONE

echo FORMATO: %FORMAT% >> "!LOG!"

echo.
echo Escolha o conteudo:
echo.
echo 1 - Video completo
echo 2 - Apenas um trecho
echo.

set "CONTENT_OPTION="
set /p "CONTENT_OPTION=Opcao [1-2]: "

if "%CONTENT_OPTION%"=="1" goto MODE_FULL
if "%CONTENT_OPTION%"=="2" goto MODE_CLIP
goto ERROR_MODE

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

set /p "START=Inicio do trecho (HH:MM:SS ou MM:SS): "
set /p "END=Final do trecho (HH:MM:SS ou MM:SS): "

if not defined START goto ERROR_START
if not defined END goto ERROR_END

echo INICIO: !START! >> "!LOG!"
echo FINAL: !END! >> "!LOG!"

goto MODE_DONE

:MODE_DONE

set "TEMP_DIR=%TEMP%\yt-dlp-video-%RANDOM%%RANDOM%"

mkdir "%TEMP_DIR%" >nul 2>&1

if not exist "%TEMP_DIR%" goto ERROR_TEMP

echo PASTA TEMPORARIA: %TEMP_DIR% >> "!LOG!"

echo.
echo ==================================================
echo BAIXANDO VIDEO COMPLETO
echo ==================================================
echo.

yt-dlp --force-ipv4 --retries 20 --fragment-retries 20 --retry-sleep 2 -f "bv*[height=!RESOLUTION!][vcodec^=avc1]+ba[acodec=opus]" --merge-output-format mkv -o "%TEMP_DIR%\source.%%(ext)s" "!URL!" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_DOWNLOAD

if not exist "%TEMP_DIR%\source.mkv" goto ERROR_SOURCE

echo.
echo Download concluido com sucesso.
echo DOWNLOAD CONCLUIDO >> "!LOG!"

if "%MODE%"=="full" goto PREPARE_FULL

set "START_SAFE=%START::=-%"
set "END_SAFE=%END::=-%"
set "OUTPUT_BASE=!SAFE_TITLE! - trecho !START_SAFE!-!END_SAFE!"
set "INPUT=%TEMP_DIR%\trecho.mkv"

echo.
echo ==================================================
echo CORTANDO TRECHO
echo ==================================================
echo.

echo Corte: !START! ate !END! >> "!LOG!"

ffmpeg -y -ss "!START!" -to "!END!" -i "%TEMP_DIR%\source.mkv" -map 0 -c copy "%TEMP_DIR%\trecho.mkv" >> "!LOG!" 2>&1

if errorlevel 1 goto ERROR_CUT

if not exist "%TEMP_DIR%\trecho.mkv" goto ERROR_CUT

goto PROCESS_OUTPUT

:PREPARE_FULL
set "OUTPUT_BASE=!SAFE_TITLE! - completo"
set "INPUT=%TEMP_DIR%\source.mkv"

goto PROCESS_OUTPUT

:PROCESS_OUTPUT

if "%FORMAT%"=="mkv" goto OUTPUT_MKV
if "%FORMAT%"=="mp4" goto OUTPUT_MP4
goto ERROR_FORMAT

:OUTPUT_MKV
set "FINAL_FILE=!OUTPUT_DIR!\!OUTPUT_BASE!.mkv"

echo.
echo Criando arquivo MKV final...
echo FINAL: !FINAL_FILE! >> "!LOG!"

copy /b "%INPUT%" "!FINAL_FILE!" >nul

if errorlevel 1 goto ERROR_FINAL

goto SUCCESS

:OUTPUT_MP4
set "FINAL_FILE=!OUTPUT_DIR!\!OUTPUT_BASE!.mp4"

echo.
echo ==================================================
echo CONVERTENDO PARA MP4
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
echo                 CONCLUIDO
echo ==================================================
echo.
echo Arquivo final:
echo !FINAL_FILE!
echo.
echo Temporarios removidos.
echo.
echo Log:
echo !LOG!
echo.

echo FINALIZADO: %date% %time% >> "!LOG!"
echo ================================================== >> "!LOG!"

choice /c SN /n /m "Abrir a pasta de destino? [S/N] "
if errorlevel 2 goto SUCCESS_OPEN_DONE
start "" "!OUTPUT_DIR!"

:SUCCESS_OPEN_DONE
pause
exit /b 0

:ERROR_DESTINATION
echo.
echo ERRO: selecione uma pasta de destino valida e gravavel.
pause
exit /b 1

:ERROR_RESOLUTION
echo.
echo ERRO: selecione o numero de uma resolucao H.264 disponivel.
echo ERRO: opcao de resolucao invalida. >> "!LOG!"
pause
exit /b 1

:ERROR_FORMATS
del "%VIDEO_DATA_FILE%" >nul 2>&1
del "%TITLE_FILE%" >nul 2>&1
del "%RESOLUTION_FILE%" >nul 2>&1
echo.
echo ERRO: nao foram encontradas resolucoes H.264 para este video.
echo ERRO: nenhuma resolucao H.264 disponivel. >> "!LOG!"
type "!LOG!"
pause
exit /b 1

:ERROR_YTDLP
echo.
echo ERRO: yt-dlp nao foi encontrado no PATH.
echo Consulte o log: !LOG!
echo ERRO: yt-dlp nao encontrado. >> "!LOG!"
pause
exit /b 1

:ERROR_FFMPEG
echo.
echo ERRO: ffmpeg nao foi encontrado no PATH.
echo Consulte o log: !LOG!
echo ERRO: ffmpeg nao encontrado. >> "!LOG!"
pause
exit /b 1

:ERROR_URL
echo.
echo ERRO: nenhum link foi informado.
echo ERRO: URL vazia. >> "!LOG!"
pause
exit /b 1

:ERROR_TITLE
if defined VIDEO_DATA_FILE del "%VIDEO_DATA_FILE%" >nul 2>&1
if defined TITLE_FILE del "%TITLE_FILE%" >nul 2>&1
if defined RESOLUTION_FILE del "%RESOLUTION_FILE%" >nul 2>&1
echo.
echo ERRO: nao foi possivel obter o titulo.
echo.
type "!LOG!"
pause
exit /b 1

:ERROR_SAFE_TITLE
echo.
echo ERRO: nao foi possivel preparar o nome do arquivo.
echo.
type "!LOG!"
pause
exit /b 1

:ERROR_FORMAT
echo.
echo ERRO: formato invalido.
echo ERRO: formato invalido. >> "!LOG!"
pause
exit /b 1

:ERROR_MODE
echo.
echo ERRO: opcao de conteudo invalida.
echo ERRO: modo invalido. >> "!LOG!"
pause
exit /b 1

:ERROR_START
echo.
echo ERRO: o inicio do trecho nao pode ficar vazio.
echo ERRO: START vazio. >> "!LOG!"
pause
exit /b 1

:ERROR_END
echo.
echo ERRO: o final do trecho nao pode ficar vazio.
echo ERRO: END vazio. >> "!LOG!"
pause
exit /b 1

:ERROR_TEMP
echo.
echo ERRO: nao foi possivel criar a pasta temporaria.
echo ERRO: TEMP_DIR. >> "!LOG!"
pause
exit /b 1

:ERROR_DOWNLOAD
echo.
echo ==================================================
echo ERRO: FALHA NO DOWNLOAD
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_SOURCE
echo.
echo ERRO: source.mkv nao foi encontrado.
echo ERRO: source.mkv inexistente. >> "!LOG!"
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_CUT
echo.
echo ==================================================
echo ERRO: FALHA AO CORTAR O TRECHO
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_CONVERT
echo.
echo ==================================================
echo ERRO: FALHA NA CONVERSAO PARA MP4
echo ==================================================
echo.
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

:ERROR_FINAL
echo.
echo ERRO: arquivo final nao foi criado.
echo ERRO: arquivo final inexistente. >> "!LOG!"
type "!LOG!"
rmdir /s /q "%TEMP_DIR%" >nul 2>&1
pause
exit /b 1

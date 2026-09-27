@echo off
chcp 65001 >nul
setlocal EnableExtensions EnableDelayedExpansion
title Atualizador - yt-dlp + FFmpeg

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
echo      ATUALIZADOR DE BINARIOS - YT-DLP + FFMPEG
echo ==================================================
echo.

where curl >nul 2>&1
if errorlevel 1 goto ERROR_CURL

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%BIN_DIR%" goto ERROR_BIN

mkdir "%TEMP_DIR%" >nul 2>&1
if not exist "%TEMP_DIR%" goto ERROR_TEMP

echo Consultando as versoes instaladas e as mais recentes...
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

if "!CUR_YTDLP!"=="" set "CUR_YTDLP=nao instalado"
if "!CUR_FFMPEG!"=="" set "CUR_FFMPEG=nao instalado"
if "!LATEST_YTDLP!"=="" set "LATEST_YTDLP=indisponivel"
if "!LATEST_FFMPEG!"=="" set "LATEST_FFMPEG=indisponivel"

set "STATUS_YTDLP=nao foi possivel verificar"
if /i "!CUR_YTDLP!"=="nao instalado" if /i not "!LATEST_YTDLP!"=="indisponivel" set "STATUS_YTDLP=NAO INSTALADO - INSTALACAO DISPONIVEL"
if /i not "!CUR_YTDLP!"=="nao instalado" if /i not "!LATEST_YTDLP!"=="indisponivel" (
    if /i "!CUR_YTDLP!"=="!LATEST_YTDLP!" (
        set "STATUS_YTDLP=VOCE JA ESTA NA VERSAO MAIS RECENTE"
    ) else (
        set "STATUS_YTDLP=ATUALIZACAO DISPONIVEL"
    )
)

set "STATUS_FFMPEG=nao foi possivel verificar"
if /i "!CUR_FFMPEG!"=="nao instalado" if /i not "!LATEST_FFMPEG!"=="indisponivel" set "STATUS_FFMPEG=NAO INSTALADO - INSTALACAO DISPONIVEL"
if /i not "!CUR_FFMPEG!"=="nao instalado" if /i not "!LATEST_FFMPEG!"=="indisponivel" (
    if /i "!CUR_FFMPEG!"=="!LATEST_FFMPEG!" (
        set "STATUS_FFMPEG=VOCE JA ESTA NA VERSAO MAIS RECENTE"
    ) else (
        set "STATUS_FFMPEG=ATUALIZACAO DISPONIVEL"
    )
)

echo ==================================================
echo               COMPARACAO DE VERSOES
echo ==================================================
echo.
echo YT-DLP
echo   Instalada    : !CUR_YTDLP!
echo   Mais recente : !LATEST_YTDLP!
echo   Status       : !STATUS_YTDLP!
echo.
echo FFMPEG ^(data do build^)
echo   Instalada    : !CUR_FFMPEG!
echo   Mais recente : !LATEST_FFMPEG!
echo   Status       : !STATUS_FFMPEG!
echo.

:MENU
echo ==================================================
echo O QUE DESEJA ATUALIZAR?
echo ==================================================
echo.
echo   1 - Apenas yt-dlp
echo   2 - Apenas FFmpeg
echo   3 - Ambos
echo   0 - Sair sem atualizar
echo.
echo Dica: use "update.bat 1", "2", "3" ou "0" para pular este menu.
echo.

set "OPTION=%~1"
if not defined OPTION set /p "OPTION=Opcao [0-3]: "

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
    echo Parametro invalido: %~1. Use 0, 1, 2 ou 3.
    rmdir /s /q "%TEMP_DIR%" >nul 2>&1
    exit /b 2
)

echo.
echo Opcao invalida. Digite um numero de 0 a 3.
goto MENU

:CHECK_YTDLP
if not defined DO_YTDLP goto CHECK_FFMPEG

echo.
echo ==================================================
echo ATUALIZANDO YT-DLP
echo Fonte oficial: github.com/yt-dlp/yt-dlp
echo ==================================================
echo.

curl -fL --retry 3 -o "%TEMP_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

if errorlevel 1 (
    echo.
    echo Falha na fonte oficial. Tentando fonte alternativa confiavel...
    echo Fonte: github.com/yt-dlp/yt-dlp-nightly-builds
    echo.
    curl -fL --retry 3 -o "%TEMP_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp.exe"
)

if errorlevel 1 (
    set "FAIL_YTDLP=1"
    echo.
    echo ERRO: nao foi possivel baixar o yt-dlp de nenhuma fonte.
    goto CHECK_FFMPEG
)

copy /y "%TEMP_DIR%\yt-dlp.exe" "%BIN_DIR%\yt-dlp.exe" >nul
if errorlevel 1 (
    set "FAIL_YTDLP=1"
    echo ERRO: nao foi possivel copiar yt-dlp.exe para a pasta bin.
    goto CHECK_FFMPEG
)

echo.
echo yt-dlp atualizado:
"%BIN_DIR%\yt-dlp.exe" --version

:CHECK_FFMPEG
if not defined DO_FFMPEG goto SUMMARY

echo.
echo ==================================================
echo ATUALIZANDO FFMPEG
echo Fonte oficial: github.com/BtbN/FFmpeg-Builds
echo ^(build recomendado em ffmpeg.org^)
echo ==================================================
echo.

curl -fL --retry 3 -o "%FFZIP%" "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip"

if errorlevel 1 (
    echo.
    echo Falha na fonte oficial. Tentando fonte alternativa confiavel...
    echo Fonte: gyan.dev ^(build recomendado em ffmpeg.org^)
    echo.
    curl -fL --retry 3 -o "%FFZIP%" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
)

if errorlevel 1 (
    set "FAIL_FFMPEG=1"
    echo.
    echo ERRO: nao foi possivel baixar o FFmpeg de nenhuma fonte.
    goto SUMMARY
)

echo.
echo Extraindo pacote...

powershell.exe -NoProfile -Command "try { Expand-Archive -LiteralPath $env:FFZIP -DestinationPath $env:FFDIR -Force } catch { exit 1 }"

if errorlevel 1 (
    set "FAIL_FFMPEG=1"
    echo ERRO: nao foi possivel extrair o pacote do FFmpeg.
    goto SUMMARY
)

set "FFBIN="
for /d %%D in ("%FFDIR%\*") do if exist "%%D\bin\ffmpeg.exe" set "FFBIN=%%D\bin"

if not defined FFBIN (
    set "FAIL_FFMPEG=1"
    echo ERRO: ffmpeg.exe nao foi encontrado no pacote.
    goto SUMMARY
)

echo Instalando ffmpeg.exe, ffplay.exe e ffprobe.exe em bin...

copy /y "%FFBIN%\ffmpeg.exe"  "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"
copy /y "%FFBIN%\ffplay.exe"  "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"
copy /y "%FFBIN%\ffprobe.exe" "%BIN_DIR%\" >nul || set "FAIL_FFMPEG=1"

if defined FAIL_FFMPEG (
    echo ERRO: nao foi possivel copiar os binarios do FFmpeg.
    goto SUMMARY
)

echo.
echo FFmpeg atualizado:
"%BIN_DIR%\ffmpeg.exe" -version 2>nul | findstr /b "ffmpeg version"

:SUMMARY
rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo ==================================================
echo                     RESUMO
echo ==================================================
echo.
if defined DO_YTDLP (
    if defined FAIL_YTDLP (
        echo yt-dlp:  FALHOU
    ) else (
        echo yt-dlp:  OK
    )
)
if defined DO_FFMPEG (
    if defined FAIL_FFMPEG (
        echo FFmpeg:  FALHOU
    ) else (
        echo FFmpeg:  OK
    )
)
echo.

set "ANYFAIL="
if defined DO_YTDLP if defined FAIL_YTDLP set "ANYFAIL=1"
if defined DO_FFMPEG if defined FAIL_FFMPEG set "ANYFAIL=1"

if defined ANYFAIL goto SUMMARY_FAIL

echo Atualizacao concluida com sucesso.
echo.
pause
exit /b 0

:END_NO_UPDATE
rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo Nenhuma alteracao foi feita.
echo.
pause
exit /b 0

:SUMMARY_FAIL
echo Alguns itens nao puderam ser atualizados.
echo Verifique sua conexao com a internet e tente novamente.
echo.
pause
exit /b 1

:ERROR_CURL
echo.
echo ERRO: curl nao foi encontrado no sistema.
echo O curl faz parte do Windows 10/11. Atualize o Windows
echo ou instale o curl manualmente.
echo.
pause
exit /b 1

:ERROR_BIN
echo.
echo ERRO: nao foi possivel criar a pasta bin.
echo.
pause
exit /b 1

:ERROR_TEMP
echo.
echo ERRO: nao foi possivel criar a pasta temporaria.
echo.
pause
exit /b 1

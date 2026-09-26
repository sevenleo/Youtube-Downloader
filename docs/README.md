# Downloader - yt-dlp + FFmpeg

Script para Windows que automatiza o download de vídeos utilizando **yt-dlp** e **FFmpeg**.

O programa permite escolher:

* Pasta de destino dos arquivos finais
* URL do vídeo
* Formato de saída: **MKV ou MP4**
* Resoluções H.264 realmente disponíveis, listadas por número e dimensão
* Vídeo completo ou apenas um trecho
* Horário inicial e final do trecho
* Se a pasta de destino deve ser aberta após o download
* Seleção alternativa de formatos quando a combinação preferida não está disponível
* Log detalhado com os IDs dos formatos selecionados

O processo foi desenvolvido para priorizar **confiabilidade do download e sincronização entre áudio e vídeo**, mantendo o vídeo sem recodificação sempre que possível.

---

## Requisitos

O Windows deve ter os seguintes programas disponíveis no `PATH` ou na mesma pasta de `download.bat`:

* `yt-dlp`
* `ffmpeg`

Para verificar o `yt-dlp`:

```text
yt-dlp --version
```

Para verificar o FFmpeg:

```text
ffmpeg -version
```

Para verificar de onde os executáveis estão sendo carregados:

```text
where yt-dlp
where ffmpeg
```

O script também utiliza o **PowerShell**, que normalmente já está disponível no Windows.

---

## Arquivos do projeto

A estrutura recomendada é:

```text
Downloader/
│
├── download.bat
├── yt-dlp.exe
├── ffmpeg.exe
└── docs/
    └── README.md
```

O arquivo `download.log` é criado automaticamente na pasta de destino selecionada e não precisa existir previamente.

---

## Como executar

Execute:

```text
download.bat
```

O script abrirá um seletor de pasta e, em seguida, fará as perguntas do download.

### 1. Escolha a pasta de destino

Selecione a pasta onde o arquivo final e o `download.log` serão gravados. Os arquivos temporários continuam sendo criados em `%TEMP%` e são removidos ao concluir.

### 2. Informe o link

Cole a URL do vídeo quando solicitado:

```text
Link do video: https://www.youtube.com/watch?v=xxxxxxxxxxx
```

### 3. Escolha a resolução

O script consulta as resoluções H.264 que realmente existem no vídeo e mostra cada uma com um número, altura e dimensão (por exemplo `4 - 480p - 854x480`). Digite somente o número da opção. O download prefere vídeo H.264 nessa resolução com áudio Opus; se essa combinação não estiver disponível, tenta outros formatos na mesma resolução, um arquivo único progressivo ou a resolução mais próxima abaixo. O codec pode variar nesses fallbacks. O log registra os IDs exatos dos formatos selecionados como `FORMATO_RESOLVIDO`.

### 4. Escolha o formato

```text
1 - MKV
2 - MP4
```

### MKV

Mantém os codecs dos streams baixados sem recodificá-los. A combinação preferida é:

```text
Vídeo: H.264
Áudio: Opus
```

Essa é a opção que preserva melhor os codecs originais e, consequentemente, a qualidade original do vídeo e do áudio.

### MP4

O vídeo é mantido sem recodificação, desde que o codec selecionado seja compatível com MP4:

```text
Vídeo: codec selecionado, copiado sem recodificação
```

O áudio é convertido para:

```text
Áudio: AAC 320 kbps
```

Essa opção oferece maior compatibilidade com players, dispositivos e softwares que não trabalham bem com Opus dentro de MP4.

---

### 5. Escolha o conteúdo

```text
1 - Video completo
2 - Apenas um trecho
```

### Vídeo completo

O vídeo inteiro é baixado e convertido para o formato escolhido.

### Trecho

O script solicitará:

```text
Inicio do trecho (HH:MM:SS ou MM:SS):
Final do trecho (HH:MM:SS ou MM:SS):
```

Exemplo:

```text
Inicio do trecho: 00:03:01
Final do trecho: 00:04:50
```

Também é possível usar:

```text
03:01
04:50
```

---

# Funcionamento interno

O script não baixa diretamente apenas o trecho solicitado.

Para reduzir problemas de sincronização e de falhas durante o download dos streams separados, o script normaliza o arquivo intermediário para MKV por mesclagem ou remux, sem recodificar os streams. Depois utiliza o seguinte processo:

```text
                 YouTube
                    │
                    ▼
          yt-dlp baixa o vídeo
                    │
                    ▼
      Streams selecionados
      (preferência H.264 + Opus)
                    │
                    ▼
              source.mkv
                    │
           ┌────────┴────────┐
           │                 │
      Vídeo completo      Trecho
           │                 │
           │          FFmpeg faz o corte
           │                 │
           │           trecho.mkv
           │                 │
           └────────┬────────┘
                    │
                    ▼
             Formato escolhido
               /            \
             MKV             MP4
              │               │
               │       Vídeo compatível é copiado
               │       Áudio → AAC 320 kbps
              │               │
              └───────┬───────┘
                      ▼
                Arquivo final
```

---

# Download

O `yt-dlp` é instruído a utilizar preferencialmente:

```text
Vídeo: H.264 (avc1)
Áudio: Opus
```

O comando utilizado pelo script também inclui mecanismos para aumentar a confiabilidade do download:

```text
--force-ipv4
--retries 20
--fragment-retries 20
--retry-sleep 2
--print "FORMATO_RESOLVIDO: %(format_id)s"
--no-simulate
--merge-output-format mkv
--remux-video mkv
--verbose
```

### IPv4

O `--force-ipv4` evita problemas relacionados a determinadas rotas IPv6 ou conexões com os servidores CDN do YouTube.

### Tentativas

Em caso de falha temporária durante o download, o `yt-dlp` tenta novamente.

O ID do formato escolhido é registrado em `FORMATO_RESOLVIDO`. `--no-simulate` garante que o download continue mesmo com essa opção de registro, e `--verbose` grava detalhes úteis para diagnosticar falhas.

---

# Corte do vídeo

Quando um trecho é solicitado, o arquivo completo é baixado primeiro.

Depois o FFmpeg realiza o corte localmente:

```text
source.mkv
      ↓
FFmpeg
      ↓
trecho.mkv
```

O corte utiliza:

```text
-c copy
```

Isso significa que o vídeo e o áudio não são recodificados durante o corte.

Consequentemente, não há perda adicional de qualidade nessa etapa.

---

# Conversão para MP4

Quando MP4 é selecionado, o script utiliza:

```text
Vídeo → copy
Áudio → AAC 320 kbps
```

O vídeo H.264 não é recodificado.

Somente o áudio Opus é convertido para AAC.

Isso evita uma segunda geração de compressão no vídeo e mantém uma boa qualidade de áudio.

---

# Sincronização

O objetivo principal do processo é evitar problemas de sincronização entre os streams de vídeo e áudio.

Por isso, o script utiliza a seguinte estratégia:

```text
1. Baixar vídeo completo
2. Baixar áudio completo
3. Juntar os streams
4. Cortar localmente
5. Converter somente quando necessário
```

Esse método é mais confiável do que depender exclusivamente do download direto de pequenos segmentos dos streams.

---

# Arquivos temporários

Durante a execução é criada automaticamente uma pasta temporária dentro de `%TEMP%`.

Exemplo:

```text
C:\Users\Usuario\AppData\Local\Temp\yt-dlp-video-123456789
```

Normalmente ela contém:

```text
source.mkv
trecho.mkv
```

Após a conclusão, essa pasta é removida automaticamente. O arquivo final fica na pasta de destino selecionada.

---

# Nome dos arquivos

O nome do arquivo final é baseado automaticamente no título do vídeo.

### Trecho

Exemplo:

```text
SAMU LINO DÁ SHOW E MENGÃO VENCE O CLÁSSICO COM GOLAÇOS FLAMENGO 3X0 BOTAFOGO - trecho 00-03-01-00-04-50.mp4
```

### Vídeo completo

Exemplo:

```text
SAMU LINO DÁ SHOW E MENGÃO VENCE O CLÁSSICO COM GOLAÇOS FLAMENGO 3X0 BOTAFOGO - completo.mp4
```

Caracteres que não podem ser utilizados em nomes de arquivos do Windows são automaticamente substituídos.

O processamento do nome é realizado usando o PowerShell para evitar problemas com caracteres acentuados e caracteres especiais.

---

# Logs

O script cria automaticamente:

```text
download.log
```

na pasta de destino selecionada.

O log registra:

* Data e hora da execução
* URL utilizada
* Título do vídeo
* Formato escolhido
* Modo de download
* Horários do trecho
* Pasta temporária
* IDs dos formatos selecionados e saída detalhada do `yt-dlp`
* Saída do FFmpeg
* Erros durante o processo

Após um download bem-sucedido, o script pergunta se deve abrir a pasta de destino.

O arquivo é especialmente útil para diagnosticar falhas.

Em caso de problema, o conteúdo de `download.log` pode ser utilizado para identificar em qual etapa ocorreu o erro.

---

# Fluxo de saída

## MKV + vídeo completo

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ arquivo final MKV
```

## MKV + trecho

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ corte FFmpeg
→ trecho.mkv
```

## MP4 + vídeo completo

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ H.264 permanece intacto
→ Opus convertido para AAC 320 kbps
→ arquivo MP4
```

## MP4 + trecho

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ corte FFmpeg
→ H.264 permanece intacto
→ Opus convertido para AAC 320 kbps
→ arquivo MP4
```

---

# Qual formato escolher?

### Use MKV quando:

* qualidade original for prioridade;
* você quiser manter Opus;
* o player utilizado aceitar MKV;
* não precisar de compatibilidade máxima.

### Use MP4 quando:

* compatibilidade for prioridade;
* o arquivo será utilizado em celulares, TVs, navegadores ou softwares que preferem MP4;
* você quiser H.264 + AAC;
* não houver necessidade de preservar o áudio Opus original.

---

# Solução de problemas

## O script não inicia

Verifique:

```text
yt-dlp --version
```

e:

```text
ffmpeg -version
```

Caso algum comando não seja reconhecido, o programa correspondente não está disponível no `PATH`.

---

## O FFmpeg encontrado é inesperado

Verifique:

```text
where ffmpeg
```

O Windows pode encontrar mais de uma instalação.

Exemplo:

```text
C:\Program Files\...\ffmpeg.exe
```

Nesse caso, o primeiro resultado normalmente é o executável utilizado pelo script.

Também é possível verificar:

```text
ffmpeg -version
```

---

## O download falha

Consulte:

```text
download.log
```

O log contém a saída completa do `yt-dlp` e normalmente permite identificar se o problema está relacionado à conexão, ao YouTube, ao formato selecionado ou a outro componente.

---

## O vídeo possui caracteres estranhos no nome

O script utiliza o PowerShell para tratar os caracteres inválidos do Windows.

Além disso, o script inicia com:

```text
chcp 65001
```

para trabalhar com UTF-8 e reduzir problemas com caracteres acentuados.

---

# Observações técnicas

O script prioriza:

```text
1. Confiabilidade do download
2. Sincronização de áudio e vídeo
3. Preservação da qualidade
4. Compatibilidade do arquivo
```

O vídeo selecionado é copiado sem recodificação quando o contêiner escolhido é compatível.

Quando MP4 é escolhido, somente o áudio é recodificado para AAC 320 kbps.

Quando MKV é escolhido, vídeo e áudio permanecem nos codecs originais sempre que estes estiverem disponíveis.

---

# Licença e uso

Este projeto é apenas uma ferramenta de automação utilizando `yt-dlp` e `FFmpeg`.

O usuário é responsável por utilizar a ferramenta de acordo com:

* direitos autorais aplicáveis;
* permissões do conteúdo;
* legislação local;
* termos de serviço das plataformas utilizadas.

Use o programa somente para conteúdos que você tenha autorização para baixar.

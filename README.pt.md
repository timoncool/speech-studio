<div align="center">

<img src="frontend/public/favicon.svg" width="72" alt="Logo do Speech Studio"/>

# Speech Studio

**Estúdio de voz com IA gratuito e offline para Windows, macOS (Apple silicon) e Linux — dá voz a qualquer texto com vozes clonadas e embutidas e transforma gravações em texto com quem fala. O russo feito direito: números no caso certo, acentos, «ё» e homógrafos. 100% local, zero Python: um aplicativo nativo (Rust + C++ sobre CUDA, Vulkan ou Metal); cada modelo e cada motor baixam com um botão.**

[![License](https://img.shields.io/github/license/timoncool/speech-studio?style=flat-square)](LICENSE)
[![Stars](https://img.shields.io/github/stars/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/stargazers)
[![Latest release](https://img.shields.io/github/v/release/timoncool/speech-studio?include_prereleases&style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Downloads](https://img.shields.io/github/downloads/timoncool/speech-studio/total?style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Last commit](https://img.shields.io/github/last-commit/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/commits)

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh.md) · [Español](README.es.md) · **Português** · [Français](README.fr.md)

![Speech Studio — um diálogo com uma voz para cada personagem](docs/screenshots/pt-01-speech.webp)

</div>

## O que é

O **Speech Studio** tem três modos. **Speech** dá voz a um texto: uma frase, um diálogo com uma voz por personagem, um podcast que o diretor escreve para você ou um livro inteiro a partir de EPUB, FB2, PDF ou DOCX. **Listening** transforma uma gravação, um vídeo ou um link em texto com quem disse o quê e quando, edita pelas palavras, confere uma locução com o texto que ela deveria dizer e cria vozes da biblioteca a partir de quem fala. **Live** é uma conversa por voz com um agente de IA conectado por MCP: ele diz cada frase assim que a escreve e ouve você pelo microfone.

Tudo roda **na sua máquina**: sem nuvem e sem assinatura; seus textos, gravações e impressões de voz não saem do computador. Se quiser uma voz que sua placa não roda, há também as vozes na nuvem do **OpenRouter**, **Google Gemini**, **fal.ai** e **Runware**, cada uma com sua chave e uma confirmação antes da primeira chamada; vêm desligadas.

É um **aplicativo totalmente nativo**: sem Python embutido, sem torch, sem wheels de CUDA. O estúdio inteiro é **Rust + motores nativos C++ (GGUF/ONNX) sobre CUDA, Vulkan ou Metal**. Modelos, motores, as bibliotecas CUDA e VC++ e o ffmpeg **o próprio aplicativo baixa** na primeira execução, e um modelo que outro estúdio do autor já tem é aproveitado em vez de baixado de novo. **O aplicativo é feito e testado para placas NVIDIA**; para o resto, veja *O que roda onde*.

## Para agentes de IA

Enquanto o Speech Studio está aberto, ele serve MCP em `http://127.0.0.1:8797/mcp`: um agente como Claude Code, Claude Desktop, Cursor ou Codex faz tudo o que a janela faz, pelo mesmo código: dá voz a um texto em um arquivo com uma só chamada, cria e edita projetos bloco a bloco, distribui vozes entre personagens, cria uma voz a partir de uma descrição, transcreve e confere gravações e exporta audiolivros com capítulos e legendas. Em Configurações, **Agente (MCP)** mostra se há um agente conectado e o que colar no cliente. A única coisa que um agente não pode fazer é ligar o microfone: o Live, uma gravação e o teste do microfone só começam com o seu próprio clique ou tecla.

1. Instale o estúdio a partir da [versão mais recente](https://github.com/timoncool/speech-studio/releases/latest) e abra-o.
2. Conecte-se ao servidor MCP:
   ```bash
   claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp
   ```
   Outros clientes: `{ "mcpServers": { "speech-studio": { "type": "streamable-http", "url": "http://127.0.0.1:8797/mcp" } } }`
3. Leia a habilidade que o servidor oferece (recurso `studio://skill`), o mesmo texto de [docs/mcp-skill.md](docs/mcp-skill.md), e comece pela ferramenta `studio_status`.

Na mesma porta o estúdio fala a API de áudio da OpenAI — `POST /v1/audio/speech`, `POST /v1/audio/transcriptions`, `GET /v1/models` —, então ferramentas feitas para a OpenAI locutam e transcrevem por ele, com o nome de uma voz da biblioteca como `voice`. [llms.txt](llms.txt) diz o mesmo para ferramentas que o procuram. Só programas deste computador podem se conectar.

## O que dá para fazer com ele

- **Um audiolivro** — solte um EPUB ou FB2: capítulos, personagens com suas vozes, um M4B com capítulos e capa, volume ACX conferido.
- **Um podcast ou um diálogo** — o diretor escreve um podcast sobre um tema para duas ou três vozes, ou cole linhas como «Ana: …»; a linha do tempo mostra cada sobreposição.
- **Uma locução para um vídeo** — dê voz a um roteiro ou a legendas dentro dos seus tempos, coloque música e sons no editor de faixas e exporte no volume do vídeo.
- **A transcrição de uma entrevista ou aula** — quem disse o quê e quando, tempo de cada palavra, legendas pelas regras das emissoras, a gravação editada pelo texto.
- **A revisão de uma dublagem** — sobreponha o roteiro à gravação e veja cada palavra dita errada ou pulada.
- **Sua própria biblioteca de vozes** — grave ou importe vozes, limpe-as e crie vozes a partir de quem fala numa gravação.
- **Uma voz para o seu agente de IA** — converse com o Claude Code ou qualquer agente MCP no Live, ou deixe-o locutar e transcrever arquivos por MCP ou pela API compatível com OpenAI.

Exemplos de todos os motores e vozes, com a acentuação russa antes e depois, estão na [página do projeto](https://timoncool.github.io/speech-studio/pt.html).

## Três modos

| Modo | O que faz |
|------|--------------|
| 🔊 **Speech** | Texto, diálogo, podcast ou livro → blocos locutados com tomadas, revisados de ouvido e exportados em WAV, FLAC, MP3, OGG, M4A ou M4B com capítulos, legendas e texto sincronizado |
| 👂 **Listening** | Áudio, vídeo ou link → uma transcrição com falantes e tempos por palavra, um player karaokê, edição pelo texto, exportação para SRT, VTT, ASS, TXT ou JSON e conferência com um texto de referência |
| 🎙️ **Live** | Uma conversa por voz com um agente de IA: o agente conectado por MCP diz cada frase assim que a escreve, você responde ao microfone e é reconhecido frase a frase, uma frase dita por cima o interrompe; só você liga o microfone, pausa com M ou Ctrl+Shift+Espaço, um widget sobre todas as janelas |

## Recursos

**Vozes e motores**

- **Seis motores locais** — [Higgs Audio v3](https://huggingface.co/bosonai) (clonagem, emoções e sons), **Fish Audio Drama 3** (a voz aberta mais forte; inteira numa placa de 24 GB, a partir de 6 GB com os especialistas na RAM), **Audio8 TTS** (fala natural em 23 idiomas, clonagem ou a voz própria do modelo), **Supertonic 3** (rápido, dez vozes embutidas), **Qwen3-TTS 1.7B** (nove vozes embutidas, clonagem, uma instrução por bloco) e **Qwen3-TTS com acento russo**, a LoRA de siriusfreak aplicada na hora.
- **Motor por personagem e por bloco** sobre o do projeto; um motor que não fala com a voz escolhida diz qual voz própria vai usar.
- **Biblioteca de vozes** — um pacote de vozes com um clique, 743 vozes russas do catálogo na nuvem, gravação pelo microfone, importação de arquivos e ZIP, foto, gênero, idade, idioma, tags e consentimento; a transcrição da referência é reconhecida uma vez e guardada.
- **Limpeza da referência** — Mel-Band Roformer separa a voz da música e o **DPDFNet** remove o ruído com controle de intensidade, para a fala continuar viva; o original é guardado.
- **O melhor trecho da referência** — janelas nas pausas entre palavras reconhecidas, cada uma com duração, relação sinal/ruído, clipping e a pausa mais longa.
- **Voz por descrição** — o Qwen3-TTS VoiceDesign cria uma voz a partir de «uma senhora idosa de voz suave e um pouco rouca…»: ouça vários candidatos e guarde um na biblioteca.

- **Vozes na nuvem** — OpenRouter, Google Gemini (pedidos em paralelo ou Batch pela metade do preço), fal.ai e Runware, cada uma com sua chave e uma confirmação antes da primeira chamada.

**Texto que soa certo**

- **Normalizador do russo** — números no caso e gênero certos, datas, horas, dinheiro, unidades, números romanos, marcas latinas e siglas.
- **Acentos, «ё» e homógrafos** pelo RUAccent — o modelo grande de homógrafos (big_poetry em int8, as mesmas escolhas do fp32) decide pela frase; os duvidosos ficam destacados para você e seus `+` são mantidos.
- **Dicionário de pronúncia** — substituição, regex e fonemas, no escopo do projeto, idioma, voz ou motor, importação de Balabolka `.rex`/`.dic`/`.bxd` e teste de ouvido direto do dicionário.
- **Marcação compatível com o SpeechKit** — acento `+`, pausas `sil<[300]>` e `[pause 500ms]`, ritmo `[rate:1.2]`, `**ênfase**`; o que um motor ignora é marcado, nunca descartado em silêncio.

- **Ainda mais texto lido certo** — um subconjunto de SSML, dicas de prosódia em cor palavra por palavra, telefones, e-mails e códigos, um trecho em outro idioma com a mesma voz e uma revisão do roteiro antes da locução.

**Projetos**

- **O texto como linha do tempo** — blocos com falantes, tomadas (até cinco), controle de desatualizados, desfazer e refazer, arrastar para reordenar, buscar e substituir com regex, notas e marcadores.
- **Controle de qualidade** — cada tomada é reconhecida de volta e comparada palavra a palavra, além de checar clipping, silêncio, corte, estalo no fim, frase repetida, cauda longa, música ou ruído no lugar da fala e semelhança com a voz de referência; várias tomadas com a melhor escolhida.
- **Diálogos e podcasts** — linhas `Ana:` e `[Ana]` viram personagens, o diretor escreve um podcast sobre um tema para N falantes, continua um roteiro dentro de um projeto aberto ou reescreve as falas selecionadas, pausas negativas criam interrupções e a **linha do tempo** mostra cada sobreposição com cortar, juntar ou manter.
- **Audiolivros** — EPUB, FB2, PDF, DOCX, HTML, Markdown e TXT com prévia dos capítulos, elenco pelo diretor, M4B com capítulos e capa, partes para livros longos e checagem de volume ACX.
- **Legendas como projeto** — falas de SRT, VTT e ASS locutadas no seu tempo: o ritmo sobe antes, uma tomada que não cabe é refeita e depois vêm a aceleração e o empréstimo do silêncio anterior.
- **Checagem antes de exportar** — blocos sem áudio, desatualizados, com falha, com o arquivo alterado ou mais longos que a fala são listados antes da exportação; só a duração pode ser liberada.
- **Volume** — predefinições de vídeo (−14 LUFS), podcast (−16 LUFS) e ACX com limitador true-peak, room tone e respirações entre blocos.
- **Arquivo de projeto** — `.speechproj` com texto, vozes e áudio leva um projeto para outro computador.

- **Montagem** — um editor de faixas sob o texto: mover, aparar, fades e volume de cada clipe, faixas de música e sons de arquivos ou pastas, ou uma música feita ali mesmo no YuE2, MiniMax Music3 ou ACE-Step Studio.
- **Exportação para cada uso** — um arquivo por capítulo, uma faixa por personagem em um ZIP, uma tabela CSV de falas, efeitos de voz (rádio, telefone, alto-falante), volume uniforme dos trechos, taxa de amostragem e profundidade de bits, modelos de nomes de arquivo, um manifesto SHA-256 e uma nota de como foi feito.
- **Comparar e aprovar** — A/B de tomadas, uma comparação às cegas de motores no mesmo texto, blocos aprovados que uma nova locução não toca, um semáforo de qualidade e um relatório do projeto.
- **Livros e peças** — a fala direta separada da do narrador, um elenco de personagens com vozes escolhidas para eles, notas de rodapé limpas, os nomes do livro no dicionário, um projeto atualizado a partir de um livro corrigido sem perder o que já foi locutado e um player HTML do livro.

**Listening**

- **Parakeet-TDT v3, Nemotron 3.5 (28 idiomas com pontuação) e CrisperWhisper 2.0 turbo (99 idiomas com o tempo das palavras) no audio.cpp, Parakeet Ultra no transcribe.cpp**, até oito falantes com Nemotron 3 Diarization, uma gravação longa cortada nas pausas pelo **Silero VAD v6**, ruído removido pelo DPDFNet e música pelo Roformer.
- **Uma dica para o CrisperWhisper** — tema e grafias, mais as correções que você adicionar.
- **Lote** — solte vários arquivos de uma vez; cada um vira uma gravação na fila de reconhecimento.
- **Um texto conhecido sobre a gravação** — o texto de referência recebe o tempo de cada palavra ouvida, frase por frase.
- **Vozes a partir dos falantes** — uma voz da biblioteca com as palavras mais limpas de cada falante.

- **Enquanto você fala** — reconhecimento pelo microfone frase a frase, uma pasta vigiada reconhecida em segundo plano, reconhecimento rápido sem o editor.
- **O modelo certo sozinho** — com o idioma em Auto, o VoxLingua107 o identifica e a gravação vai para um reconhecedor que o conhece; o modelo escolhido é baixado na primeira vez que é usado.
- **Legendas pelas regras das emissoras** (Netflix, BBC e outras), quem fala quanto, números em algarismos na exportação.

**Live**

- **Uma conversa por voz** — o agente conectado por MCP diz cada frase assim que a escreve; você responde ao microfone e é reconhecido frase a frase; uma frase de duas palavras ou mais dita por cima do agente o interrompe, enquanto uma tosse, uma risada ou um «aham» não; uma linha digitada chega a ele como se fosse dita.
- **Mudanças na hora** — a voz do agente, o reconhecedor, o idioma e o microfone mudam no meio da conversa, por você ou pelo agente a seu pedido: o reconhecedor troca na hora, um microfone ou idioma novo reinicia o microfone e, enquanto o motor da nova voz carrega, a janela mostra «Carregando a voz…».
- **Um ritmo natural** — uma frase do agente é dita inteira até 300 caracteres, uma pausa que o motor deixa dentro de uma frase dura no máximo 0,6 s e cada frase toca uma só vez.
- **Só você liga o microfone** — o Live, uma gravação no Listening, uma gravação de voz e o teste do microfone só começam com o seu próprio clique ou tecla; um agente não tem nenhuma ferramenta que ligue o microfone, e a janela não aceita os cliques dele nesses botões. A barra superior mostra «Microfone ligado», para quê e há quanto tempo, com um X que o desliga; sem nenhuma janela do estúdio aberta por cinco segundos, o microfone desliga sozinho.
- **Microfone e som** — pausa com M ou Ctrl+Shift+Espaço de qualquer janela (o agente se cala e não ouve você; segurar para falar), o X encerra a conversa e o que o agente está dizendo, medidores de nível, ouvir a si mesmo, teste do alto-falante e volume do agente.
- **Papéis: um treinador para qualquer conversa** — escolha quem o agente interpreta: um entrevistador, um investidor, um cliente que quer desconto, um cético, um chefe, um colega difícil, a plateia, um parceiro de idioma, um oponente de debate. Um papel é uma ideia pronta, não uma moldura: o agente pode mudá-lo, e vocês dois inventam os seus conversando — uma conversa difícil, um exercício de dicção ou de idioma, um improviso, uma cena de teatro — e guardam como arquivo na pasta `roles`. Depois da conversa, a análise do agente traz o seu ritmo, pausas, vícios de linguagem e a rapidez das respostas, três frases suas, três coisas a mudar e uma meta, em comparação com a última vez no mesmo papel.
- **Cinco visuais e um widget** — aura, onda, barras, radial e matriz, e um pequeno widget sobre todas as janelas; Ctrl+3 abre o Live.
- **Medido** — sua frase vira texto 0,64–0,74 s depois que você para de falar (Parakeet numa RTX 4090), e o primeiro som da resposta chega de 15 ms a 1 s depois que o agente a escreve, conforme o motor.

**Em todo lugar**

- **API compatível com a OpenAI**, **MCP** com controle da janela para agentes e um painel de assistente com um modelo do OpenRouter e as mesmas ferramentas.
- **Diagnóstico** — placa de vídeo, driver, CUDA, bibliotecas, logs e um pacote para relatar erros sem chaves.
- **O aplicativo inteiro em seis idiomas**, tema claro e escuro, atualização automática e totalmente portátil.
- **A primeira execução pergunta só o que você quer** — vozes, reconhecimento e extras; motores, bibliotecas e a versão para a sua placa vêm junto, uma linha cada com seu tamanho.
- **Paleta de comandos** (Ctrl+K), atalhos próprios, um modo simples e uma dica «?» em cada controle.

## Velocidade medida

Num mesmo computador: RTX 4090 (24 GB) no Windows 11, Speech Studio como é instalado.

**Locução** — Speech Studio 0.2.0, o mesmo texto russo de 17 segundos, a segunda execução de cada motor (modelo carregado); Higgs — no motor audio.cpp em 22.10, três execuções, primeiro som não medido; Audio8 — em 09.11, um texto russo de 18 segundos com voz clonada: ele entrega a frase inteira, então o primeiro som vem em 2,3 s nesse texto e em 0,4–1,0 s numa frase dita no Live; o primeiro som é o tempo até o áudio começar na janela, a memória de vídeo o pico enquanto fala.

| Motor | Mais rápido que o tempo real | Primeiro som | Streaming | Download | Memória de vídeo |
|---|---|---|---|---|---|
| Supertonic 3 | 168× | 103 ms | por frase | 0,1 GB | 1,0 GB |
| Qwen3-TTS | 12× | 19 ms | enquanto fala | 2,2 GB | 3,1 GB |
| Qwen3-TTS + ru stress LoRA | 10× | 26 ms | enquanto fala | 2,3 GB | 3,5 GB |
| Fish Audio Drama 3 | 7,0× | 326 ms | enquanto fala | 21,4 GB | 21,9 GB |
| Audio8 TTS | 5,8× | 2,3 s | por frase | 1,5 GB | 2,0 GB |
| Higgs Audio v3 | 3,4× | — | enquanto fala | 5,3 GB | ~7 GB |

**Reconhecimento** — Speech Studio 0.2.1 (CrisperWhisper — motor audio.cpp em 22.10: uma gravação russa de 625,8 s, com o carregamento do motor; no processador e a memória de vídeo não medidos), uma gravação russa de 5 minutos de uma só voz (299,7 s) no Listening sem separar falantes, com o carregamento do modelo incluído; o modelo libera a memória de vídeo ao terminar.

| Modelo | Na placa | No processador | Idiomas | Download | Memória de vídeo |
|---|---|---|---|---|---|
| Parakeet TDT v3 | 96× (3,1 s) | 33× (9,2 s) | 25 | 0,7 GB | 1,2 GB |
| Parakeet Ultra | 73× (4,1 s) | 33× (9,2 s) | 25 | 0,7 GB | 1,0 GB |
| Nemotron 3.5 | 73× (4,1 s) | 29× (10,3 s) | 28 | 0,7 GB | 1,0 GB |
| CrisperWhisper 2.0 turbo | 96× (6,5 s) | — | 99 | 1,6 GB | — |

**Live** — uma vez da conversa.

| Etapa | Tempo |
|---|---|
| Sua frase em texto depois que você para de falar (Parakeet na placa, incluída a pausa que encerra a frase) | 0,64–0,74 s |
| O agente escreve a resposta (Claude Sonnet) | 2,0–3,9 s |
| Primeiro som da resposta (por motor, veja Locução) | 15 ms – 1,0 s |

## Capturas de tela

A tela inicial — um texto, um livro, legendas ou um roteiro de podcast viram projeto:

![Tela inicial do Speech Studio](docs/screenshots/pt-08-home.webp)

Listening — transcrição com falantes e tempos por palavra, player karaokê, edição pelo texto e exportação:

![Listening no Speech Studio](docs/screenshots/pt-06-listening.webp)

Live — uma conversa por voz com um agente de IA: o que você disse e o que ele responde, cinco visuais, pausa e fim da conversa:

![Live no Speech Studio](docs/screenshots/pt-11-live.webp)

## Requisitos

- **SO:** Windows 10 / 11 (x64); macOS 13 ou mais novo em Apple silicon e Linux x86-64 como builds experimentais
- **GPU:** NVIDIA com 8 GB de VRAM ou mais. Placas a partir da Turing (GTX 16, RTX 20 e mais novas) com driver 580 ou mais novo rodam os builds CUDA 13; GTX 900, GTX 10, Volta e drivers 525–579 rodam os builds CUDA 12 do Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarização, Qwen3-TTS, separação e diretor, e o Parakeet Ultra pelo Vulkan. O Drama 3 precisa de CUDA 13: cabe inteiro em 24 GB; a partir de 6 GB os especialistas (18 GB) ficam na RAM, que então precisa de 32 GB, e a VRAM que ele usa é definida nas Configurações (Auto ou de 5 a 24 GB). Uma placa AMD ou Intel roda os builds Vulkan, um computador sem placa os de CPU, um Mac o Metal (sem Drama 3); essas configurações não são testadas pelo autor
- **WebView2** — já vem no Windows 11; no Windows 10 o instalador baixa
- **Disco:** cerca de 9 GB para os modelos, motores e bibliotecas padrão, mais o que você adicionar (o Drama 3 tem 23 GB)

Numa máquina NVIDIA, a única coisa instalada à mão é um **[driver NVIDIA](https://www.nvidia.com/Download/index.aspx)** recente.

## Início rápido

1. **Baixe** da [versão mais recente](https://github.com/timoncool/speech-studio/releases/latest): para Windows a portátil `Speech-Studio-<versão>-portable-windows-x64.zip` (descompacte onde quiser) ou o instalador `Speech.Studio_<versão>_x64-setup.exe`; para macOS `Speech-Studio-<versão>-macos-arm64.dmg`; para Linux `Speech-Studio-<versão>-linux-x86_64-experimental.AppImage` ou `Speech-Studio-<versão>-linux-amd64-experimental.deb`.
2. **Execute** `Speech Studio.exe` (Windows), o Speech Studio em Aplicativos (macOS) ou o AppImage (Linux).
3. No painel **Primeira execução** clique em **Baixar os marcados** — o aplicativo baixa uma vez os modelos, motores e bibliotecas.
4. **Cole um texto** e clique em **Criar**, ou mude para **Listening** e solte uma gravação.

> No Windows tudo é baixado e fica **dentro da pasta do aplicativo**: modelos, caches, vozes e projetos não vão para nenhum outro lugar. No Linux fica em `~/.local/share/speech-studio`, no macOS em `~/Library/Application Support/Speech Studio`.

O que mudou e quando está em [CHANGELOG.md](CHANGELOG.md), e o botão de brilhos no topo do aplicativo mostra isso. Regras para colaboradores e agentes de código: [AGENTS.md](AGENTS.md).

## Tudo o que o aplicativo baixa

O painel da primeira execução baixa tudo isso com um botão. Atrás de um proxy, ou onde o Hugging Face está bloqueado, configure o proxy em Configurações → **Rede**. Você também pode baixar os arquivos diretos, colocá-los onde diz a última coluna (a partir da pasta do aplicativo) e clicar em **Importar da pasta**; os componentes em arquivo compactado o aplicativo baixa sozinho. Cada arquivo é conferido com seu SHA-256 fixado antes do uso. Tamanhos e hashes vêm do manifesto do aplicativo, `crates/speech-server/src/setup.rs`, e das versões fixadas dos crates compartilhados que ele nomeia, e esta tabela é conferida com eles. A tabela lista os arquivos do Windows: uma placa NVIDIA em que roda o CUDA 13 pega os builds CUDA 13, uma placa NVIDIA ou um driver mais antigos os builds CUDA 12.9, uma placa AMD ou Intel os builds Vulkan, um computador sem placa os builds de CPU; uma etapa com outro dispositivo escolhido em Configurações → Modelos pega o build desse dispositivo. No Linux e no macOS o aplicativo baixa os builds dos mesmos componentes para esse sistema.

<!-- downloads:start -->
| Componente | Necessidade | Arquivos (links diretos) | Tamanho | Onde colocar |
|---|---|---|---|---|
| Higgs Audio v3 Q8_0 | obrigatório | [q8_0.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/q8_0.gguf) → `models\higgs-q8_0\q8_0.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/config.json) → `models\higgs-q8_0\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/chat_template.jinja) → `models\higgs-q8_0\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer.json) → `models\higgs-q8_0\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer_config.json) → `models\higgs-q8_0\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q8_0\higgs_audio_v2_tokenizer_config.json` | 5.5 GB | como está, no caminho após a seta |
| Audio8 TTS Realistic-Air Q8_0 | opcional | [Audio8-TTS-Realistic-Air-Q8_0.gguf](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF/resolve/10f8bcf54be8b20b53abaaa1a074a62b34089ab2/Audio8-TTS-Realistic-Air-Q8_0.gguf) → `models\audio8\Audio8-TTS-Realistic-Air-Q8_0.gguf` | 1.5 GB | como está, no caminho após a seta |
| audio.cpp server, CUDA 13 (Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarization) | obrigatório | [audiocpp-server-win-cuda13-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda13-x64.zip) | 136 MB | descompactar os arquivos, sem subpastas, em `tools\audiocpp\` |
| Gemma-4 12B QAT q4_0 + vision | opcional | [gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\gemma-4-12b-it-qat-q4_0.gguf`<br>[mmproj-gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/mmproj-gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\mmproj-gemma-4-12b-it-qat-q4_0.gguf` | 7.2 GB | como está, no caminho após a seta |
| Gemma-4 12B Q5_K_M + vision | opcional | [gemma-4-12b-it-Q5_K_M.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q5_K_M.gguf) → `models\mt-q5_0\gemma-4-12b-it-Q5_K_M.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q5_0\mmproj-F16.gguf` | 8.6 GB | como está, no caminho após a seta |
| Gemma-4 12B Q6_K + vision | opcional | [gemma-4-12b-it-Q6_K.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q6_K.gguf) → `models\mt-q6_k\gemma-4-12b-it-Q6_K.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q6_k\mmproj-F16.gguf` | 10.0 GB | como está, no caminho após a seta |
| Gemma-4 12B Q8_0 + vision | opcional | [gemma-4-12b-it-Q8_0.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q8_0.gguf) → `models\mt-q8_0\gemma-4-12b-it-Q8_0.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q8_0\mmproj-F16.gguf` | 12.8 GB | como está, no caminho após a seta |
| Parakeet-TDT 0.6B v3 Q8_0 | obrigatório | [parakeet-tdt-0.6b-v3-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/resolve/d8d8c32cb2017c57e5307767343e97870f8a7f53/parakeet-tdt-0.6b-v3-Q8_0.gguf) → `models\asr-gguf-parakeet-v3\parakeet-tdt-0.6b-v3-Q8_0.gguf` | 740 MB | como está, no caminho após a seta |
| Higgs Audio v3 Q6_K | opcional | [q6_k.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/q6_k.gguf) → `models\higgs-q6_k\q6_k.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/config.json) → `models\higgs-q6_k\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/chat_template.jinja) → `models\higgs-q6_k\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer.json) → `models\higgs-q6_k\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer_config.json) → `models\higgs-q6_k\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q6_k\higgs_audio_v2_tokenizer_config.json` | 5.0 GB | como está, no caminho após a seta |
| Higgs Audio v3 Q4_K_M | opcional | [q4_k_m.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/q4_k_m.gguf) → `models\higgs-q4_k_m\q4_k_m.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/config.json) → `models\higgs-q4_k_m\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/chat_template.jinja) → `models\higgs-q4_k_m\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer.json) → `models\higgs-q4_k_m\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer_config.json) → `models\higgs-q4_k_m\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q4_k_m\higgs_audio_v2_tokenizer_config.json` | 4.1 GB | como está, no caminho após a seta |
| Fish Audio Drama 3 Q4 + drama_engine (6 GB+ GPU, 32 GB RAM under 24 GB) | opcional | [.gitattributes](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/.gitattributes) → `models\drama3\.gitattributes`<br>[LICENSE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/LICENSE) → `models\drama3\LICENSE`<br>[NOTICE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/NOTICE) → `models\drama3\NOTICE`<br>[README.md](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/README.md) → `models\drama3\README.md`<br>[codec.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/codec.safetensors) → `models\drama3\codec.safetensors`<br>[drama.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/drama.json) → `models\drama3\drama.json`<br>[globals.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/globals.safetensors) → `models\drama3\globals.safetensors`<br>[layer00.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer00.safetensors) → `models\drama3\layer00.safetensors`<br>[layer01.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer01.safetensors) → `models\drama3\layer01.safetensors`<br>[layer02.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer02.safetensors) → `models\drama3\layer02.safetensors`<br>[layer03.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer03.safetensors) → `models\drama3\layer03.safetensors`<br>[layer04.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer04.safetensors) → `models\drama3\layer04.safetensors`<br>[layer05.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer05.safetensors) → `models\drama3\layer05.safetensors`<br>[layer06.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer06.safetensors) → `models\drama3\layer06.safetensors`<br>[layer07.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer07.safetensors) → `models\drama3\layer07.safetensors`<br>[layer08.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer08.safetensors) → `models\drama3\layer08.safetensors`<br>[layer09.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer09.safetensors) → `models\drama3\layer09.safetensors`<br>[layer10.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer10.safetensors) → `models\drama3\layer10.safetensors`<br>[layer11.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer11.safetensors) → `models\drama3\layer11.safetensors`<br>[layer12.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer12.safetensors) → `models\drama3\layer12.safetensors`<br>[layer13.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer13.safetensors) → `models\drama3\layer13.safetensors`<br>[layer14.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer14.safetensors) → `models\drama3\layer14.safetensors`<br>[layer15.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer15.safetensors) → `models\drama3\layer15.safetensors`<br>[layer16.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer16.safetensors) → `models\drama3\layer16.safetensors`<br>[layer17.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer17.safetensors) → `models\drama3\layer17.safetensors`<br>[layer18.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer18.safetensors) → `models\drama3\layer18.safetensors`<br>[layer19.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer19.safetensors) → `models\drama3\layer19.safetensors`<br>[layer20.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer20.safetensors) → `models\drama3\layer20.safetensors`<br>[layer21.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer21.safetensors) → `models\drama3\layer21.safetensors`<br>[layer22.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer22.safetensors) → `models\drama3\layer22.safetensors`<br>[layer23.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer23.safetensors) → `models\drama3\layer23.safetensors`<br>[layer24.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer24.safetensors) → `models\drama3\layer24.safetensors`<br>[layer25.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer25.safetensors) → `models\drama3\layer25.safetensors`<br>[layer26.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer26.safetensors) → `models\drama3\layer26.safetensors`<br>[layer27.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer27.safetensors) → `models\drama3\layer27.safetensors`<br>[layer28.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer28.safetensors) → `models\drama3\layer28.safetensors`<br>[layer29.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer29.safetensors) → `models\drama3\layer29.safetensors`<br>[layer30.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer30.safetensors) → `models\drama3\layer30.safetensors`<br>[layer31.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer31.safetensors) → `models\drama3\layer31.safetensors`<br>[layer32.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer32.safetensors) → `models\drama3\layer32.safetensors`<br>[layer33.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer33.safetensors) → `models\drama3\layer33.safetensors`<br>[layer34.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer34.safetensors) → `models\drama3\layer34.safetensors`<br>[layer35.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer35.safetensors) → `models\drama3\layer35.safetensors`<br>[layer36.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer36.safetensors) → `models\drama3\layer36.safetensors`<br>[layer37.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer37.safetensors) → `models\drama3\layer37.safetensors`<br>[layer38.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer38.safetensors) → `models\drama3\layer38.safetensors`<br>[layer39.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer39.safetensors) → `models\drama3\layer39.safetensors`<br>[tokenizer.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tokenizer.json) → `models\drama3\tokenizer.json`<br>[tower.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tower.safetensors) → `models\drama3\tower.safetensors`<br>[drama_engine.dll](https://github.com/timoncool/drama-rs/releases/download/engine-0.1.3/drama_engine.dll) → `models\drama-engine\drama_engine.dll` | 23.0 GB | como está, no caminho após a seta |
| Supertonic 3 int8 | opcional | [LICENSE](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/LICENSE) → `models\supertonic3\LICENSE`<br>[duration_predictor.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/duration_predictor.onnx) → `models\supertonic3\onnx\duration_predictor.onnx`<br>[text_encoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/text_encoder.onnx) → `models\supertonic3\onnx\text_encoder.onnx`<br>[vector_estimator.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vector_estimator.onnx) → `models\supertonic3\onnx\vector_estimator.onnx`<br>[vocoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vocoder.onnx) → `models\supertonic3\onnx\vocoder.onnx`<br>[tts.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/tts.json) → `models\supertonic3\onnx\tts.json`<br>[unicode_indexer.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/unicode_indexer.json) → `models\supertonic3\onnx\unicode_indexer.json`<br>[F1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F1.json) → `models\supertonic3\voice_styles\F1.json`<br>[F2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F2.json) → `models\supertonic3\voice_styles\F2.json`<br>[F3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F3.json) → `models\supertonic3\voice_styles\F3.json`<br>[F4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F4.json) → `models\supertonic3\voice_styles\F4.json`<br>[F5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F5.json) → `models\supertonic3\voice_styles\F5.json`<br>[M1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M1.json) → `models\supertonic3\voice_styles\M1.json`<br>[M2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M2.json) → `models\supertonic3\voice_styles\M2.json`<br>[M3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M3.json) → `models\supertonic3\voice_styles\M3.json`<br>[M4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M4.json) → `models\supertonic3\voice_styles\M4.json`<br>[M5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M5.json) → `models\supertonic3\voice_styles\M5.json` | 108 MB | como está, no caminho após a seta |
| Qwen3-TTS engine (qwen.dll) and codec | opcional | [qwen-tokenizer-12hz-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-tokenizer-12hz-Q8_0.gguf) → `models\qwen3-tts\qwen-tokenizer-12hz-Q8_0.gguf`<br>[qwen-windows-x64.zip](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/qwen-windows-x64.zip) | 295 MB | como está, no caminho após a seta<br>descompactar os arquivos, sem subpastas, em `models\qwen3-tts\` |
| Qwen3-TTS 1.7B Q4_K_M (CustomVoice + Base) | opcional | [qwen-talker-1.7b-customvoice-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q4_K_M.gguf`<br>[qwen-talker-1.7b-base-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q4_K_M.gguf` | 2.4 GB | como está, no caminho após a seta |
| Qwen3-TTS 1.7B Q8_0 (CustomVoice + Base) | opcional | [qwen-talker-1.7b-customvoice-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q8_0.gguf`<br>[qwen-talker-1.7b-base-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q8_0.gguf` | 4.1 GB | como está, no caminho após a seta |
| Qwen3-TTS VoiceDesign Q4_K_M | opcional | [qwen-talker-1.7b-voicedesign-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q4_K_M.gguf` | 1.2 GB | como está, no caminho após a seta |
| Qwen3-TTS VoiceDesign Q8_0 | opcional | [qwen-talker-1.7b-voicedesign-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q8_0.gguf` | 2.0 GB | como está, no caminho após a seta |
| Qwen3-TTS Russian stress LoRA | opcional | [adapter.safetensors](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf/resolve/c16b3af3db3095120efc1ebb73f6522442bcb3b1/lora/adapter.safetensors) → `models\qwen3-tts\qwen-talker-1.7b-base-ru-stress-lora.safetensors` | 35 MB | como está, no caminho após a seta |
| Parakeet Ultra 0.6B Q8_0 (Moondream, transcribe.cpp) | opcional | [parakeet-ultra-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-ultra-gguf/resolve/a1539be1b849ea7463b88343e5277b373d0c4ea9/parakeet-ultra-Q8_0.gguf) → `models\asr-gguf-parakeet-ultra\parakeet-ultra-Q8_0.gguf` | 740 MB | como está, no caminho após a seta |
| transcribe.cpp engine (CUDA 13) | obrigatório | [transcribe-windows-cuda13-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cuda13-x64.zip) | 131 MB | descompactar os arquivos, sem subpastas, em `models\transcribe-engine\` |
| transcribe.cpp engine (CPU) | opcional | [transcribe-windows-cpu-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cpu-x64.zip) | 5 MB | descompactar os arquivos, sem subpastas, em `models\transcribe-engine-cpu\` |
| transcribe.cpp engine (Vulkan) | opcional | [transcribe-windows-vulkan-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-vulkan-x64.zip) | 19 MB | descompactar os arquivos, sem subpastas, em `models\transcribe-engine-vulkan\` |
| Nemotron 3.5 ASR 0.6B Q8_0 (NVIDIA) | opcional | [nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf](https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/53ce288f69326b415aeef030cfcb02a08a8096dd/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf) → `models\asr-gguf-nemotron\nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf` | 751 MB | como está, no caminho após a seta |
| CrisperWhisper 2.0 turbo (audio.cpp) | opcional | [LICENSE.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/LICENSE.md) → `models\crisperwhisper2-turbo\LICENSE.md`<br>[README.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/README.md) → `models\crisperwhisper2-turbo\README.md`<br>[added_tokens.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/added_tokens.json) → `models\crisperwhisper2-turbo\added_tokens.json`<br>[config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/config.json) → `models\crisperwhisper2-turbo\config.json`<br>[generation_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/generation_config.json) → `models\crisperwhisper2-turbo\generation_config.json`<br>[merges.txt](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/merges.txt) → `models\crisperwhisper2-turbo\merges.txt`<br>[model.safetensors](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/model.safetensors) → `models\crisperwhisper2-turbo\model.safetensors`<br>[normalizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/normalizer.json) → `models\crisperwhisper2-turbo\normalizer.json`<br>[preprocessor_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/preprocessor_config.json) → `models\crisperwhisper2-turbo\preprocessor_config.json`<br>[special_tokens_map.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/special_tokens_map.json) → `models\crisperwhisper2-turbo\special_tokens_map.json`<br>[tokenizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer.json) → `models\crisperwhisper2-turbo\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer_config.json) → `models\crisperwhisper2-turbo\tokenizer_config.json`<br>[vocab.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/vocab.json) → `models\crisperwhisper2-turbo\vocab.json` | 1.6 GB | como está, no caminho após a seta |
| Nemotron 3 Diarization | recomendado | [nemotron3_diar_v3.onnx](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/nemotron3_diar_v3.onnx) → `models\nemotron-diar\nemotron3_diar_v3.onnx`<br>[LICENSE](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/LICENSE) → `models\nemotron-diar\LICENSE` | 401 MB | como está, no caminho após a seta |
| Mel-Band Roformer voc_fv6 Q8_0 | recomendado | [voc_fv6-Q8_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q8_0.gguf) → `models\bsroformer\voc_fv6-Q8_0.gguf` | 252 MB | como está, no caminho após a seta |
| Mel-Band Roformer voc_fv6 Q5_0 | opcional | [voc_fv6-Q5_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q5_0.gguf) → `models\bsroformer\voc_fv6-Q5_0.gguf` | 167 MB | como está, no caminho após a seta |
| Mel-Band Roformer voc_fv6 Q4_0 | opcional | [voc_fv6-Q4_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q4_0.gguf) → `models\bsroformer\voc_fv6-Q4_0.gguf` | 139 MB | como está, no caminho após a seta |
| WeSpeaker ResNet34-LM (voice similarity) | recomendado | [voxceleb_resnet34_LM.onnx](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM/resolve/f0c48c298fd835726c27956a5d617bad7115627e/voxceleb_resnet34_LM.onnx) → `models\faces\wespeaker\voxceleb_resnet34_LM.onnx` | 27 MB | como está, no caminho após a seta |
| Voice gate (ReDimNet2-B3) | opcional | [redimnet2_b3_lm.onnx](https://huggingface.co/diarizeapp/redimnet2-b3-onnx/resolve/96144a5d37ba2900e5ba61e726d7cdd7f0c087a8/redimnet2_b3_lm.onnx) → `models\voice-gate\redimnet2_b3_lm.onnx` | 19 MB | como está, no caminho após a seta |
| Wake word (PhoneticXeus int8) | opcional | [model.int8.onnx](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/model.int8.onnx) → `models\wake-word\model.int8.onnx`<br>[vocab.json](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/vocab.json) → `models\wake-word\vocab.json` | 357 MB | como está, no caminho após a seta |
| RUAccent (stress, «ё», homographs) | recomendado | [model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/model.onnx) → `models\ruaccent\nn\nn_accent\model.onnx`<br>[config.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/config.json) → `models\ruaccent\nn\nn_accent\config.json`<br>[vocab.txt](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/vocab.txt) → `models\ruaccent\nn\nn_accent\vocab.txt`<br>[model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/model.onnx) → `models\ruaccent\nn\nn_omograph\tiny2\model.onnx`<br>[tokenizer.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/tokenizer.json) → `models\ruaccent\nn\nn_omograph\tiny2\tokenizer.json`<br>[accents.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/accents.json.gz) → `models\ruaccent\dictionary\accents.json.gz`<br>[omographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/omographs.json.gz) → `models\ruaccent\dictionary\omographs.json.gz`<br>[yo_words.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_words.json.gz) → `models\ruaccent\dictionary\yo_words.json.gz`<br>[yo_homographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_homographs.json.gz) → `models\ruaccent\dictionary\yo_homographs.json.gz` | 34 MB | como está, no caminho após a seta |
| RUAccent big_poetry homographs int8 | opcional | [model.onnx](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/model.onnx) → `models\ruaccent\nn\nn_omograph\big_poetry\model.onnx`<br>[tokenizer.json](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/tokenizer.json) → `models\ruaccent\nn\nn_omograph\big_poetry\tokenizer.json` | 185 MB | como está, no caminho após a seta |
| Smart Turn v3.2 (end of phrase in Live) | opcional | [smart-turn-v3.2-f32.gguf](https://huggingface.co/audio-cpp/audio.cpp-gguf/resolve/73ce4162f6cce370dd533c6782b772135c1b2a8e/Smart-Turn-v3-GGUF/smart-turn-v3.2-f32.gguf) → `models\smart-turn\smart-turn-v3.2-f32.gguf` | 32 MB | como está, no caminho após a seta |
| Silero VAD v6.2 | recomendado | [silero-vad-v6.2-F32.gguf](https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/a9b2d61c20ae4ddf3644edb34969e8185a485448/silero-vad-v6.2-F32.gguf) → `models\silero-vad\silero-vad-v6.2-F32.gguf` | 1 MB | como está, no caminho após a seta |
| VoxLingua107 language identification Q8_0 | obrigatório | [lang-id-voxlingua107-ecapa-Q8_0.gguf](https://huggingface.co/handy-computer/lang-id-voxlingua107-ecapa-gguf/resolve/e59f0f3847e77723b7d58447ef9456fa0c7e9436/lang-id-voxlingua107-ecapa-Q8_0.gguf) → `models\langid-voxlingua107\lang-id-voxlingua107-ecapa-Q8_0.gguf` | 27 MB | como está, no caminho após a seta |
| DPDFNet 48 kHz (noise suppression) | recomendado | [dpdfnet2_48khz_hr.onnx](https://huggingface.co/Ceva-IP/DPDFNet/resolve/dd6818d00f50c836fed43a6243ebe49116de5964/onnx/dpdfnet2_48khz_hr.onnx) → `models\dpdfnet\dpdfnet2_48khz_hr.onnx` | 10 MB | como está, no caminho após a seta |
| BSRoformer.cpp (CUDA 13) | recomendado | [BSRoformer-windows-cuda13-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda13-x64.zip) | 119 MB | descompactar os arquivos, sem subpastas, em `tools\bsroformer\` |
| BSRoformer.cpp (CPU) | opcional | [BSRoformer-windows-cpu-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cpu-x64.zip) | 3 MB | descompactar os arquivos, sem subpastas, em `tools\bsroformer-cpu\` |
| BSRoformer.cpp (Vulkan) | opcional | [BSRoformer-windows-vulkan-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-vulkan-x64.zip) | 26 MB | descompactar os arquivos, sem subpastas, em `tools\bsroformer-vulkan\` |
| llama.cpp server (CUDA 13) | opcional | [llama-b12248-bin-win-cuda-13.5-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b12248/llama-b12248-bin-win-cuda-13.5-x64.zip) | 171 MB | descompactar os arquivos, sem subpastas, em `tools\llama\` |
| ONNX Runtime | obrigatório | [onnxruntime-win-x64-1.31.1.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.31.1/onnxruntime-win-x64-1.31.1.zip) | 83 MB | descompactar com a árvore de pastas em `models\runtime\` |
| ONNX Runtime GPU (CUDA) | recomendado | [onnxruntime-win-x64-gpu_cuda13-1.31.1.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.31.1/onnxruntime-win-x64-gpu_cuda13-1.31.1.zip) | 309 MB | descompactar com a árvore de pastas em `models\runtime\` |
| FFmpeg (static build) | obrigatório | [ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip](https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip) | 199 MB | pegar ffmpeg.exe e ffprobe.exe do arquivo e colocar em `tools\ffmpeg\` |
| yt-dlp + deno | opcional | [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe) → `tools\yt-dlp\yt-dlp.exe`<br>[deno-x86_64-pc-windows-msvc.zip](https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip) | 66 MB | como está, no caminho após a seta<br>descompactar os arquivos, sem subpastas, em `tools\yt-dlp\` |
| CUDA 13 runtime (cudart, cuBLAS, cuFFT) | obrigatório | [nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/86/00/d5436004268f049214193659ebc36550b5ef3925c3d13b4cc980e13be6f5/nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl)<br>[nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/a3/df/f1246959833e2c437db8be3e5b477f66b87f8817821ed40de6c7561c9a36/nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl)<br>[libcufft-windows-x86_64-12.4.0.43-archive.zip](https://developer.download.nvidia.com/compute/cuda/redist/libcufft/windows-x86_64/libcufft-windows-x86_64-12.4.0.43-archive.zip) | 586 MB | pegar todos os .dll do arquivo e colocar em `models\higgs-engine\` |
| cuDNN 9 | recomendado | [nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/87/6a/e55ff0ac26a5c6e2b21f41c9d04ad096b4ed6da593fba7e25845c61b0532/nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl) | 436 MB | pegar todos os .dll do arquivo e colocar em `models\higgs-engine\` |
| audio.cpp server, CUDA 12.9 (cards and drivers CUDA 13 left) | opcional | [audiocpp-server-win-cuda12-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda12-x64.zip) | 184 MB | descompactar os arquivos, sem subpastas, em `tools\audiocpp-cuda12\` |
| audio.cpp server, Vulkan | opcional | [audiocpp-server-win-vulkan-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-vulkan-x64.zip) | 36 MB | descompactar os arquivos, sem subpastas, em `tools\audiocpp-vulkan\` |
| audio.cpp server, CPU | opcional | [audiocpp-server-win-cpu-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cpu-x64.zip) | 19 MB | descompactar os arquivos, sem subpastas, em `tools\audiocpp-cpu\` |
| CUDA 12.9 runtime (cudart, cuBLAS, cuFFT) | opcional | [nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/59/df/e7c3a360be4f7b93cee39271b792669baeb3846c58a4df6dfcf187a7ffab/nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl)<br>[nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/e2/fc9a0e985249d873150276d5afb02e39a66817fedbf1a385724393e505ed/nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl)<br>[nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/ee/29955203338515b940bd4f60ffdbc073428f25ef9bfbce44c9a066aedc5c/nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl) | 757 MB | pegar todos os .dll do arquivo e colocar em `models\cuda12-runtime\` |
| Qwen3-TTS CUDA 13 module | opcional | [ggml-cuda13-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda13-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 142 MB | como está, no caminho após a seta |
| Qwen3-TTS CUDA 12.9 module | opcional | [ggml-cuda12-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda12-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 170 MB | como está, no caminho após a seta |
| Qwen3-TTS Vulkan module | opcional | [ggml-vulkan-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-vulkan-windows-x64.dll) → `models\qwen3-tts\ggml-vulkan.dll` | 42 MB | como está, no caminho após a seta |
| BSRoformer.cpp (CUDA 12.9) | opcional | [BSRoformer-windows-cuda12-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda12-x64.zip) | 173 MB | descompactar os arquivos, sem subpastas, em `tools\bsroformer-cuda12\` |
| llama.cpp server (CUDA 12.4) | opcional | [llama-b12248-bin-win-cuda-12.4-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b12248/llama-b12248-bin-win-cuda-12.4-x64.zip) | 289 MB | descompactar os arquivos, sem subpastas, em `tools\llama-cuda12\` |
| llama.cpp server (Vulkan) | opcional | [llama-b12248-bin-win-vulkan-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b12248/llama-b12248-bin-win-vulkan-x64.zip) | 38 MB | descompactar os arquivos, sem subpastas, em `tools\llama-vulkan\` |
| llama.cpp server (CPU) | opcional | [llama-b12248-bin-win-cpu-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b12248/llama-b12248-bin-win-cpu-x64.zip) | 25 MB | descompactar os arquivos, sem subpastas, em `tools\llama-cpu\` |
<!-- downloads:end -->

## O que roda onde

Cada etapa tem o seu dispositivo em Configurações → Modelos: Auto, GPU (CUDA), Vulkan ou CPU. Auto pega o build CUDA da placa — CUDA 13, ou CUDA 12.9 numa placa ou num driver mais antigos.

| Etapa | Motor | NVIDIA (CUDA) | Vulkan (AMD, Intel) | CPU | Mac (Apple silicon) | Nuvem |
|---|---|---|---|---|---|---|
| Voz e clonagem | Higgs Audio v3 (audio.cpp) | sim | sim | sim | Metal | OpenRouter, Gemini, fal.ai, Runware |
| Voz e clonagem | Qwen3-TTS (qwentts.cpp) | sim | sim | sim | Metal | — |
| Voz e clonagem | Audio8 TTS (audio.cpp) | sim | sim | sim | Metal | — |
| Voz e clonagem | Fish Audio Drama 3 (drama-rs) | só CUDA 13 | não | não | não | — |
| Voz | Supertonic 3 (audio.cpp) | sim | sim | sim | Metal | — |
| Reconhecimento de fala | Parakeet-TDT v3, Nemotron 3.5 (audio.cpp) | sim | sim | sim | Metal | OpenRouter, fal.ai |
| Reconhecimento de fala | Parakeet Ultra (transcribe.cpp) | CUDA 13 | sim | sim | Metal | — |
| Reconhecimento de fala | CrisperWhisper 2.0 turbo (audio.cpp) | sim | não | sim | Metal | — |
| Falantes | Nemotron 3 Diarization (audio.cpp) | sim | sim | sim | Metal | não |
| Pausas e ruído | Silero VAD v6 (transcribe.cpp), DPDFNet | não | não | sim | CPU | não |
| Voz separada da música | Mel-Band Roformer (BSRoformer.cpp) | sim | sim | sim | Metal | não |
| Acentos e homógrafos | RUAccent | não | não | sim | CPU | não |
| Diretor (podcast, personagens) | Gemma-4 12B (llama.cpp) | sim | sim | sim | Metal | OpenRouter |

## Solução de problemas

**O instalador para no WebView2.** Instale o WebView2 com o instalador [Evergreen Standalone x64](https://go.microsoft.com/fwlink/p/?LinkId=2124701) da Microsoft e rode de novo o instalador do Speech Studio.

**Os downloads travam ou falham.** Arquivos grandes continuam de onde pararam: clique de novo. Se o Hugging Face ou o GitHub estiverem bloqueados, configure um proxy em Configurações → **Rede**, ou coloque os arquivos da tabela no lugar e clique em **Importar da pasta**.

**Um motor de voz fala de placa ou driver antigos.** Os builds CUDA 13 precisam de uma placa a partir da Turing (GTX 16, RTX 20 e mais novas) e do driver 580 ou mais novo; GTX 900, GTX 10, Volta e drivers 525–579 recebem sozinhos os builds CUDA 12. O Drama 3 só tem build CUDA 13: numa placa ou num driver mais antigos, atualize o driver ou use outro motor ou as vozes na nuvem.

**O reconhecimento não encontra fala.** A gravação está abaixo do limiar de fala: reduza-o em **Reconhecimento → Corte nas pausas**.

**Configurações → Diagnóstico** junta um pacote para relatar erros sem chaves: anexe-o a uma [issue](https://github.com/timoncool/speech-studio/issues).

## Como funciona

**Stack:** um shell nativo **Tauri 2 (Rust)** roda o `speech-server` (axum) no mesmo processo em `127.0.0.1:8797` e abre uma janela com a SPA — React 19 + Vite + Tailwind. Os motores vivem no processo pelas suas ABIs em C: Drama 3 (`drama_engine.dll`, [timoncool/drama-rs](https://github.com/timoncool/drama-rs)), Qwen3-TTS (`qwen.dll`, [timoncool/qwentts.cpp](https://github.com/timoncool/qwentts.cpp)); Parakeet Ultra, Silero VAD e VoxLingua107 (`transcribe_engine.dll`, [timoncool/transcribe.cpp](https://github.com/timoncool/transcribe.cpp)); DPDFNet, RUAccent e WeSpeaker sobre o ONNX Runtime. Higgs Audio v3, Audio8 TTS, Supertonic 3, Parakeet-TDT v3, Nemotron 3.5, CrisperWhisper e Nemotron 3 Diarization rodam num programa auxiliar, o servidor audio.cpp (`audiocpp_server`, [timoncool/audio.cpp](https://github.com/timoncool/audio.cpp)); BSRoformer.cpp e o servidor do llama.cpp também são auxiliares; ffmpeg para o áudio. **Nenhum processo Python em execução.**

### Compilar a partir do código

```bash
git clone https://github.com/timoncool/speech-studio.git
cd speech-studio

cd frontend && npm install && npm run build && cd ..   # 1) SPA
cargo build --release -p speech-server                  # 2) servidor nativo (axum)
cd desktop && npm install && npx tauri build            # 3) shell desktop (Tauri)
```

Precisa de Node 20+, Rust (toolchain MSVC) e WebView2. Os motores não precisam ser recompilados — o aplicativo baixa binários prontos. Os pacotes da versão saem de `scripts/build-release.ps1` (Windows), `scripts/build-linux-docker.sh` (Linux, num contêiner Ubuntu 22.04) e `scripts/build-release-macos.sh` (macOS, num Mac com Apple silicon).

### Linux (experimental)

O .deb e o AppImage para Linux x86-64 são **experimentais**: o mesmo código com os builds Linux dos mesmos motores, não testados pelo autor num desktop Linux real. Correções são muito bem-vindas como pull requests. O driver NVIDIA (580 ou mais novo para os builds CUDA 13, 525 ou mais novo para os CUDA 12), `libgomp1` e `libssl3` vêm do sistema; modelos e motores vão para `~/.local/share/speech-studio`.

### macOS (experimental)

O .dmg para macOS 13 ou mais novo em Apple silicon também é **experimental**: os builds macOS dos mesmos motores sobre Metal, não testados pelo autor num Mac real. O Drama 3 não é oferecido lá: ele roda só em placas NVIDIA. O aplicativo é assinado ad hoc e não notarizado, então o macOS barra a primeira execução: abra-o uma vez em Ajustes do Sistema → Privacidade e Segurança → **Abrir Mesmo Assim**. Modelos e motores vão para `~/Library/Application Support/Speech Studio`.

## Estatísticas anônimas e novidades

O aplicativo pede novidades ao servidor do autor ao iniciar e depois a cada hora; o pedido não leva nenhum identificador. A tela da primeira execução tem uma caixa **Enviar estatísticas de uso anônimas**, marcada por padrão; o mesmo interruptor fica em Configurações, ao lado de **O que é enviado**. Enquanto marcada, o aplicativo envia o relatório do dia: um id de instalação aleatório criado neste computador, a versão, o SO, o idioma da janela, a placa de vídeo como fabricante, uma faixa de memória de vídeo e se o CUDA funciona, quantas tarefas terminaram, falharam ou foram canceladas, os modelos de cada etapa, os segundos de fala que os motores locais geraram e os segundos que trabalharam para isso, e por que uma tarefa falhou, em uma linha sem caminhos, nomes nem texto entre aspas (num download que falhou, o componente e o site de onde vinha). Nunca: textos, gravações, vozes, nomes de arquivo ou nada pessoal. `DO_NOT_TRACK=1` ou `STUDIO_TELEMETRY=0` desligam as estatísticas por completo.

## Mais aplicativos de IA portáteis

| Projeto | O que é |
|--------|----------|
| [Dub Studio](https://github.com/timoncool/dub-studio) | Dublagem de vídeo com IA: vozes clonadas, legendas traduzidas e texto na tela |
| [ACE-Step Studio](https://github.com/timoncool/ACE-Step-Studio) | Estúdio de música com IA — músicas, vocais, covers e clipes |
| [YuE2 Studio](https://github.com/timoncool/YuE2-Studio) | Gerador de músicas com partitura editável |
| [MiniMax Music3 Studio](https://github.com/timoncool/MiniMax-Music3-Studio) | Estúdio de música nativo local/nuvem com MiniMax Music 3 |
| [VideoSOS](https://github.com/timoncool/videosos) | Estúdio de vídeo com IA |

## Contribuições e forks

Colaboradores são muito bem-vindos: correções dos builds **macOS e Linux**, **placas AMD e Intel** no Vulkan e mais idiomas da interface (`frontend/src/locales/`, `crates/speech-server/locales/`) — faça um fork e envie um pull request.

## Autores

- **Nerual Dreming** — [Telegram](https://t.me/nerual_dreming) | [X](https://x.com/nerualdreming) | [neuro-cartel.com](https://neuro-cartel.com) | fundador do [ArtGeneration.me](https://artgeneration.me)
- **Neuro-Soft** — [Telegram](https://t.me/neuroport) | aplicativos de IA portáteis

## Créditos

- **[Boson AI](https://huggingface.co/bosonai)** — Higgs Audio v3, e **[drbaph / Higgs-Audio-v3-Studio](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio)** — suas quantizações GGUF.
- **[Fish Audio](https://fish.audio)** — Drama 3; **[apiplant/drama-q4](https://huggingface.co/apiplant/drama-q4)** — seus pesos Q4; **[drama-rs](https://github.com/timoncool/drama-rs)** — o motor Rust/CUDA.
- **[Equipe Qwen](https://huggingface.co/Qwen)** — Qwen3-TTS; **[ServeurpersoCom / qwentts.cpp](https://github.com/ServeurpersoCom/qwentts.cpp)** e **[Serveurperso/Qwen3-TTS-GGUF](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF)** — o motor C++ e as quantizações GGUF; **[siriusfreak](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf)** — a LoRA de acento russo.
- **[Edge0](https://huggingface.co/Edge0/Audio8-TTS-Realistic-Air)** — Audio8 TTS Realistic-Air; **[Hansda/Audio8-TTS-Realistic-Air-GGUF](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF)** — suas quantizações GGUF.
- **[Supertone](https://huggingface.co/Supertone)** — Supertonic 3; **[askurios8/supertonic-3-int8](https://huggingface.co/askurios8/supertonic-3-int8)** — sua exportação int8.
- **[RUAccent](https://huggingface.co/ruaccent)** — acentos, «ё» e homógrafos.
- **[NVIDIA](https://huggingface.co/nvidia)** — Parakeet-TDT, Nemotron 3.5 ASR e Nemotron 3 Diarization; o ONNX da diarização vem do [altunenes/parakeet-rs](https://github.com/altunenes/parakeet-rs).
- **[handy-computer / transcribe.cpp](https://github.com/handy-computer/transcribe.cpp)** — o motor transcribe.cpp e os GGUF de Parakeet-TDT, Parakeet Ultra, Nemotron 3.5, Silero VAD v6.2 e VoxLingua107.
- **[Silero](https://github.com/snakers4/silero-vad)** — Silero VAD v6; **[Ceva-IP / DPDFNet](https://huggingface.co/Ceva-IP/DPDFNet)** — supressão de ruído.
- **[nyra health / CrisperWhisper 2.0 turbo](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo)** — reconhecimento Whisper com o tempo das palavras; **[0xShug0 / audio.cpp](https://github.com/0xShug0/audio.cpp)** — o motor em que rodam o Higgs Audio v3, o Audio8 TTS, o Supertonic 3, o Parakeet-TDT v3, o Nemotron 3.5, o CrisperWhisper e o Nemotron 3 Diarization.
- **[chenmozhijin / BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp)** e **[GaboxR67](https://huggingface.co/GaboxR67)** — separação de voz.
- **[WeSpeaker](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM)** — semelhança de voz; **[Google Gemma](https://huggingface.co/google)** e **[llama.cpp](https://github.com/ggml-org/llama.cpp)** — o diretor.
- **[ONNX Runtime](https://github.com/microsoft/onnxruntime)**, **[FFmpeg](https://ffmpeg.org)**, **[Tauri](https://tauri.app)**, **[ort](https://github.com/pykeio/ort)** e as bibliotecas NVIDIA CUDA.

## Apoie o autor

Eu faço software livre e pesquiso IA — quase tudo o que crio é gratuito. As doações me deixam criar e pesquisar mais.

**[Todas as formas de apoiar](DONATE.md)** | **[dalink.to/nerual_dreming](https://dalink.to/nerual_dreming)** | **[boosty.to/neuro_art](https://boosty.to/neuro_art)**

- **BTC:** `1E7dHL22RpyhJGVpcvKdbyZgksSYkYeEBC`
- **ETH (ERC20):** `0xb5db65adf478983186d4897ba92fe2c25c594a0c`
- **USDT (TRC20):** `TQST9Lp2TjK6FiVkn4fwfGUee7NmkxEE7C`

## Histórico de estrelas

<a href="https://github.com/timoncool/speech-studio/stargazers">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="docs/stars-dark.svg" />
   <source media="(prefers-color-scheme: light)" srcset="docs/stars-light.svg" />
   <img alt="Star History Chart" src="docs/stars-light.svg" />
 </picture>
</a>

## Licença

O código do aplicativo é [MIT](LICENSE). **Os modelos não**: cada um mantém sua licença. Leia-as antes de publicar ou vender o que você locutar.

| Componente | Licença | O que significa |
|---|---|---|
| Higgs Audio v3 (Boson AI) | [Boson Higgs TTS 3 Research and Non-Commercial License](https://huggingface.co/bosonai/higgs-tts-3-4b/blob/main/LICENSE) | Pesquisa e uso pessoal; criadores podem monetizar seu próprio conteúdo com o Creator Use Grant se citarem o Higgs Audio da Boson AI; um produto ou serviço para outros precisa de licença da Boson |
| Fish Audio Drama 3 | [Fish Audio Community License](https://huggingface.co/fishaudio/drama-3/blob/main/LICENSE) | Pesquisa e uso não comercial; uso comercial enquanto a sua receita anual for menor que 500 000 USD, acima disso licença da Fish Audio; sem serviço hospedado de texto para fala |
| Qwen3-TTS (Qwen) | Apache-2.0 |  |
| LoRA de acento russo do Qwen3-TTS (siriusfreak) | Apache-2.0 |  |
| Audio8 TTS Realistic-Air (Edge0) | Apache-2.0 |  |
| Supertonic 3 (Supertone) | BigScience OpenRAIL-M | Leia as restrições de uso |
| RUAccent e seu modelo de homógrafos em int8 | MIT |  |
| Parakeet-TDT, Nemotron 3 Diarization (NVIDIA) | CC-BY-4.0, OpenMDW-1.1 | Cite a NVIDIA |
| Nemotron 3.5 ASR (NVIDIA) | OpenMDW-1.1 | Cite a NVIDIA |
| transcribe.cpp | MIT |  |
| Silero VAD | MIT |  |
| DPDFNet (Ceva) | Apache-2.0 |  |
| CrisperWhisper 2.0 turbo (nyra health) | MIT para o código; nyra health Non-Commercial Research License para os pesos e o que eles geram | Só uso não comercial; o uso comercial precisa de uma licença da nyra health |
| audio.cpp | Apache-2.0 |  |
| Mel-Band Roformer voc_fv6, GGUF | As páginas dos modelos não indicam licença | Pergunte aos autores antes do uso comercial |
| WeSpeaker ResNet34-LM | CC-BY-4.0 | Cite o WeSpeaker |
| Gemma-4 12B (Google) | Termos do Gemma | Leia-os |
| llama.cpp, ONNX Runtime, BSRoformer.cpp, qwentts.cpp | MIT |  |
| ElevenLabs UI Orb, SiriWave | MIT |  |
| LiveKit Agents UI Aura | Apache-2.0 |  |
| FFmpeg (build BtbN) | Build GPL | Programa à parte, não ligado ao aplicativo |
| NVIDIA CUDA runtime, cuBLAS, cuFFT, cuDNN | Termos próprios da NVIDIA | Baixados da NVIDIA e do PyPI |

<sub>texto para fala · clonagem de voz · TTS em russo · acentos · criador de audiolivros · podcast · reconhecimento de fala · transcrição · diarização · offline · local · código aberto · Windows · macOS · Linux</sub>

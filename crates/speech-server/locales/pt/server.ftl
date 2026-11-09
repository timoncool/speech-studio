asr-windowed = transcrição por janelas na GPU ({ $speakers } { $speakers ->
    [one] falante
   *[other] falantes
})

## Separation, OCR, benchmark, casting library, cloud ASR and TTS

atomic-extract-separation-audio = extraindo áudio 44,1k para a separação
atomic-separating = separando ({ $model })
atomic-separation-failed = separação: { $error }
ocr-detecting-burned-text = detectando texto embutido ({ $model })
bench-stage = ⏱ { $name }: { $seconds } s | GPU ~{ $gpu_avg }% (pico { $gpu_max }%) | CPU ~{ $cpu_avg }% | VRAM { $vram } MB | limite: { $bound }
bench-total = ⏱ { $label } TOTAL: { $seconds } s
casting-no-casting-json = o projeto não tem casting.json (o casting não foi executado)
casting-profile-delete-failed = excluindo o perfil { $slug }: { $error }
casting-profile-not-found = perfil “{ $slug }” não encontrado
cloud-asr-no-key = o ASR na nuvem está ativado, mas a chave do OpenRouter não foi definida
cloud-asr-no-model = nenhum modelo STT do OpenRouter foi escolhido nas configurações
cloud-asr-failed = ASR na nuvem: { $error }
cloud-asr-empty = o STT na nuvem retornou uma transcrição vazia
cloud-tts-no-key = o TTS na nuvem está ativado, mas a chave do OpenRouter não foi definida
cloud-tts-no-voice = nenhuma voz TTS foi definida nas configurações (cada modelo tem suas próprias vozes)
cloud-tts-failed = TTS na nuvem: { $error }
cloud-tts-too-short = TTS na nuvem: o áudio é curto demais ({ $bytes } { $bytes ->
    [one] byte
   *[other] bytes
})
cloud-tts-read-wav = lendo o wav da nuvem: { $error }

## Shared messages, compositing, downloads, dub timing, endpoints, preview frame

common-read = lendo { $path }: { $error }
common-parse = analisando { $path }: { $error }
common-ffmpeg-start = iniciando o ffmpeg: { $error }
compose-summary = composição: títulos={ $titles } (bbox), desfoque em { $boxes } { $boxes ->
    [one] caixa
   *[other] caixas
}, sub_px={ $sub_px }
compose-taglines-no-mt = slogans: a tradução automática está indisponível ({ $error }) -> só desfoque
compose-taglines-translating = slogans: traduzindo os textos da cartela
compose-taglines-failed = slogans: a tradução falhou; só desfoque
downloads-interrupted = o download parou junto com o aplicativo; o que foi baixado está salvo em .part e continuará dali
downloads-nothing-to-download = nenhum dos ids escolhidos é um componente baixável
downloads-busy = já há um download em andamento
downloads-thread-failed = a thread de download não iniciou: { $error }
downloads-panicked = o download parou num erro interno: { $error }
timing-words-not-recognized = os tempos por palavra da dublagem não foram reconhecidos em { $count } { $count ->
    [one] fala
   *[other] falas
} ({ $examples }); as palavras delas são destacadas pelo comprimento
openrouter-catalog-failed = catálogo do OpenRouter: { $error }
openrouter-empty-key = a chave está vazia
llm-server-no-address = o endereço do servidor não foi definido
llm-server-no-answer = o servidor { $base } não respondeu: { $reason }
llm-server-status = { $endpoint } respondeu { $status }
llm-server-not-json = { $endpoint } não retornou JSON: { $error }
llm-server-not-model-list = { $endpoint } não retornou uma lista de modelos OpenAI (sem o campo data)
remix-start = remixando { $count } { $count ->
    [one] linha
   *[other] linhas
} → { $instruction }
remix-no-llm = remix: o LLM está indisponível: { $error }
remix-lines-changed = remix: as falas mudaram durante o remix; o projeto não foi alterado, execute o remix de novo
frame-empty-ass = ASS vazio: { $error }
frame-read-preview = lendo o quadro de prévia: { $error }
frame-read-source = lendo o quadro original: { $error }

## Glossary, hardware, job records, LLM providers

glossary-bad-format = format “{ $format }”: json ou tsv
glossary-series-unreadable = o glossário da série não pode ser lido: { $error }
glossary-no-text = o projeto não tem texto: o glossário é montado a partir da fala reconhecida, analise primeiro
glossary-lines = glossário: { $count } { $count ->
    [one] linha
   *[other] linhas
} de texto
glossary-no-llm = glossário: o LLM está indisponível: { $error }
glossary-failed = glossário: { $error }
glossary-proposed = glossário: { $count } { $count ->
    [one] entrada proposta
   *[other] entradas propostas
}
glossary-series-profile-missing = perfil da série “{ $slug }” não encontrado; o glossário dele não foi aplicado
glossary-series-slug-unreadable = o glossário da série “{ $slug }” não pode ser lido: { $error }
glossary-series-applied = glossário da série “{ $slug }”: { $count } { $count ->
    [one] entrada
   *[other] entradas
}, { $added } adicionadas ao projeto
hw-no-nvidia = sem GPU NVIDIA
jobs-serialize-record = serializando job.json: { $error }
jobs-record-missing = { $file } não está em { $dir }
llm-llama-server-missing = llama-server não encontrado ({ $path })
llm-gemma-missing = o GGUF do Gemma não foi encontrado ({ $path })
llm-mmproj-missing = o projetor de visão do Gemma (mmproj) não foi encontrado ({ $path })
llm-chat-client = cliente de chat: { $error }
llm-local-no-text-model = o servidor local ({ $url }) foi escolhido para a tradução, mas nenhum modelo foi selecionado
llm-local-no-vision-model = o servidor local ({ $url }) foi escolhido para visão, mas nenhum modelo foi selecionado
llm-local-client = cliente do servidor local: { $error }
llm-local-label = servidor local { $url } · { $model }
llm-openrouter-no-key = o OpenRouter foi escolhido, mas a chave não foi definida
llm-agent-no-vision = o agente conectado por MCP não vê imagens; escolha outro provedor para elas
llm-agent-label = Agente (MCP)
llm-agent-no-port = a porta do servidor ainda não é conhecida; tente de novo em instantes
llm-agent-absent = o modelo do estúdio é o agente ligado (Definições → Modelos), mas agora nenhum agente está ligado: ligue-o (Definições → Agente) ou escolha outro modelo
llm-openrouter-no-text-model = o OpenRouter foi escolhido para a tradução, mas nenhum modelo foi selecionado
llm-openrouter-no-vision-model = o OpenRouter foi escolhido para visão, mas nenhum modelo foi selecionado
llm-openrouter-not-text = o modelo do OpenRouter { $model } não responde em texto; escolha outro para a tradução
llm-openrouter-not-vision = o modelo do OpenRouter { $model } não aceita imagens; escolha um modelo de visão
llm-vision-missing = nenhum ({ $reason })
llm-pair = tradução: { $text }; visão: { $vision }

## Media tools and OCR

common-ffmpeg-exit = o ffmpeg saiu com o código { $code }:
    { $tail }
media-ffprobe-start = o ffprobe não iniciou: { $error }
media-ffprobe-exit = o ffprobe saiu com o código { $code }: { $stderr }
media-ffprobe-no-streams = ffprobe: sem streams
media-no-streams = a entrada não tem fluxo de vídeo nem de áudio
media-no-duration = não foi possível determinar a duração
media-no-wav = o ffmpeg não criou o wav
media-ffmpeg-hung = o ffmpeg não terminou em { $seconds } s e foi encerrado (travou)
media-ffprobe-duration-exit = ffprobe duration saiu com o código { $code }
media-env-filter-script = script de filtro de envelope: { $error }
media-no-sample-rate = { $path }: a taxa de amostragem não foi lida ({ $stderr })
ocr-no-blur = { $error }; sem desfoque
ocr-models-missing = os modelos de OCR não foram encontrados
ocr-detection-failed = a detecção OCR falhou ({ $error })
ocr-summary = OCR: { $regions } { $regions ->
    [one] região
   *[other] regiões
}, { $localize } para localizar, { $bands } { $bands ->
    [one] faixa
   *[other] faixas
}, sub_y={ $sub_y }

## Hardware presets

common-write = gravando { $what }: { $error }
preset-rtx5090-title = RTX 5090 (32 GB)
preset-rtx4090-title = RTX 4090 (24 GB)
preset-top-subtitle = Qualidade máxima: os melhores quants localmente
preset-gpu16-title = GPU de 16 GB
preset-gpu16-subtitle = Alta qualidade (4080/4070 Ti e similares)
preset-gpu12-title = GPU de 12 GB
preset-gpu12-subtitle = Equilibrado (3060/4070 e similares)
preset-gpu8-title = GPU de 8 GB
preset-gpu8-subtitle = Econômico: quants leves (3060 Ti/4060)
preset-weak-nvidia-cloud-title = NVIDIA fraca + nuvem
preset-weak-nvidia-cloud-subtitle = O pesado (tradução/visão/voz) no OpenRouter, separação e ASR na sua GPU
preset-cloud-title = CPU + nuvem (sem NVIDIA)
preset-cloud-subtitle = O pesado no OpenRouter, o local no processador; roda sem placa de vídeo (é preciso uma chave)
preset-custom-title = Personalizado
preset-custom-subtitle = Vou ajustar cada parâmetro manualmente
preset-reason-no-gpu = Nenhuma GPU NVIDIA encontrada: modo “CPU + nuvem”, o pesado no OpenRouter e o local no processador (mais lento, mas funciona)
preset-reason-top-card = { $gpu } detectada: quants máximos
preset-reason-by-vram = { $gpu } · { $vram } GB de VRAM: { $preset }
preset-reason-low-vram = { $gpu } · { $vram } GB de VRAM é pouco para modelos locais; a nuvem é mais confiável
preset-unknown = predefinição desconhecida: { $id }

## Service port, shortening, frontend, launch defaults

common-corrupt = { $path } está corrompido: { $error }
common-create-dir = pasta { $path }: { $error }
service-bad-port = { $value }: não é um número de porta (esperado 1..65535)
service-exe-path = caminho do exe do serviço: { $error }
service-request-not-sent = a conexão foi aceita, mas a requisição não foi enviada: { $error }
service-no-health-answer = a conexão foi aceita, mas /health não respondeu: { $error }
service-not-http = a resposta não é HTTP
service-health-status = um servidor HTTP; /health respondeu { $code }
service-health-not-dub-studio = um servidor HTTP; /health não respondeu com um corpo do Speech Studio
service-health-not-json = um servidor HTTP; /health não respondeu com JSON
service-health-no-fields = /health diz ser { $app }, mas sem os campos do serviço: { $error }
service-other-app = outro aplicativo ({ $app })
service-health-no-app = um servidor HTTP; /health sem nome de aplicativo
service-port-reserved = o sistema não libera a porta, embora ninguém aceite conexões nela (talvez esteja numa faixa reservada pelo Windows: { $command })
service-port-dub-studio = o Speech Studio { $version } está nela ({ $executable })
service-port-other = outro processo a ocupa: { $what }
service-port-busy = A porta 127.0.0.1:{ $port } está ocupada há { $seconds } s: { $who }. Erro: { $error }.

    Feche o programa que a ocupa ou defina outra porta com a variável de ambiente { $env } (por exemplo { $env }={ $other_port }).
shorten-line-done = encurtando { $n }/{ $total }: { $from } -> { $to } caracteres
shorten-line-rejected = encurtando { $n }/{ $total }: a resposta não foi aceita ({ $reason })
shorten-line-no-answer = encurtando { $n }/{ $total }: o LLM não respondeu: { $error }
shorten-auto-start = { $count } { $count ->
    [one] fala não cabe
   *[other] falas não cabem
} no espaço; encurtando a tradução e dublando só elas
shorten-higgs-unloaded = o Higgs foi descarregado enquanto a tradução é encurtada
shorten-auto-no-llm = o encurtamento da tradução foi pulado: o LLM está indisponível: { $error }
shorten-none-shortened = encurtamento: nenhuma das { $count } falas ficou mais curta; ficam como foram dubladas
shorten-done = { $count } de { $total } { $total ->
    [one] fala encurtada
   *[other] falas encurtadas
}
shorten-nothing = nada a encurtar: todas as falas cabem no seu espaço
shorten-no-llm = encurtamento: o LLM está indisponível: { $error }
shorten-start = encurtando { $count } { $count ->
    [one] fala
   *[other] falas
}: { $provider }
shorten-all-failed = o encurtamento falhou: o LLM não respondeu a nenhuma fala ({ $id }: { $error })
spa-not-built = o frontend não foi compilado
settings-bad-speaker-count = speaker_count: espera-se um número inteiro de 0 a { $max } (0 é automático)
settings-bad-vo-gain = vo_gain_db={ $value }: espera-se um número de { $min } a { $max } dB
settings-bad-src-lang = src_lang={ $value }: não é um código de idioma nem "auto"
settings-bad-tgt-lang = tgt_lang={ $value }: não é um código de idioma
settings-bad-casting-ref = casting_ref={ $value }: não é o slug de um perfil de casting
settings-style-too-long = tr_style_custom tem mais de { $max } caracteres
settings-too-many-slots = { $name }: mais de { $max } espaços
settings-unknown-field = campo de padrões de início desconhecido: { $key }
settings-read-failed = lendo os padrões de início: { $error }
settings-patch-not-object = o corpo de PATCH /settings/launch é um objeto de campos
settings-write-failed = gravando os padrões de início: { $error }

## Takes, translation, TTS text, MCP arguments

takes-delete-old = excluindo a tomada antiga { $path }: { $error }
takes-missing = a tomada { $take } não está no histórico da fala { $line }
translate-no-vision = o tipo de conteúdo não foi determinado: a visão está indisponível ({ $error })
translate-subs-already-translated = as legendas já estão no idioma de destino -> sem tradução automática (só voz)
translate-transcribe-only = transcrever: tgt=o texto original, sem tradução
translate-same-language = mesmo idioma -> sem tradução automática (tgt=o original)
translate-no-llm = a tradução falhou: o LLM está indisponível: { $error }
translate-ctx-pass = passada de contexto: visão de layout/cena + tradução da transcrição
translate-failed = a tradução falhou: { $error }
translate-untranslated = a tradução falhou: { $left } de { $total } { $total ->
    [one] linha
   *[other] linhas
} ficaram no idioma original (detalhes no registro e em logs/llama-server.log)
translate-untranslated-auto = a tradução falhou: { $left } de { $total } { $total ->
    [one] linha
   *[other] linhas
} ficaram no idioma original; se a fala do vídeo já está no idioma de destino, indique o idioma original e a tradução não será necessária (detalhes no registro e em logs/llama-server.log)
translate-done = tradução pronta: { $done }/{ $total } linhas, títulos={ $titles }
translate-left-untranslated = { $left } de { $total } { $total ->
    [one] linha
   *[other] linhas
} ficaram no idioma original (detalhes em logs/llama-server.log)
translate-coverage-retry = cobertura da tradução: { $count } { $count ->
    [one] linha
   *[other] linhas
} sem tradução; traduzindo de novo
translate-coverage-failed = cobertura da tradução: a nova tradução falhou ({ $error })
translate-coverage-left = cobertura da tradução: restam { $count } sem tradução
tts-silent-after-cleanup = nenhuma fala restou após limpar o texto, então silêncio: { $count } { $count ->
    [one] fala
   *[other] falas
} ({ $lines })
mcp-bad-speaker-count = speaker_count: espera-se um número inteiro de 0 a { $max }
mcp-speakers-without-diarize = speaker_count maior que 1 não é compatível com diarize=false

## Routes: projects, voices, setup, translation, export, jobs, alignment

project-serialize = serializando project.json: { $error }
project-delete-failed = excluindo o projeto { $path }: { $error }
setup-no-ids = ids está vazio
setup-fetching-missing = Baixando os modelos que faltam para esta função…
setup-diarization-missing = O modelo de diarização não foi baixado ({ $error }); a análise seguirá sem separar os falantes
setup-pick-model-files = Arquivo(s) do modelo
setup-pick-models-folder = Pasta com modelos prontos
setup-no-folder = a pasta { $path } não existe
voices-not-found = a voz { $name } não foi encontrada em voices/
voices-no-vocals = não há voz para medir o F0; analise primeiro
voices-speakers-changed = vozes por espaços: os falantes mudaram enquanto a voz era medida; o projeto não foi alterado, execute de novo
analyze-args-not-object = argumentos de analyze: esperava-se um objeto
cost-analyze = OpenRouter: US${ $spent } gastos na análise (US${ $total } usados no total)
cost-run = OpenRouter: US${ $spent } gastos na execução (US${ $total } usados no total)
translate-lines-to = Traduzindo { $count } { $count ->
    [one] linha
   *[other] linhas
} → { $lang }
translate-note = tradução: { $note }
translate-titles-failed = os títulos não foram traduzidos ({ $error }); no vídeo eles ficarão no idioma original
translate-titles-error = traduzindo os títulos: { $error }
translate-lines-changed = tradução: as falas ou os títulos mudaram durante a tradução; o projeto não foi alterado, execute a tradução de novo
export-pick-folder = Onde salvar os resultados
export-copy-failed = copiando para { $path }: { $error }
jobs-wait-not-number = wait: esperava-se um número de segundos
jobs-unknown-kind = tipo de tarefa desconhecido em job.json: { $kind }
jobs-not-resumable = uma tarefa { $kind } não pode ser retomada
align-no-source-text = alinhamento com a fala: as falas não têm texto no idioma original (as legendas foram importadas no idioma de destino)
align-no-vocals = alinhamento com a fala: o projeto não tem faixa de voz; analise primeiro
align-recognizing = alinhamento com a fala: reconhecendo as palavras
align-recognition-failed = alinhamento com a fala: reconhecimento: { $error }
align-no-speech = alinhamento com a fala: nenhuma fala reconhecida; os tempos não foram alterados
align-mismatch = alinhamento com a fala: as falas não coincidiram com a fala ({ $share }% correspondido); os tempos não foram alterados
align-lines-changed = alinhamento com a fala: as falas mudaram durante o reconhecimento; os tempos não foram alterados, execute o alinhamento de novo
align-done = alinhado com a fala: { $share }% das falas por palavras, tempo alterado em { $changed }; deslocamento { $offset } s

## Analysis

analyze-serialize = serializando { $what }: { $error }
analyze-read-model = lendo o modelo { $path }: { $error }
analyze-stage-cache-unreadable = o cache da etapa { $stage } não pode ser lido ({ $error }); recalculando
analyze-checkpoint-not-saved = o ponto de controle da etapa { $stage } não foi salvo: { $error }
analyze-diarize-continuous = diarização: { $speakers } { $speakers ->
    [one] falante esperado
   *[other] falantes esperados
}, uma passada contínua pela gravação inteira sem reiniciar rótulos a cada hora
analyze-diarize-fragments = diarização: { $speakers } { $speakers ->
    [one] falante esperado
   *[other] falantes esperados
}, associando vozes entre fragmentos
analyze-wespeaker-needed = o WeSpeaker é necessário para um número definido de falantes: { $error }
analyze-align-skipped = alinhamento com a fala pulado: as legendas estão no idioma de destino e a fala no original
analyze-align-recognizing = alinhando as legendas com a fala: reconhecendo as palavras
analyze-align-recognition-failed = alinhando as legendas: reconhecimento da fala: { $error }
analyze-align-no-speech = alinhamento com a fala: nenhuma fala reconhecida; os tempos do arquivo foram mantidos
analyze-align-mismatch = as legendas não coincidiram com a fala ({ $share }% das falas correspondidas); os tempos do arquivo foram mantidos
analyze-align-done = legendas alinhadas com a fala: { $share }% das falas por palavras, o resto deslocado junto com as vizinhas; deslocamento do arquivo { $offset } s
analyze-more = mais { $count }
analyze-window-plan = faixa longa de { $duration } s: plano de { $windows } { $windows ->
    [one] janela
   *[other] janelas
} (a primeira ~{ $first } s); o ASR por janelas ainda não está ativo, processamento em um bloco
analyze-audio-cached = áudio do cache (a fonte não mudou); pulando o ffmpeg
analyze-extracting-audio = extraindo o áudio (ffmpeg -> 16k mono)
analyze-stems-stale = os stems vieram de uma extração anterior; separando de novo
analyze-separating = separando a voz ({ $model }): voz limpa para a diarização/ASR
analyze-separation-cached = separação do cache (os stems já estão prontos)
analyze-separation-failed = a separação falhou ({ $error }); diarização/ASR no áudio bruto
analyze-separator-missing = { $model } não foi encontrado; diarização/ASR no áudio bruto
analyze-diarizing = diarização ({ $model })
analyze-diarization-cached = diarização do cache
analyze-diarization-count-failed = diarização com um número definido de falantes: { $error }
analyze-diarization-failed = a diarização falhou ({ $error }); seguindo com um único falante
analyze-diarization-model-missing-count = o modelo { $model } não foi encontrado: o número definido de falantes não pode ser aplicado
analyze-subs-no-diarization = legendas: sem diarização (o clipe inteiro de uma vez)
analyze-diarization-model-missing = o modelo de diarização não foi encontrado; seguindo com um único falante
analyze-fewer-speakers = eram esperados { $expected } { $expected ->
    [one] falante
   *[other] falantes
}, { $found } distinguidos: as vozes que faltam não foram adicionadas
analyze-speakers-matched = vozes associadas entre fragmentos: { $speakers } { $speakers ->
    [one] falante
   *[other] falantes
}
analyze-transcript-cached = transcrição do cache: { $segments } { $segments ->
    [one] segmento
   *[other] segmentos
}
analyze-read-subs = lendo as legendas { $path }: { $error }
analyze-subs-empty = as legendas não foram reconhecidas ou estão vazias: { $path }
analyze-subs-imported = legendas importadas: { $lines } { $lines ->
    [one] fala
   *[other] falas
}, { $speakers } { $speakers ->
    [one] falante
   *[other] falantes
}
analyze-cloud-asr = transcrevendo na nuvem (OpenRouter STT)
analyze-cloud-stt-failed = STT na nuvem: { $error }
analyze-cloud-done = nuvem: { $lines } { $lines ->
    [one] fala
   *[other] falas
}, { $speakers } { $speakers ->
    [one] falante
   *[other] falantes
}
analyze-hallucination-filter = filtro de alucinações do ASR: { $error }
analyze-asr-language = O Parakeet não transcreve «{ $lang }»: conhece 25 idiomas europeus. Escolha o CrisperWhisper em Definições → Modelos (99 idiomas); escolher o idioma de origem na janela troca-o automaticamente.
analyze-asr-no-words = O vídeo tem { $speech } s de fala e o Parakeet ouviu { $words } palavras: conhece apenas 25 idiomas europeus. Escolha o idioma de origem na janela em vez de Auto — para um idioma fora da Europa o estúdio troca o reconhecimento para o CrisperWhisper.
analyze-hidden-hallucinations = falas alucinadas do ASR ocultadas (sem voz): { $count }: { $lines }
analyze-hidden-by-text = títulos e sons do ASR ocultados pelo texto (há som no intervalo, a voz não está separada da música): { $count }: { $lines }
analyze-voiced-suspects = parecem alucinações, mas há voz; mantidas com uma marca: { $count }: { $lines }
analyze-merged-fragments = mesclando fragmentos: { $before } -> { $after } segmentos
analyze-characters-by-voice = personagens por voz: { $count }
analyze-segments-speakers = { $segments } { $segments ->
    [one] segmento
   *[other] segmentos
}, { $speakers } { $speakers ->
    [one] falante
   *[other] falantes
}
analyze-project-unparsable = project.json não pode ser lido ({ $error }): ele contém o glossário do projeto, então a análise parou para não perdê-lo
analyze-glossary-fixed = glossário: { $count } { $count ->
    [one] erro corrigido
   *[other] erros corrigidos
} no reconhecimento de termos
analyze-no-speech-nodub = não há segmentos de fala; mantendo a faixa original (nodub)
analyze-auto-nodub = auto: não há fala adequada para dublagem -> NODUB (o original + texto na tela localizado)
analyze-casting-style = descrições de personagens do perfil de casting -> estilo de tradução ({ $chars } caract.)
analyze-empty-removed = segmentos sem palavras removidos: { $count }
analyze-translation-cached = tradução do cache
analyze-ocr-cached = detecção de texto na tela do cache
analyze-audio-no-ocr = modo áudio: sem vídeo, a detecção de texto na tela não é necessária
analyze-ocr-off = a detecção de texto embutido está desativada (a caixa)
analyze-content-type = tipo de conteúdo (auto): { $kind }
analyze-casting-cached = casting do cache
analyze-casting-checkpoint = o ponto de controle do casting não foi salvo: { $error }
analyze-profile-voices-missing = as vozes do perfil não foram encontradas em voices/ -> clone: { $voices }
analyze-profile-voices-applied = as vozes do perfil foram aplicadas à dublagem ({ $count } { $count ->
    [one] personagem
   *[other] personagens
})
analyze-audio-no-casting = modo áudio: sem vídeo, o casting de personagens não é necessário
analyze-cache-not-saved = não foi possível salvar cache.json: { $error } (não é grave)

## Casting, recording, voice library

casting-relabel = rerrotulagem por voz: { $count } { $count ->
    [one] personagem
   *[other] personagens
} por voz (a diarização encontrou { $found })
casting-skipped-no-speech = casting pulado: não há segmentos de fala
casting-skipped-no-speakers = casting pulado: não há falantes
casting-speakers-ranked = personagens que falam: { $count } (ordenados por tempo de fala)
casting-faces-bound = rostos vinculados a falantes: { $count } de { $total }
casting-bit-part-skipped = { $character }: o falante { $speaker } tem 1 fala e nem rosto nem voz -> pulado (ponta sem casting)
casting-character-face = { $character }: falante { $speaker }, { $lines } { $lines ->
    [one] fala
   *[other] falas
}, { $seconds } s de fala, rosto: sim
casting-character-no-face = { $character }: falante { $speaker }, { $lines } { $lines ->
    [one] fala
   *[other] falas
}, { $seconds } s de fala, rosto: não
casting-profile-other-type = o perfil é de outro tipo ({ $previous } ≠ { $current }); correspondência cruzada pulada
casting-cross-episode = entre episódios: nomes/vozes transferidos: { $count }
casting-saved = casting.json pronto: { $count } { $count ->
    [one] personagem
   *[other] personagens
}
casting-save-failed = não foi possível gravar casting.json: { $error }
casting-faces-collected = rostos coletados: { $faces } ({ $samples } amostras)
casting-face-clusters = identidades de rosto (agrupamentos por vetor): { $count }
casting-anime-detector-missing = o detector de anime não foi encontrado ({ $path }); sem avatares
casting-anime-detector-failed = o detector de anime não carregou: { $error }; sem avatares
casting-scrfd-missing = o SCRFD não foi encontrado; sem avatares (casting por voz)
casting-scrfd-failed = o SCRFD não carregou: { $error }; sem avatares
casting-embedder-missing = { $model } não foi encontrado ({ $path }); avatares sem embedding
casting-embedder-failed = { $model } não carregou: { $error }; avatares sem embedding
casting-no-frame = o ffmpeg não extraiu o quadro
casting-voice-no-vocals = embedding de voz pulado: não há voz limpa
casting-voice-no-model = voz pulada: não há o modelo WeSpeaker ({ $path })
casting-voice-model-failed = o WeSpeaker não carregou: { $error }; voz pulada
casting-voice-trim-failed = amostra de voz { $speaker }: o corte falhou: { $error }
casting-voice-embedding-failed = voz { $speaker }: o embedding falhou: { $error }
casting-voice-embeddings = embeddings de voz: { $count }
casting-applying-profile = aplicando o perfil da biblioteca: { $slug }
casting-library-profile-missing = o perfil da biblioteca “{ $slug }” não foi encontrado; nada aplicado
record-busy = Já há uma gravação em andamento
record-mic-not-found = O microfone “{ $name }” não foi encontrado: está desconectado ou foi renomeado; escolha outro
record-no-mic = Nenhum microfone encontrado
record-mic-config = configuração do microfone: { $error }
record-create-wav = criando o wav: { $error }
record-format-unsupported = o formato { $format } não é suportado
record-mic-open = abrindo o microfone: { $error }
record-mic-start = iniciando o microfone: { $error }
record-not-recording = Nada está sendo gravado
record-silent = O microfone gravou só silêncio, por isso a voz não foi guardada: verifique se o sistema deixa os programas usar o microfone (Windows: Definições → Privacidade e segurança → Microfone), se ele não está sem som e se o microfone certo está escolhido.
record-not-levelled = A voz foi guardada como foi gravada, sem igualar o volume: { $error }. Os clones dela podem sair baixos; grave-a de novo mais perto do microfone ou iguale-a na ficha da voz.
voices-not-file-list = { $page }: não é uma lista de arquivos: { $error }
voices-list-endless = { $page }: a lista de arquivos do dataset não termina
voices-too-large = { $url }: mais do que os { $size } bytes fixados
voices-size-mismatch = { $url }: chegaram { $got } bytes, foram fixados { $size }
voices-fetch-failed = O download falhou ({ $tries } tentativas): { $error }. Verifique a rede e clique de novo: ele continua de onde parou.
cloud-no-model = { $provider } está escolhido, mas nenhum modelo de voz: escolha um nas configurações do projeto.
voices-sha-mismatch = { $url }: o SHA-256 { $got } não coincide com o fixado { $want }
voices-bad-name = nome inválido
voices-not-in-dataset = a voz { $name } não está no dataset { $dataset }
voices-bad-size = { $name }: o tamanho no catálogo não é válido
voices-no-sha = { $name }: não há SHA-256 no catálogo
voices-pack-downloading = baixando o pacote de vozes
voices-pack-unpacking = descompactando
voices-open-zip = abrindo o zip: { $error }
voices-create-file = criando { $name }: { $error }
voices-unpack-file = descompactando { $name }: { $error }
voices-pack-done = pronto: { $count } { $count ->
    [one] arquivo
   *[other] arquivos
}
voices-pack-taken = obtido de { $app }: { $count } { $count ->
    [one] arquivo
   *[other] arquivos
}

## Downloads by link

url-no-fetch = não existe o download { $id }
url-stopped = interrompido
url-already-downloading = este link já está sendo baixado: { $id }
url-probe-not-json = a resposta do yt-dlp -J não é JSON: { $error }; { $stderr }
url-interrupted = o download parou junto com o estúdio; o que foi baixado está na pasta dele e continuará dali
url-not-a-link = “{ $url }” não é um link: { $error }
url-not-http = “{ $url }” não é um link http(s)
url-bad-subs-lang = “{ $lang }” não é um código de idioma de legendas
url-cookies-not-file = { $path } não é um arquivo cookies.txt de até 1 MB
url-cookies-empty-or-large = cookies.txt está vazio ou passa de 1 MB
url-thread-failed = a thread de download não iniciou: { $error }
url-cookies-gone = o cookies.txt deste download sumiu da pasta; comece o download de novo com cookies
url-disk-space = são necessários cerca de { $need } MB, há { $free } MB livres
url-no-subs = o vídeo não tem legendas “{ $lang }” feitas por pessoas (há: { $have })
url-no-subs-none = o vídeo não tem legendas “{ $lang }” feitas por pessoas (não há nenhuma)
url-no-media-file = o yt-dlp terminou, mas não há arquivo de vídeo em { $path }: { $stderr }
url-subs-failed = as legendas “{ $lang }” não foram baixadas ({ $code }): { $error }
url-ytdlp-start = iniciando o yt-dlp: { $error }
url-ytdlp-wait = aguardando o yt-dlp: { $error }
url-still-downloading = { $id } ainda está sendo baixado
url-cancelled-removed = { $id } foi cancelado e o que foi baixado foi removido: comece um novo download
url-still-stopping = { $id } ainda está parando; tente de novo em alguns segundos
url-cancel-first = { $id } ainda está sendo baixado; cancele primeiro
url-no-subs-file = o yt-dlp terminou sem arquivo de legendas em { $path }: { $stderr }
url-no-parent = { $path } não tem pasta pai
url-subs-empty = as legendas { $path } não têm nenhuma fala
url-ytdlp-missing = o componente ytdlp não foi baixado
url-bad-quality = qualidade “{ $quality }”: best, 1080, 720, 480 ou audio

## yt-dlp

ytdlp-update-missing = a atualização do yt-dlp { $version } ({ $path }) não está no lugar; roda a fixada { $pinned }
ytdlp-http-client = cliente HTTP: { $error }
ytdlp-tag-not-version = o lançamento do yt-dlp com a tag “{ $tag }” não é uma versão
ytdlp-no-checksum = { $url } não contém { $file }
ytdlp-new-says = o novo yt-dlp { $tag } diz ser “{ $said }”; continua o { $current }
ytdlp-new-failed = o novo yt-dlp { $tag } não iniciou: { $error }; continua o { $current }
ytdlp-too-large = { $url }: mais de { $limit } bytes
ytdlp-sha-mismatch = { $url }: o SHA-256 { $got } não coincide com SHA2-256SUMS { $want }
ytdlp-exit-code = código de saída { $code }: { $stderr }
ytdlp-component-missing = o componente ytdlp (yt-dlp e deno) não foi baixado
ytdlp-no-ffmpeg = sem ffmpeg: nem o componente ffmpeg nem ffmpeg.exe no PATH
ytdlp-start = iniciando { $program }: { $error }
ytdlp-timeout = o yt-dlp não respondeu em { $seconds } s: { $stderr }
ytdlp-playlist = o link é uma playlist ou um canal ({ $count } { $count ->
    [one] vídeo
   *[other] vídeos
}), não um único vídeo
ytdlp-redirect-only = o link não tem o vídeo em si, só outro link
ytdlp-live = uma transmissão ao vivo ({ $status })
ytdlp-thumbnail-not-image = a miniatura não é uma imagem ({ $mime })
ytdlp-thumbnail-too-large = a miniatura passa de 4 MB
ytdlp-failed-silently = o yt-dlp falhou sem mensagem

## Render: voicing, QC, mix, mux

render-input = entrada { $width }x{ $height } dur={ $duration }s
render-nodub-original = nodub: a faixa de áudio original
render-done-audio = pronto (só áudio) -> { $path }
render-building-ass = montando o ASS (títulos + legendas dubladas)
render-burning = gravando as legendas + desfoque (ffmpeg + libass, NVENC)
render-burn-off = legendas/títulos desativados (subs.burn=off)
render-encode-cpu-mode = o estúdio roda no processador, então o vídeo é codificado no processador (x264), mais devagar que na placa de vídeo
render-encode-cpu-driver = o driver NVIDIA { $have } é anterior a { $need }, necessário para codificar na placa de vídeo, então o vídeo é codificado no processador (x264), mais devagar. Atualize o driver pelo site da NVIDIA para codificar na placa
render-muxing = multiplexando vídeo + áudio
render-track-dub = { $lang } (dublagem)
render-track-original = { $lang } (original)
render-two-tracks = duas faixas: { $dub } + { $original } -> { $container }
render-multitrack-failed = a multiplexação multifaixa falhou ({ $error }) -> uma faixa
render-subtitle-tracks = legendas como faixas mkv: { $tracks }
render-subtitle-tracks-failed = legendas como faixas mkv: { $error }
render-mp4-companion-failed = o mp4 complementar não foi montado ({ $error }); o player abrirá o mkv (o VLC funciona)
render-done = pronto -> { $path }
render-dub-audio-done = o áudio dublado está pronto
render-synth-thread-ended = a thread de síntese terminou sem resultado
render-synth-timeout = { $engine }: a síntese passou de { $seconds } s; cancelada, o motor está livre
render-engine-stuck = { $engine }: a síntese não cancela após { $seconds } s; a renderização parou (o motor travou na DLL)
render-higgs-gpu-old = A voz local Higgs não roda em { $card }: precisa de uma NVIDIA a partir da Turing (compute capability { $min }, GTX 16 e RTX 20 ou mais novas). Escolha a voz "Google · direto" ou "Pelo OpenRouter".
render-higgs-driver-old = A voz local Higgs precisa do driver NVIDIA { $min } ou mais novo, e este é mais antigo: { $card }. Atualize o driver ou escolha a voz "Google · direto" ou "Pelo OpenRouter".
render-higgs-no-gpu = A voz local Higgs precisa de uma placa NVIDIA com CUDA, e nenhuma foi encontrada ({ $card }). Escolha a voz "Google · direto" ou "Pelo OpenRouter".
render-higgs-model-missing = Higgs Audio v3 não foi baixado ({ $path }): baixe-o em Ajustes → Modelos → Voz.
render-defect-runaway = descontrolada
render-defect-cutoff = cortada
render-defect-silence = silêncio
render-defect-hum = zumbido
render-recognition-no-answer = o reconhecimento não retornou resposta
render-second-pass-overflow = a segunda passada de voz pediu um encurtamento, que nela está desativado
render-no-translated-lines = não há falas traduzidas -> silêncio, a faixa original
render-takes-of-removed-lines = históricos de tomadas de falas removidas apagados: { $count }
render-extracting-audio = extraindo o áudio (ffmpeg 44,1k estéreo)
render-separator-missing = o motor de separação não foi encontrado -> sem fundo (keep_music off)
render-ref-from-mix = a referência do clone vem da mixagem sem separação: o fundo original também soa nela
render-emotion-ref-failed = segmento { $segment }: a referência de emoção não foi cortada ({ $error }); usando a referência de identidade do falante
render-cloud-voices = vozes na nuvem por falante: { $voices }
render-synth-keys-reset = { $error }; as chaves de síntese recomeçam
render-synthesizing = sintetizando { $count } de { $total } { $total ->
    [one] segmento
   *[other] segmentos
}
render-voicing-cached = voz do cache: { $count } { $count ->
    [one] segmento
   *[other] segmentos
}
render-cloud-tts-parallel = TTS na nuvem: { $count } { $count ->
    [one] segmento
   *[other] segmentos
} em { $threads } threads paralelas
render-cloud-tts-ready = TTS na nuvem: pré-síntese pronta ({ $count } { $count ->
    [one] segmento
   *[other] segmentos
})
render-takes-quarantined = { $error }; o histórico de tomadas da fala { $line } foi separado em { $path } e recomeçou
render-pinned-take = fala { $line }: toca a tomada fixada; a nova voz não a substitui
render-take-unpinned = fala { $line }: a tomada foi desafixada porque o texto da fala mudou
render-selected-take = fala { $line }: toca a tomada escolhida { $take }; a nova voz não a substitui
render-write-cloud-segment = gravando o seg{ $line } da nuvem: { $error }
render-cloud-tts-failed = ⚠ segmento { $line }: o TTS na nuvem falhou ({ $error }); o original foi mantido
render-loading-higgs = carregando o Higgs
render-failures-kept-generated = ⚠ segmento { $line }: { $attempts } falhas de síntese ({ $error }); usada a voz gerada (amplitude { $range } dB)
render-failures-kept-original = ⚠ segmento { $line }: { $attempts } falhas/timeouts de síntese ({ $error }); a fala original foi mantida
render-regenerating = segmento { $line }: { $error }; regenerando ({ $attempt }/{ $attempts })
render-defects-kept-generated = ⚠ segmento { $line }: todas as { $attempts } tentativas têm defeito ({ $defect }); usada a voz gerada (amplitude { $range } dB)
render-silent-kept-original = ⚠ segmento { $line }: { $attempts } tentativas sem som; usado o original
render-retry-alt-ref = outra referência
render-retry-temperature = temperatura maior
render-defect-regenerating = segmento { $line }: defeito de síntese ({ $defect }); regenerando ({ $via } { $attempt }/{ $attempts })
render-write-segment = gravando seg{ $line }: { $error }
render-too-many-artifacts = TTS: artefatos de zumbido demais ({ $in_a_row } seguidos, { $retries } novas tentativas no total); regenerar não ajuda. O problema provavelmente é a máquina (modelo/VRAM) ou os clipes de referência de voz. Parado no segmento { $line }.
render-multi-take = segmento { $line }: várias tomadas; escolhida a mais próxima do espaço (desvio de { $deviation } s)
render-stretch-over-cap = segmento { $line }: precisa esticar x{ $needed } (espaço { $slot } s), limite x{ $cap }; o texto está mais rápido que o normal
render-lead-into-silence = começar no silêncio: { $lines } falas começam antes, { $seconds } s no total, em vez de serem aceleradas
render-silence-trimmed = corte de silêncio do TTS: { $seconds } s removidos de { $lines } { $lines ->
    [one] fala
   *[other] falas
} ({ $pauses } s de pausas); graças ao corte a aceleração ficou dentro do limite em { $into_cap } { $into_cap ->
    [one] fala
   *[other] falas
}
render-fit-summary = ajuste: { $over }/{ $total } segmentos acima do limite ({ $share }%)
render-fit-summary-drift = ajuste: { $over }/{ $total } segmentos acima do limite ({ $share }%), sincronia recuperada em { $drift }
render-qc-start = QC: conferindo { $count } { $count ->
    [one] fala
   *[other] falas
} por transcrição
render-qc-unheard = QC: { $count } de { $total } falas não foram conferidas; o reconhecimento falhou: { $reason }
render-qc-mismatch = QC: { $count } { $count ->
    [one] fala não coincide
   *[other] falas não coincidem
} com a tradução; ressintetizando
render-qc-resynthesized = QC: segmento { $line } ressintetizado (tentativa { $attempt })
render-qc-unconfirmed = ⚠ QC: não foi possível confirmar o segmento { $line } (“{ $text }”); confira a fala manualmente
render-qc-kept-mismatch = ⚠ QC: o segmento { $line } não coincide com o texto traduzido; mantida a voz gerada
render-qc-resynth-unheard = QC: { $count } { $count ->
    [one] fala ressintetizada não foi conferida
   *[other] falas ressintetizadas não foram conferidas
}; o reconhecimento falhou: { $reason }
render-qc-summary = resultado do QC: { $fixed }/{ $total } corrigidas, { $flagged } ainda marcadas, { $unheard } não conferidas
render-qc-all-confirmed = QC: todas as falas foram confirmadas por transcrição ✓
render-qc-rest-confirmed = QC: as demais falas foram confirmadas por transcrição
render-laying-out = posicionando a dublagem na linha do tempo
render-peak-limiter = limitador de picos: { $lines } { $lines ->
    [one] fala
   *[other] falas
}, { $samples } amostras acima do teto { $ceiling } reduzidas sem clipping
render-tempo-fit = ajuste de andamento da faixa inteira x{ $factor }
render-voiceover-envelope = voice-over: o original a { $db } dB SOB a tradução, cheio nas pausas (envelope dinâmico, { $blocks } blocos)
render-voiceover-flat = voice-over: o envelope está indisponível -> atenuação plana
render-mix-no-ducking = mixagem: instrumental + voz dublada (ducking DESLIGADO, fundo cheio)
render-mix-ducking = mixagem: instrumental + voz dublada (ducking LIGADO, envelope, { $blocks } blocos)
render-mix-sidechain = o envelope está indisponível -> ducking por sidechain
render-mix-plain = o sidechain está indisponível -> mixagem direta
render-loudness-off = o nivelamento de volume está desligado: a mixagem como está
render-loudness-normalizing = normalizando o volume (EBU R128, true peak)
render-loudnorm-skipped = loudnorm pulado ({ $error })
render-track-gain = ganho da faixa { $db } dB
render-dub-timings-not-written = os tempos da dublagem para as legendas não foram gravados: o posicionamento ({ $lines } falas, { $spans } trechos) não coincidiu com os segmentos ({ $segments }); as legendas seguem os tempos originais
render-dub-timing-mismatch = tempos da dublagem: a fala { $line } da visão de síntese não coincide com o segmento nº { $index } do projeto
render-word-timings = tempos por palavra das legendas: reconhecendo { $count } { $count ->
    [one] fala
   *[other] falas
} da dublagem
render-refs-unchecked = conferência de referências: { $count } { $count ->
    [one] candidata aceita
   *[other] candidatas aceitas
} sem conferência; o reconhecimento falhou: { $error }
render-refs-all-failed = ⚠ falante { $speaker }: nenhuma referência candidata passou na conferência (ouvido: “{ $heard }”); usando a melhor pela pontuação
render-speaker-ref-failed = referência do falante { $speaker }: { $error }
render-speaker-ref = referência do falante { $speaker }: “{ $text }” ({ $seconds } s, { $candidates } { $candidates ->
    [one] candidata
   *[other] candidatas
}, conferência ok)
render-speaker-ref-unchecked = referência do falante { $speaker }: “{ $text }” ({ $seconds } s, { $candidates } { $candidates ->
    [one] candidata
   *[other] candidatas
}, conferência ⚠ falhou)

## Setup: components

setup-comp-higgs-purpose = Dá voz ao texto, clona uma voz a partir de uma amostra curta e transmite emoção
setup-comp-higgs-engine-name = Motor de voz e reconhecimento (audio.cpp)
setup-comp-higgs-engine-purpose = O programa que executa a narração Higgs e o reconhecimento CrisperWhisper e Nemotron 3.5
setup-comp-audiocpp-build-name = Motor audio.cpp: versão { $build }
setup-comp-audiocpp-build-purpose = O motor compilado para outro dispositivo: baixado quando a narração ou o reconhecimento são definidos para ele em Definições → Modelos
setup-comp-gemma-purpose = O diretor: escreve um podcast sobre um tema e distribui vozes aos personagens de um livro
setup-comp-gemma-q5-0-purpose = O assistente de roteiro, mais preciso que o q4_0
setup-comp-gemma-q6-k-purpose = O assistente de roteiro, ainda mais preciso
setup-comp-gemma-q8-0-purpose = O assistente de roteiro, o mais preciso e o mais pesado
setup-comp-parakeet-purpose = Reconhece a fala com o tempo de cada palavra: verifica os takes de ouvido e transcreve gravações
setup-comp-higgs-q6-k-purpose = Voz: variante mais leve que Q8_0
setup-comp-higgs-q4-k-m-purpose = Voz: a variante mais leve
setup-comp-sortformer-name = Quem está falando (até 8 vozes)
setup-comp-sortformer-purpose = Distingue quem fala numa gravação
setup-comp-roformer-purpose = Separa a voz da música: limpa a amostra de voz e a gravação antes do reconhecimento
setup-comp-roformer-q5-purpose = Separação de voz: variante mais leve que Q8_0
setup-comp-roformer-q4-purpose = Separação de voz: a variante mais leve
setup-comp-wespeaker-name = Semelhança de voz (WeSpeaker)
setup-comp-wespeaker-purpose = Mede o quanto uma locução se parece com a sua voz de referência, para verificar a locução
setup-comp-voice-gate-name = Porta de voz (ReDimNet2-B3)
setup-comp-voice-gate-purpose = Reconhece a sua própria voz para que uma conversa no Live comece só quando você fala
setup-comp-wake-word-name = Palavra de ativação (PhoneticXeus)
setup-comp-wake-word-purpose = Reconhecedor universal de fonemas: o Live ativa com uma frase que você definir, em qualquer idioma
setup-comp-ruaccent-purpose = Marca a sílaba tônica e a letra ё em textos russos e distingue homógrafos pelo contexto
setup-comp-ruaccent-omograph-big-name = Modelo grande de homógrafos do RUAccent
setup-comp-ruaccent-omograph-big-purpose = O modelo big_poetry do RUAccent em int8 para ler os homógrafos pelo sentido (замо́к/за́мок, мука́/му́ка): acerta mais que o tiny2 embutido (29 de 33 contra 24 nas frases de teste); depois de baixado, é usado no lugar dele
setup-comp-silero-vad-name = Silero VAD v6.2
setup-comp-silero-vad-purpose = Encontra onde há fala numa gravação, para cortar uma longa nas pausas e não no meio da palavra no reconhecimento (1,2 MB)
setup-comp-langid-purpose = Detecta o idioma de uma gravação em «Auto» para que um modelo que o conhece a reconheça (107 idiomas)
setup-comp-dpdfnet-name = Supressão de ruído DPDFNet
setup-comp-dpdfnet-purpose = Remove o ruído das referências de voz e das gravações, com controle de intensidade para a fala não soar estéril (10,5 MB)
denoise-missing = O modelo de supressão de ruído DPDFNet não foi baixado: baixe-o em Configurações → Modelos.
setup-comp-bsroformer-engine-name = Motor de separação de voz (NVIDIA)
setup-comp-bsroformer-engine-purpose = O programa que separa a voz na placa de vídeo
setup-comp-bsroformer-engine-cpu-name = Motor de separação de voz (processador)
setup-comp-bsroformer-engine-cpu-purpose = Para um computador sem NVIDIA: roda no processador, mais devagar
setup-comp-llama-name = Servidor do assistente (llama.cpp)
setup-comp-llama-purpose = O programa que roda o Gemma para o assistente de roteiro
setup-llama-vulkan-glibc = O assistente usa a placa pelo Vulkan: a versão CUDA do llama.cpp precisa da glibc { $need } ou mais nova, este sistema tem a { $glibc }
setup-comp-onnxruntime-purpose = Necessário para reconhecer a fala e os falantes
setup-comp-onnxruntime-gpu-purpose = Para que o reconhecimento de fala e de pessoas rode na placa de vídeo, não no processador
setup-comp-ffmpeg-purpose = Lê e grava áudio e vídeo
setup-comp-ytdlp-name = Download por link (yt-dlp + deno)
setup-comp-ytdlp-purpose = Buscar uma gravação por link (YouTube e os outros sites do yt-dlp)
setup-comp-cuda-runtime-purpose = Bibliotecas da NVIDIA que os motores usam na placa de vídeo; não é preciso instalar o CUDA à parte
setup-comp-cudnn-purpose = Necessário para o reconhecimento de fala e de pessoas na placa de vídeo
setup-comp-vcruntime-purpose = Bibliotecas do sistema Windows para os motores (incluídas)
setup-comp-nvidia-driver-name = Driver NVIDIA
setup-comp-nvidia-driver-purpose = Necessário para usar a placa de vídeo; é instalado pelo site da NVIDIA, não pelo app

## Setup: downloads and installation

setup-http-status = { $url }: status { $status }
setup-write = gravando: { $error }
setup-not-zip = não é um zip: { $error }
setup-zip-entry = entrada do zip: { $error }
setup-open = abrindo { $path }: { $error }
setup-verifying = Conferindo o SHA-256 de { $file }…
setup-install-record = o registro de instalação: { $error }
setup-disk-space = espaço insuficiente: são necessários { $need } GB, há { $free } GB livres ({ $path })
setup-unknown-component = não existe o componente { $id }
setup-not-removable = { $id } não é instalado pelo aplicativo
setup-component-busy = o componente está sendo baixado; pause o download
setup-paused = o download foi pausado
setup-rate-limited = { $url }: o servidor responde { $status } há { $minutes } min
setup-proxy-scheme = proxy { $proxy }: o esquema { $scheme } não serve para downloads (http, https, socks4, socks5)
setup-proxy-no-host = proxy { $proxy }: sem host
setup-proxy-no-port = proxy { $proxy }: sem porta
setup-proxy-credentials = proxy { $proxy }: o download de modelos (ureq) não consegue passar ao proxy esse login ou senha: há / ? #, espaço, caracteres não ASCII ou (no SOCKS5) dois-pontos na senha; as requisições à nuvem por esse proxy funcionam, para baixar é preciso uma senha sem esses caracteres
setup-start-failed = { $url }: não foi possível começar em { $retries } tentativas: { $error }
setup-chunk-manifest = o manifesto de blocos: { $error }
setup-range-incomplete = intervalo incompleto: { $got }/{ $want } bytes
setup-range-failed = intervalo { $start }-{ $end } após { $retries } tentativas: { $error }
setup-range-status = intervalo { $start }-{ $end }: status { $status } (esperado 206)
setup-range-read = lendo o intervalo: { $error }
setup-create = criando { $path }: { $error }
setup-read = lendo: { $error }
setup-download-failed = { $url } após { $retries } tentativas: { $error }
setup-size-mismatch = { $file }: baixados { $got } bytes, fixados { $want }; o arquivo foi removido e a próxima tentativa recomeça
setup-hash-mismatch = { $file }: o SHA-256 { $got } não coincide com o fixado { $want }; o arquivo foi removido e a próxima tentativa recomeça
setup-unpacking = Descompactando { $file }…
setup-rename = renomeando { $path }: { $error }
setup-waiting-other = Aguardando outro download dos mesmos componentes…
setup-delete = excluindo { $path }: { $error }
setup-downloading = Baixando os modelos…
setup-taken-from = { $file } aproveitado de { $app } sem baixar
setup-source-changed = { $url }: o servidor entrega { $got } bytes, fixados { $want }; a fonte mudou
setup-manifest-write = manifesto { $path }: { $error }
setup-archive-no-files = o arquivo { $path } não contém os arquivos necessários
setup-wheel-not-zip = o wheel não é um zip: { $error }
setup-wheel-entry = entrada do wheel: { $error }
setup-archive-no-dll = o arquivo { $path } não contém bibliotecas
setup-unpack = descompactando { $path }: { $error }
setup-finalize = finalizando { $path }: { $error }

## Settings applied after analysis

post-analyze-bad-vo-gain = vo_gain: esperava-se um número de dB, chegou { $value }
post-analyze-bad-flag = { $name }: esperava-se 0 ou 1, chegou { $value }
post-analyze-bad-container = container: esperava-se mp4 ou mkv, chegou { $value }
post-analyze-bad-voice-slots = voice_slots: esperava-se um objeto {"{"}male:[…], female:[…]{"}"}
post-analyze-edit-failed = o ajuste após a análise { $edit }: { $error }

## Messages of the engines and libraries: glossary, LLM, ASR, translation, separation, faces, TTS, captions, OCR

glossary-over-limit = o glossário tem { $total } entradas, mais que { $max }
glossary-empty-term = a entrada { $entry } não tem termo
glossary-field-too-long = entrada { $entry } («{ $term }»): um campo passa de { $max } caracteres
glossary-duplicate = o termo «{ $term }» aparece duas vezes (entradas { $first } e { $second })
glossary-tsv-keep = linha { $line }: keep é «{ $value }»; esperado 1 ou 0
glossary-tsv-empty-term = linha { $line }: falta o termo
glossary-one-of = envie entradas ou TSV, não ambos
glossary-nothing = nada para salvar: não há entradas nem TSV
llm-spawn-failed = o llama-server não iniciou: { $error }
llm-gguf-missing = o modelo GGUF não foi encontrado ({ $path })
llm-log-file = log do llama-server { $path }: { $error }
llm-exited-early = o llama-server terminou antes de ficar pronto ({ $status }); stderr: { $stderr }
llm-not-ready = o llama-server não subiu em { $secs } s (porta { $port }); stderr: { $stderr }
llm-no-gpu-device = a Gemma local precisa de uma placa de vídeo, e a versão do llama.cpp não vê nenhuma ({ $detail }). Escolha um servidor local, o OpenRouter ou o agente para o assistente de roteiro em Ajustes → Modelos, ou verifique o driver da placa.
llm-http = a solicitação ao modelo falhou: { $error }
llm-api = API do modelo: { $error }
llm-rejected = a API do modelo recusou a solicitação ({ $status }): { $body }
llm-empty-answer = o modelo { $model } devolveu uma resposta vazia (sem motivo)
llm-empty-answer-reason = o modelo { $model } devolveu uma resposta vazia (finish_reason={ $reason })
llm-cut-short = o modelo { $model } atingiu o limite de { $max_tokens } { $max_tokens ->
    [one] token
   *[other] tokens
} (finish_reason=length): a resposta está incompleta; provavelmente gasta o orçamento raciocinando — escolha um modelo sem raciocínio obrigatório
llm-prompt-cut = o servidor leu só { $read } { $read ->
    [one] token
   *[other] tokens
} de uma solicitação de { $chars } { $chars ->
    [one] caractere
   *[other] caracteres
} e descartou o início — o contexto dele é pequeno: aumente num_ctx no Ollama ou o Context Length do modelo no LM Studio
asr-engine = reconhecimento de fala: { $error }
asr-wav-read = não foi possível ler o wav { $path }: { $error }
asr-resample = reamostragem: { $error }
asr-speaker-count = o número de locutores deve ser de 1 a { $max }
speakers-embedding = voz do locutor { $speaker }: { $error }
speakers-no-sample = locutor { $speaker }: não há trecho de fala de pelo menos 0,3 segundo para comparar a voz
speakers-too-many-voices = número de locutores indicado: { $given }, mas o trecho tem { $found } { $found ->
    [one] voz diferente
   *[other] vozes diferentes
}: não foi possível juntá-las com segurança pela voz
speakers-bad-embedding = o modelo de voz devolveu características de voz vazias ou inválidas
speakers-dimension-changed = o tamanho das características de voz mudou
speakers-more-voices = o trecho tem mais vozes que o número de locutores indicado
speakers-unmatched = não foi possível associar com segurança as vozes do trecho ao número de locutores indicado
translate-format-json = tradução: o modelo responde com um objeto JSON pelo esquema ({ $model })
translate-format-json-probe = tradução: tentando respostas por esquema JSON ({ $model }); se recusar, linhas numeradas
translate-format-numbered = tradução: linhas numeradas; o modelo { $model } não declara structured_outputs no catálogo do OpenRouter
translate-schema-ignored = tradução: { $model } aceitou o esquema JSON, mas respondeu com linhas numeradas; daqui em diante, linhas numeradas
translate-schema-ignored-server = tradução: o servidor aceitou o esquema JSON, mas respondeu com linhas numeradas; daqui em diante, linhas numeradas
translate-schema-refused = tradução: { $model } recusou respostas por esquema JSON ({ $status }: { $body }); daqui em diante, linhas numeradas
translate-schema-refused-server = tradução: o servidor recusou respostas por esquema JSON ({ $status }: { $body }); daqui em diante, linhas numeradas
translate-line-flawed = tradução: a linha { $line } mantém uma tradução com uma observação: { $reason }
translate-line-failed = tradução: a linha { $line } não foi traduzida: { $reason }
translate-line-reason = { $line }: { $reason }
translate-lines-rejected = tradução: { $bad } de { $total } { $total ->
    [one] linha
   *[other] linhas
} não passaram na verificação ({ $reasons })
translate-batch-stopped = tradução: o bloco de linhas { $first }..{ $last } falhou ({ $error }); a tradução parou
translate-batch-failed = tradução: o bloco de linhas { $first }..{ $last } falhou ({ $error })
translate-layout-no-vision = layout do quadro: ignorado (nenhum modelo de visão escolhido ou disponível)
translate-layout-not-needed = layout do quadro: ignorado (as legendas não são gravadas no vídeo, não é necessário)
translate-layout = layout do quadro: sub_style={ $sub_style } titles={ $titles } brands={ $brands }
translate-layout-failed = layout do quadro: ignorado ({ $error })
translate-scene-failed = contexto da cena: ignorado ({ $error })
translate-scene-no-vision = contexto da cena: ignorado (nenhum modelo de visão escolhido ou disponível)
translate-audio-failed = contexto de áudio: ignorado ({ $error })
translate-context-trimmed = tradução: o bloco de contexto de { $chars } { $chars ->
    [one] caractere
   *[other] caracteres
} foi cortado para { $budget } (proteção de n_ctx)
translate-names-skipped = tradução: o glossário automático de nomes foi ignorado ({ $error })
translate-chunks = tradução: { $lines } { $lines ->
    [one] linha
   *[other] linhas
} -> { $chunks } { $chunks ->
    [one] bloco
   *[other] blocos
} (glossário: { $terms } { $terms ->
    [one] termo
   *[other] termos
}, { $names } { $names ->
    [one] nome
   *[other] nomes
})
translate-pass-done = tradução: pronto, { $translated } traduzidas ({ $flawed } com observação), { $untranslated } no idioma original
glossary-pass = glossário: passagem do modelo { $pass }/{ $passes }
glossary-schema-refused = glossário: o servidor recusou respostas por esquema JSON ({ $status }); pedindo JSON como texto
content-type-decided = tipo de conteúdo: { $decided ->
    [anime] animação
   *[other] imagem real
} ({ $votes } { $votes ->
    [one] resposta clara
   *[other] respostas claras
} de { $frames } quadros)
line-missing = ausente da resposta
line-cut = a resposta foi cortada pelo limite de tokens
line-untranslated = não está no idioma de destino
line-echo = repete o original
line-too-short = curta demais ({ $got } < { $min })
line-too-long = longa demais ({ $got } > { $max })
line-loop = repete em loop «{ $gram }»
line-term-missing = falta o termo do glossário «{ $term }»
line-gender = o falante fala de si no outro gênero: «{ $form }»
line-latin = a palavra original «{ $word }» ficou sem tradução
translate-frame = extração de um quadro: { $error }
translate-audio = contexto de áudio: { $error }
translate-empty = o modelo não traduziu nenhuma de { $lines } { $lines ->
    [one] linha
   *[other] linhas
}; a última causa: { $reason }
translate-empty-no-reason = o modelo não traduziu nenhuma de { $lines } { $lines ->
    [one] linha
   *[other] linhas
}
translate-contract = a resposta do modelo: { $problem }
translate-contract-answer = a resposta do modelo: { $problem }; a resposta: { $answer }
answer-no-json-object = não contém um objeto JSON
answer-not-json = não é um JSON válido ({ $error })
answer-no-terms = não contém a lista terms
remix-failed = remix: { $error }
sep-engine-missing = o motor de separação não foi encontrado ({ $path })
sep-model-missing = o modelo de separação não foi encontrado ({ $path })
sep-spawn = início do motor de separação: { $error }
sep-engine-failed = o motor de separação falhou (código { $code }): { $tail }
sep-fell-back-to-cpu = a versão da separação de voz para a placa de vídeo não encontrou a placa e calculou no processador ({ $why }). Escolha «CPU» ou «Auto» para a limpeza de voz em Ajustes → Modelos ou verifique o driver da placa.
sep-engine-killed = o motor de separação parou sem código de saída: { $tail }
sep-no-output = o motor de separação não criou a faixa de voz ({ $path })
sep-audio-io = áudio da separação: { $error }
faces-model-missing = o modelo não foi encontrado ({ $path })
ffmpeg-exit = o ffmpeg terminou com o código { $code }: { $tail }
ffmpeg-killed = o ffmpeg parou sem código de saída: { $tail }
onnx-runtime = o ambiente ONNX: { $error }
onnx-runtime-load = o ambiente ONNX { $file } não carregou ({ $error }). Quase sempre um Microsoft Visual C++ antigo o impede: instale o Visual C++ 2015–2022 Redistributable (x64) atual da Microsoft e reinicie o estúdio
files-error = arquivos: { $error }
model-no-outputs = o modelo não devolveu saídas
faces-output-shape = { $model }: a saída do modelo tem uma forma inesperada { $shape }
faces-output-count = { $model }: eram esperadas { $expected } saídas do modelo, vieram { $got }
faces-crop-size = o embedding do rosto espera um recorte alinhado de 112x112, veio { $width }x{ $height }
faces-sample-rate = o clipe de voz deve ter { $expected } Hz, tem { $got } Hz
faces-clip-too-short = o clipe de voz é curto demais para analisar
captions-write-ass = gravação do arquivo de legendas: { $error }
captions-filter-script = gravação do script de filtros do ffmpeg: { $error }
captions-ffmpeg-wait = espera do ffmpeg: { $error }
captions-ffmpeg-timeout = o ffmpeg não terminou em { $secs } s e foi interrompido (travou): { $tail }
captions-burn-failed = a gravação das legendas no vídeo falhou: { $tail }
captions-nvenc-driver-old = o driver da NVIDIA é antigo demais para codificar vídeo na placa de vídeo (NVENC): é preciso a versão { $need } ou mais nova. Atualize o driver pelo site da NVIDIA e renderize de novo
captions-frame-failed = o quadro de pré-visualização falhou: { $tail }
ocr-no-dictionary = o modelo de reconhecimento de texto não tem dicionário de caracteres
ocr-internal = reconhecimento de texto: erro interno ({ $error })
update-title = Atualização do Speech Studio
quit-title = Fechar o Speech Studio?
quit-running = O estúdio ainda está trabalhando: { $n } tarefa(s) na fila ou em andamento. Fechar agora as interrompe; as tomadas já feitas estão salvas e o resto pode ser locutado depois com “Locutar o alterado”.
quit-button-close = Fechar
quit-button-stay = Não fechar
update-open-page = A versão { $version } já está disponível. Abrir a página de download?
update-install-now = A versão { $version } já está disponível. Atualizar agora? O aplicativo será reiniciado.
update-button-open = Abrir
update-button-install = Atualizar
update-button-later = Mais tarde
update-open-failed = Não foi possível abrir { $url }: { $error }
update-failed = A atualização não foi instalada: { $error }

## Voice library
voice-missing = Não há voz chamada { $name }.
voice-transcribing = Reconhecendo o texto de referência de { $name }…
voice-no-speech = Nenhuma fala reconhecida em { $name }: a gravação pode estar em silêncio ou ser só música.
voice-clean-missing = A limpeza da voz precisa do motor de separação de voz e do seu modelo: baixe-os em Configurações → Modelos → Limpeza de voz, ou use a voz sem limpeza.
voice-cleaning = Limpando { $name } de música e ruído…
voice-denoising = Removendo o ruído de { $name }…
voice-design-missing = O modelo Qwen3-TTS VoiceDesign não foi baixado: baixe-o em Configurações → Modelos.
voice-designing = Criando uma voz a partir da descrição: { $n } de { $total }
voice-design-gone = Esses candidatos não existem mais: crie a voz de novo.
voice-design-empty = Descreva a voz e escreva um texto de amostra.
voice-design-name = Voz criada
setup-comp-qwen3-tts-design-name = Qwen3-TTS VoiceDesign (criação de voz)
setup-comp-qwen3-tts-design-purpose = Uma voz a partir de uma descrição — gênero, idade, timbre, jeito: vários candidatos, o melhor guardado na biblioteca como referência
voice-bad-audio = { $name } não é um arquivo de som aceito pelo estúdio: WAV, MP3, FLAC, M4A ou OGG.
voice-default-name = Minha voz
voice-lang-needs-whisper = { $name } está num idioma que o Parakeet não conhece ({ $lang }): o texto é reconhecido pelo CrisperWhisper. Baixe o CrisperWhisper em Configurações → Modelos → Reconhecimento de fala.
voice-no-words = { $name } ainda não tem tempos de palavras: reconheça primeiro o texto de referência.

## Speech
tts-defective = A voz saiu com defeito em todas as tentativas ({ $defects }): tente outra voz, um texto mais curto ou uma temperatura menor.
speech-block-silent = Não há nada a dizer neste bloco: só pausas e marcações.
speech-synth-block = Locutando { $n } de { $total }: { $text }
speech-synth-all-failed = Nenhum bloco foi locutado: { $error }
speech-export-not-ready = { $count } blocos impedem a exportação (sem locução, desatualizados, com o arquivo da tomada perdido ou alterado, ou mais longos que a fala): a verificação antes da exportação os aponta.
speech-export-empty = Não há nada para exportar: todos os blocos estão vazios ou ocultos.
speech-export-block = Juntando { $n } de { $total }…
speech-export-loudness = Ajustando o volume ao predefinido…
speech-name-template-same = O modelo de nome “{ $template }” dá o mesmo nome aos { $count } arquivos: adicione {"{"}n{"}"} ou {"{"}chapter{"}"}
speech-export-stems = Faixas por personagem: { $count }
speech-by-speaker = por personagem
speech-untitled = Sem título
speech-narrator = Narrador
speech-lines = Falas
speech-no-project = O projeto { $pid } não existe.
media-loudnorm-silent = O áudio está em silêncio: não há volume para ajustar.
media-bad-format = { $format } não é um formato de exportação: WAV, FLAC, MP3, OGG ou M4A.
speech-qc-block = Verificando de ouvido { $n } de { $total }…
speech-qc-failed = A verificação não passou: reconhecido como «{ $heard }». Ouça, escolha outra tomada ou locute de novo.
stress-model-missing = O modelo de acentuação russa ainda não foi baixado: obtenha o RUAccent em Modelos
stress-failed = Não foi possível marcar a acentuação: { $detail }
dict-bad-id = Uma entrada do dicionário tem id vazio ou repetido: { $id }
dict-bad-kind = Tipo de entrada desconhecido: { $kind }
dict-empty-find = Uma entrada do dicionário não indica o que procurar
dict-bad-pattern = O padrão de uma entrada do dicionário tem um erro: { $detail }
dict-import-unknown = { $name } não é um dicionário do Balabolka (.dic, .rex ou .bxd)
speech-engine-unknown = Motor de síntese desconhecido: { $engine }
speech-lang-unsupported = { $engine } não fala { $lang }: escolha outro idioma ou Automático nas configurações do projeto
speech-no-voice-blocks = Estes blocos não têm voz, e o motor deles só fala com uma voz da biblioteca: { $blocks }. Escolha uma voz para o projeto, o locutor ou o bloco.
speech-no-voice-library-empty = Estes blocos não têm voz, e o motor deles só fala com uma voz da biblioteca, que ainda não tem vozes: { $blocks }. Adicione uma voz na aba Vozes (de um arquivo, uma gravação ou um pacote) ou escolha um motor com vozes próprias.
speech-unknown-voice-blocks = Estes blocos têm uma voz que não está na biblioteca nem é uma voz própria do motor deles (muitas vezes uma voz integrada de outro motor): { $blocks }. Escolha uma voz para o projeto, o locutor ou o bloco.
speechproj-not-a-package = Este não é um arquivo de projeto do Speech Studio (.speechproj)
speechproj-bad-path = O arquivo de projeto tem um caminho fora da pasta: { $path }
speechproj-bad-manifest = A descrição do arquivo de projeto está danificada: { $detail }
speechproj-too-new = O arquivo de projeto foi feito por um Speech Studio mais novo (formato { $version }, versão { $app }): atualize o estúdio
speechproj-engine-missing = O projeto precisa do motor { $engine }, que este estúdio não tem
speechproj-asset-missing = Falta { $path } no arquivo de projeto
speechproj-asset-damaged = { $path } no arquivo de projeto está danificado (tamanho ou SHA-256 não conferem)
speech-subs-empty = Nenhuma fala com tempo foi encontrada em { $name } (são lidos SRT, VTT e ASS)
book-unknown-format = { $name }: livros são lidos de EPUB, FB2, PDF, DOCX, HTML, MD e TXT
book-empty = Nenhum texto foi encontrado em { $name }
book-damaged = O arquivo do livro está danificado ou não é o que o nome diz: { $detail }
speech-chapter-n = Capítulo { $n }
export-save-as = Salvar como
speech-qc-unlike = A voz da tomada não parece a referência (semelhança { $similarity }, precisa 0,5 ou mais): narre de novo ou verifique a referência
listen-speaker-n = Falante { $n }
listen-no-source = o projeto não tem gravação
listen-whisper-missing = o CrisperWhisper foi escolhido, mas não está instalado: baixe-o em Configurações → Modelos
listen-live-downloading = baixando o reconhecedor; o reconhecimento começa assim que terminar
say-hushed = a pessoa parou a fala na janela: espere a próxima frase dela antes de continuar
live-metrics-bad-range = Não é um número de linha: { $value }
say-no-floor = Agora ninguém fala por cima do estúdio: não há depois do quê continuar
echo-guard-no-owner = A proteção contra eco pela sua voz precisa da sua voz gravada (Configurações, filtro de voz): até lá o estúdio evita ouvir a si mesmo só pelo volume e pelas próprias palavras
voice-gate-stale = Sua voz foi gravada por uma versão anterior e não é usada: grave-a de novo (Configurações, filtro de voz). Até lá o filtro de voz deixa passar todas as frases
say-mic-off = o microfone está desligado: a pessoa o liga no Live e então voice_listen a ouve
mic-only-person = só a pessoa liga o microfone, pelo botão no estúdio
listen-gguf-missing = O modelo de reconhecimento «{ $engine }» não foi baixado: baixe-o com o motor transcribe.cpp em Configurações → Modelos.
listen-gguf-lang = O modelo «{ $engine }» não reconhece «{ $lang }»: escolha outro modelo ou o CrisperWhisper.
listen-separate-missing = separar a voz requer o motor e o modelo Roformer: baixe-os em Configurações → Modelos
listen-diar-missing = encontrar os falantes requer o modelo de diarização: baixe-o em Configurações → Modelos ou indique um único falante
listen-step-audio = Preparando o áudio
listen-step-separate = Separando a voz da música e do ruído
listen-step-diarize = Encontrando os falantes
listen-step-asr = Reconhecendo a fala
listen-step-langid = Idioma da gravação: { $lang }
listen-step-asr-piece = Reconhecendo a fala: trecho { $n } de { $total }
listen-step-denoise = Removendo o ruído
listen-no-speech = Nenhuma fala encontrada na gravação: se houver fala baixa, reduza o limiar de fala (agora { $threshold }) em Reconhecimento → Corte nas pausas.
listen-step-filter = Verificando alucinações do reconhecedor
listen-step-done = Pronto: { $lines } linhas
listen-nothing-marked = nenhuma palavra está marcada para remoção
listen-cut-everything = todas as palavras estão marcadas: nada sobraria
listen-step-cut = Cortando { $n } trechos
listen-cut-name = { $name } (editado)
listen-step-cut-done = A gravação editada está pronta
listen-export-format = formato de transcrição desconhecido { $format }
listen-not-media = { $file } não é um arquivo de áudio nem de vídeo
listen-not-recording = nada está sendo gravado
listen-mic-name = Gravação { $time }
listen-mic-origin = microfone
listen-empty = a transcrição não tem linhas
listen-transcribe-first = reconheça a gravação primeiro
listen-align-no-text = Cole o texto que a gravação deveria dizer.
drama-needs-cuda = A voz Drama 3 precisa de uma placa NVIDIA com CUDA, e nenhuma foi encontrada ({ $card }). Escolha Higgs.
drama-needs-vram = O Drama 3 precisa de uma placa de vídeo de 6 GB ou mais, e esta tem { $gb } GB ({ $card }). Escolha Higgs.
drama-needs-ram = Numa placa com menos de 24 GB, o Drama 3 mantém os especialistas (18 GB) na RAM: precisa de 32 GB de RAM, e este computador tem { $gb } GB. Escolha Higgs.
render-drama-missing = Drama 3 está escolhida mas não foi baixada por completo: baixe-a em Modelos (voz) ou escolha Higgs.
render-drama-load-failed = carregando Drama 3: { $error }
render-drama-vram-limit-low = O limite de memória de vídeo do Drama 3 ({ $gb } GB em Definições → Modelos) não deixa espaço para o seu cache de especialistas nesta máquina: defina-o como Automático ou aumente-o ({ $error })
render-drama-vram-busy = O Drama 3 não encontrou espaço para o seu cache de especialistas: outros programas ocupam a memória da placa gráfica. Feche-os e volte a narrar ({ $error })
render-loading-drama = carregando Drama 3
render-drama-no-ref-text = a referência de voz não tem transcrição, e Drama 3 não clona uma voz sem ela
drama-voice-no-text = A voz «{ $name }» não tem texto de referência, e Drama 3 não clona uma voz sem ele: abra a voz na «Biblioteca de vozes» e escreva o «Texto de referência» ou carregue em «Reconhecer o texto de referência».
drama-voice-no-piece = A voz «{ $name }» dura mais de { $secs } s, e não se encontrou nela um trecho de até { $secs } s com palavras inteiras: abra a voz na «Biblioteca de vozes», carregue em «Encontrar os melhores trechos» e use um, ou corte a referência.
setup-comp-drama3-purpose = Fish Audio Drama 3: um segundo motor de voz com clonagem expressiva; placas de vídeo a partir de 6 GB, com 32 GB de RAM em placas com menos de 24 GB
drama-needs-voice = O Drama 3 clona uma voz a partir da referência: escolha uma voz para o bloco ou o projeto
setup-comp-parakeet-ultra-purpose = Reconhecimento de fala: versão ajustada, com menos erros
setup-comp-transcribe-engine-purpose = O motor de reconhecimento transcribe.cpp na GPU: Parakeet Ultra, a deteção do idioma e o Nemotron 3.5 numa conversa ao vivo
setup-comp-transcribe-engine-cpu-purpose = O motor de reconhecimento transcribe.cpp no processador, para um computador sem NVIDIA: Parakeet Ultra, a deteção do idioma e o Nemotron 3.5 numa conversa ao vivo, mais lento
setup-comp-transcribe-engine-vulkan-purpose = O motor de reconhecimento transcribe.cpp em Vulkan, para uma placa AMD ou Intel e uma NVIDIA onde o CUDA 13 não funciona: Parakeet Ultra, a deteção do idioma e o Nemotron 3.5 numa conversa ao vivo
setup-comp-asr-gguf-nemotron-purpose = Nemotron 3.5: 28 idiomas com pontuação, entre eles russo, chinês e japonês (NVIDIA)
render-defect-clipping = clipping
render-defect-click = um clique no final
director-failed = o modelo diretor não respondeu: { $error }
director-not-json = o modelo diretor não respondeu na forma combinada: { $answer }
director-empty = o modelo diretor não escreveu nenhuma fala
director-not-verbatim = o modelo diretor alterou duas vezes o texto do livro no trecho que começa com «{ $start }»; marque-o à mão
director-need-topic = são necessários um tema e pelo menos dois falantes com nome
director-starting = Iniciando o modelo diretor
director-writing = { $model } escreve o roteiro
director-casting = Marcando falantes: parte { $n } de { $total }
director-done = Pronto
cloud-catalog-empty = o provedor não listou nenhum modelo de voz
cloud-unknown-model = { $provider } não tem o modelo { $model } no catálogo
cloud-no-schema = o modelo { $model } não tem descrição de formulário
cloud-unknown-provider = { $provider } não é um provedor de nuvem aqui (fal, runware)
cloud-limit-unpriced = o limite diário de nuvem para agentes é { $limit } USD e esta execução não tem preço antes de rodar
cloud-limit-reached = o limite diário de nuvem para agentes foi atingido: { $used } de { $limit } USD
cloud-no-key = { $provider } foi escolhido, mas não tem chave: adicione-a em Configurações, Nuvem
cloud-failed = { $provider }: { $error }
cloud-no-audio = o modelo na nuvem respondeu sem áudio
cloud-no-text-field = o modelo { $model } não aceita texto
cloud-no-audio-field = o modelo { $model } não aceita um arquivo de áudio
cloud-key-one-line = a chave deve ser uma linha não vazia
cloud-no-model-chosen = escolha um modelo de { $provider } nas configurações do projeto
google-no-voice = escolha uma voz do Google Gemini nas configurações do projeto
google-batch-sending = Google Batch: enviando { $n } frases em um único lote
google-batch-waiting = Google Batch: { $state }, { $done } de { $total } frases prontas
google-batch-dead = o lote do Google Batch { $name } terminou { $state } sem resultados ({ $error }); ao locutar de novo, as mesmas frases vão num novo lote
google-batch-unknown = o lote do Google Batch { $name } foi enviado, mas a resposta se perdeu e o Google ainda não o lista; locute de novo mais tarde (registro { $journal })
google-batch-many = { $n } lotes do Google têm o nome { $name }; resolva-os no Google AI Studio (registro { $journal })
google-batch-queue-full = a fila do Google Batch deste modelo está cheia, nada foi criado; tente de novo quando o lote em andamento terminar: { $error }
google-batch-rejected = o Google recusou o lote: nada foi criado nem pago, é seguro tentar de novo: { $error }
google-batch-no-row = o resultado do lote não tem linha para esta frase
google-retry = Google: { $what } falhou (tentativa { $attempt } de { $of }), de novo em { $secs } s: { $error }
google-read-failed = Google: { $what } falhou { $attempts } vezes seguidas: { $error }
google-phrase-failed = o Google não locutou esta frase: { $error }
google-direct-sending = Google Gemini: { $n } frases, { $threads } pedidos de cada vez
google-direct-progress = Google Gemini: { $done } de { $total } frases
google-err-prepayment = o pré-pagamento da API Google Gemini acabou: recarregue o projeto da chave no Google AI Studio e inicie a locução de novo
google-err-cap = o projeto da chave Google atingiu o limite de gastos: aumente-o no Google AI Studio ou aguarde o próximo mês
google-err-key = o Google não aceitou a chave: confira-a em Configurações → Nuvem
google-err-access = a chave Google não tem acesso a este modelo: confira o projeto da chave no Google AI Studio ou escolha outro modelo
google-err-rate = o Google limitou a frequência de pedidos: reduza os pedidos simultâneos nos ajustes ou tente mais tarde
google-err-filter = o Google recusou-se a locutar esta frase pelas regras de conteúdo: reformule-a ou locute-a com outro motor
chat-empty = escreva uma mensagem ao assistente
chat-no-model = escolha o modelo do assistente
chat-too-many-turns = o assistente deu { $turns } passos e parou; diga se deve continuar
assistant-no-agent = o assistente é o agente conectado, mas nenhum está conectado agora: conecte um (Configurações, Agente) ou escolha um modelo do OpenRouter como assistente
assistant-no-answer = o agente não respondeu em { $minutes } min
dict-try-local-only = o teste de ouvido funciona com os motores deste computador (Higgs, Drama 3); o motor na nuvem do projeto é pago a cada execução
dict-try-empty = digite uma frase para testar
dict-try-before = Teste de ouvido: como está escrito
dict-try-after = Teste de ouvido: com a regra
speech-part-n = parte { $n }
speech-export-encoding = Codificando a parte { $n } de { $total }

render-supertonic-missing = O Supertonic 3 está escolhido mas não baixado: baixe-o em Modelos (narração) ou escolha outro motor.
render-supertonic-load-failed = O Supertonic 3 não carregou: { $error }
render-audio8-missing = O Audio8 TTS está escolhido mas não baixado: baixe-o em Modelos (narração) ou escolha outro motor.
render-audio8-load-failed = O Audio8 TTS não carregou: { $error }
render-audio8-model-missing = Audio8 TTS não foi baixado ({ $path }): baixe-o em Ajustes → Modelos → Voz.
setup-comp-audio8-purpose = Voz multilíngue compacta com clonagem zero-shot; um motor de narração alternativo.
speech-voice-not-preset = A voz «{ $voice }» é da biblioteca e o { $engine } só fala com as suas próprias vozes: escolha uma delas para o projeto, o falante ou o bloco.
speech-voice-preset-needed = Este motor precisa de uma das suas próprias vozes.
setup-comp-supertonic3-purpose = Supertonic 3: narração rápida com dez vozes prontas e 31 idiomas, sem clonagem
render-qwen-missing = O Qwen3-TTS está escolhido mas não baixado por completo (o motor ou os pesos da quantização escolhida): baixe-o em Modelos (narração) ou escolha outro motor.
render-qwen-load-failed = O Qwen3-TTS não carregou: { $error }
setup-comp-qwen3-tts-purpose = Qwen3-TTS 1.7B: nove vozes prontas (CustomVoice) e clonagem de vozes da biblioteca (Base), dez idiomas com russo, áudio em streaming
render-qwen-clone-missing = A clonagem de voz no Qwen3-TTS usa o modelo Base da quantização escolhida e faltam os seus arquivos: baixe o Qwen3-TTS de novo em Modelos ou escolha uma voz do próprio modelo.
render-qwen-stress-missing = O Qwen3-TTS com acentuação russa está escolhido, mas não foi baixado por inteiro (o motor, o modelo Base da quantização escolhida ou o adaptador de acentuação): baixe-o em Modelos (locução) ou escolha outro motor.
render-qwen-stress-needs-voice = O Qwen3-TTS com acentuação russa só clona uma voz: escolha uma voz da biblioteca para o bloco ou o projeto.
setup-comp-qwen3-tts-ru-stress-name = Acentuação russa para o Qwen3-TTS
setup-comp-qwen3-tts-ru-stress-purpose = Um adaptador LoRA para o Qwen3-TTS Base (siriusfreak): a voz clonada segue as marcas de acento que o estúdio coloca, 96% das palavras contra 17% sem ele
setup-comp-qwen3-tts-engine-name = Motor do Qwen3-TTS
setup-comp-qwen3-tts-engine-purpose = O programa e o codec de áudio que todas as quantizações do Qwen3-TTS partilham
project-missing = o projeto { $pid } não existe mais: foi excluído ou ainda não foi criado
media-bad-rate = { $rate } Hz não serve para { $format }
media-bad-bits = { $bits } bits não serve para { $format } (WAV: 16, 24, 32; FLAC: 16, 24)
media-bad-kbps = { $kbps } kbps não serve para { $format } (MP3, OGG, M4A: de 64 a 320)
speech-own-ref-missing = falta o arquivo de referência próprio { $file } na pasta refs do projeto: envie-o de novo
speech-own-ref-no-file = o arquivo de referência não chegou
speech-no-block = o projeto não tem o bloco { $id }
director-no-tags = { $engine } não lê tags de expressão: não há nada a inserir
director-enriching = O diretor insere tags: parte { $n } de { $total }
speaker-too-little = o orador «{ $name }» fala muito pouco para memorizar a voz: são precisos pelo menos 2 segundos
book-name-note = um nome do livro: ponha a marca de acento + e marque ao conferir de ouvido
director-punctuating = O diretor pontua: parte { $n } de { $total }
director-rewriting = O diretor reescreve: parte { $n } de { $total }
director-no-instruction = Diga como reescrever os blocos: mais curto, mais formal, com outra piada.
director-nothing-written = O projeto ainda não tem texto: o diretor continua um roteiro já começado.
tracks-empty-folder = A pasta { $path } não tem arquivos de som
tracks-not-audio = { $path } não é um arquivo de som nem uma pasta
tracks-no-track = O projeto não tem a faixa { $track }
tracks-stem = faixa
tracks-no-files = Nenhum arquivo de som foi escolhido
music-no-job = { $studio } não iniciou nenhuma tarefa de música
music-no-song = { $studio } não entregou nenhuma música
music-unknown = Não há estúdio de música { $studio }
music-asking = Enviando para { $studio }…
music-making = { $studio } está fazendo a música: { $detail }
music-placing = Colocando a música na faixa…
music-track = Música
listen-export-rules = Não há regras de legenda “{ $rules }”; há: { $known }
itn-no-lang = Números em algarismos não são conhecidos para { $lang }: só ru, en, es, fr, pt, de, it, nl, da
listen-whisper-for-lang = O Parakeet não conhece { $lang }: precisa do CrisperWhisper, que não está instalado; baixe-o em Configurações → Modelos ou escolha um reconhecedor na nuvem
listen-live-name = o reconhecimento ao vivo
listen-live-no-vad = O reconhecimento ao vivo precisa do Silero VAD: baixe-o em Configurações → Modelos
listen-live-broke = O reconhecimento ao vivo parou: a gravação foi guardada, reconheça-a como de costume
watch-no-folder = A pasta { $folder } não existe
watch-no-formats = Escolha pelo menos um formato
watch-bad-format = Não há formato { $format }; há: { $known }
watch-broke = O trabalho com a pasta parou: inicie-o de novo, nada feito se perde
watch-busy = A gravação já está sendo reconhecida por outra tarefa ({ $kind }); será tentada na próxima volta
speech-corpus-name = Frases difíceis — teste do motor
speech-corpus-unknown = Não existe o conjunto de frases { $corpus }. Só existe ru_hard.
speech-qc-defects = Defeito de som: { $list }. Ouça, escolha outra tomada ou locute de novo.
qc-issue-click = um clique no fim
qc-issue-clipping = saturação
qc-issue-cutoff = cortada
qc-issue-gap = um corte no meio
qc-issue-garbled = confusão em vez do texto
qc-issue-hum = zumbido
qc-issue-noise = música ou ruído em vez de fala
qc-issue-repeat = frase repetida
qc-issue-runaway = o modelo não parou
qc-issue-silence = silêncio
qc-issue-stretched = uma palavra arrastada
qc-issue-tail = som depois da última palavra
qc-issue-whistle = assobio
speech-ab-two = Uma comparação às cegas exige exatamente dois motores.
speech-ab-engine = O motor { $engine } não serve para comparar: é preciso um motor local conhecido (os da nuvem custam dinheiro).
speech-ab-side = Locutando a variante { $n } de 2…
speech-fix-no-take = O bloco não tem tomada para consertar: locute-o primeiro.
speech-fix-sentence = Locutando de novo a frase { $n }…
speech-restore-gone = O projeto não está na lixeira: ela foi esvaziada ou o projeto foi apagado em outro lugar.
speech-restore-finder = No macOS, o projeto apagado é recuperado da Lixeira no Finder: selecione-o lá e escolha «Colocar de Volta».
speech-restore-exists = O projeto já está no lugar.
speak-aloud-no-window = Não há onde dizer em voz alta: a janela do Speech Studio não está aberta. Abra-a, e o som sai do estúdio enquanto a frase é falada.
say-session-name = Conversa { $time }
talk-widget-title = Speech Studio Live
say-no-session = Não há sessão de fala “{ $session }”: comece uma nova.
listen-live-not-running = O reconhecimento ao vivo não está ativo: ligue primeiro o microfone no Live.
listen-live-engine = O reconhecedor «{ $engine }» não reconhece ao vivo: use parakeet, nemotron ou whisper.
say-no-text = O texto está vazio: escreva o que dizer ao agente.
record-test-other = Há outra gravação em curso, não um teste de microfone.
record-test-none = Ainda não houve teste de microfone.
voice-gate-too-short = A gravação é curta demais para aprender a sua voz — repita a frase.
voice-gate-not-heard = Não consegui distinguir a sua voz — diga a frase com clareza e tente de novo.
setup-comp-crisperwhisper-purpose = Reconhecimento de fala em 99 idiomas com marcas de tempo, além dos 25 europeus do Parakeet. Licença: nyra health Non-Commercial (LICENSE.md junto aos pesos)
acpp-no-server = O motor audio.cpp não está instalado ({ $path }): baixe-o em Definições → Modelos
acpp-no-libraries = O motor audio.cpp não pode iniciar: faltam as suas bibliotecas CUDA ({ $files }). Baixe-as em Definições → Modelos
acpp-no-cuda-build = A placa gráfica (CUDA) está escolhida, mas este computador não tem uma placa NVIDIA em que o motor audio.cpp funcione: precisa de Maxwell ou mais recente com o driver 525 ou posterior. Escolha Vulkan ou o processador em Definições → Modelos
acpp-no-model = O modelo «{ $model }» do motor audio.cpp não foi baixado: baixe-o em Definições → Modelos
acpp-error = motor audio.cpp: { $error }
acpp-out-of-memory = O motor audio.cpp ficou sem memória de vídeo: escolha uma quantização do Higgs mais leve em Definições → Modelos, encurte o bloco ou feche os programas que usam a placa ({ $error })
acpp-no-device = Este computador não tem o dispositivo escolhido para o motor audio.cpp: escolha outro dispositivo para a narração ou o reconhecimento em Definições → Modelos ({ $error })
update-reason-transcribe-own-ggml = O reconhecimento já não falha após a locução com Qwen3-TTS: as bibliotecas do motor têm nomes próprios
update-reason-newer = Uma versão mais recente
bench-no-timings = Ainda não há medições: tenha primeiro uma conversa no Live e meça depois.
bench-set-empty = { $path } não tem frases.
bench-no-asr-set = A medição do reconhecimento precisa do seu conjunto: { $path } — uma lista de {"{"}"audio": ficheiro, "text": o que diz, "lang": o idioma{"}"}.
bench-kind = A medição não tem o tipo «{ $kind }»: latency, floor ou asr.
role-not-object = Um papel é escrito como um objeto com campos: pelo menos um nome.
role-no-name = O papel não tem nome: dê um nome a ele.
role-bad-id = «{ $id }» não pode ser o identificador de um papel: até 40 letras latinas minúsculas, algarismos, _ e -.
role-unknown = Não há nenhum papel «{ $id }» na pasta de papéis.
role-none = Nenhum papel foi escolhido: escolha um primeiro no Live.
role-review-no-lines = Neste papel ainda não se ouviu nada da pessoa: tenha primeiro a conversa e depois peça a análise.

<div align="center">

<img src="frontend/public/favicon.svg" width="72" alt="Logo de Speech Studio"/>

# Speech Studio

**Estudio de voz con IA gratuito y sin conexión para Windows, macOS (Apple silicon) y Linux: pone voz a cualquier texto con voces clonadas e integradas y convierte grabaciones en texto con sus hablantes. El ruso, bien hecho: números en su caso, acentos, «ё» y homógrafos. 100% local, cero Python: una aplicación nativa (Rust + C++ sobre CUDA, Vulkan o Metal); cada modelo y cada motor se descargan con un botón.**

[![License](https://img.shields.io/github/license/timoncool/speech-studio?style=flat-square)](LICENSE)
[![Stars](https://img.shields.io/github/stars/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/stargazers)
[![Latest release](https://img.shields.io/github/v/release/timoncool/speech-studio?include_prereleases&style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Downloads](https://img.shields.io/github/downloads/timoncool/speech-studio/total?style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Last commit](https://img.shields.io/github/last-commit/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/commits)

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh.md) · **Español** · [Português](README.pt.md) · [Français](README.fr.md)

![Speech Studio: un diálogo con una voz para cada personaje](docs/screenshots/es-01-speech.webp)

</div>

## Qué es

**Speech Studio** tiene tres modos. **Speech** pone voz a un texto: una frase, un diálogo con una voz por personaje, un pódcast que te escribe el director o un libro entero desde EPUB, FB2, PDF o DOCX. **Listening** convierte una grabación, un vídeo o un enlace en texto con quién dijo qué y cuándo, lo edita por sus palabras, comprueba una locución contra el texto que debía decir y crea voces de la biblioteca a partir de sus hablantes. **Live** es una conversación por voz con un agente de IA conectado por MCP: dice cada frase en cuanto la escribe y te oye por el micrófono.

Todo funciona **en tu equipo**: sin nube ni suscripción; tus textos, grabaciones y huellas de voz no salen del ordenador. Si quieres una voz que tu tarjeta no puede ejecutar, también están las voces en la nube de **OpenRouter**, **Google Gemini**, **fal.ai** y **Runware**, cada una con su clave y una confirmación antes de la primera llamada; vienen desactivadas.

Es una **aplicación totalmente nativa**: sin Python embebido, sin torch, sin wheels de CUDA. Todo el estudio es **Rust + motores nativos C++ (GGUF/ONNX) sobre CUDA, Vulkan o Metal**. Los modelos, los motores, las bibliotecas de CUDA y VC++ y ffmpeg **los descarga la propia aplicación** en el primer inicio, y un modelo que ya tiene otro estudio del autor se toma de él en lugar de descargarlo otra vez. **La aplicación está hecha y probada para tarjetas NVIDIA**; para lo demás, mira *Qué se ejecuta dónde*.

## Para agentes de IA

Mientras Speech Studio está abierto, sirve MCP en `http://127.0.0.1:8797/mcp`: un agente como Claude Code, Claude Desktop, Cursor o Codex hace todo lo que hace la ventana con el mismo código: pone voz a un texto en un archivo con una sola llamada, crea y edita proyectos bloque a bloque, asigna voces a personajes, diseña una voz a partir de una descripción, transcribe y comprueba grabaciones y exporta audiolibros con capítulos y subtítulos. En Ajustes, **Agente (MCP)** muestra si hay un agente conectado y qué pegar en el cliente. Lo único que un agente no puede hacer es encender el micrófono: Live, una grabación y la prueba del micrófono solo empiezan con tu propio clic o tecla.

1. Instala el estudio desde la [última versión](https://github.com/timoncool/speech-studio/releases/latest) y ábrelo.
2. Conéctate a su servidor MCP:
   ```bash
   claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp
   ```
   Otros clientes: `{ "mcpServers": { "speech-studio": { "type": "streamable-http", "url": "http://127.0.0.1:8797/mcp" } } }`
3. Lee la habilidad que sirve el servidor (recurso `studio://skill`), el mismo texto que [docs/mcp-skill.md](docs/mcp-skill.md), y empieza por la herramienta `studio_status`.

En el mismo puerto el estudio habla la API de audio de OpenAI —`POST /v1/audio/speech`, `POST /v1/audio/transcriptions`, `GET /v1/models`—, así que las herramientas hechas para OpenAI locutan y transcriben a través de él, con el nombre de una voz de la biblioteca como `voice`. [llms.txt](llms.txt) dice lo mismo para las herramientas que lo buscan. Solo pueden conectarse los programas de este equipo.

## Qué puedes hacer con él

- **Un audiolibro** — suelta un EPUB o FB2: capítulos, personajes con sus voces, un M4B con capítulos y portada, volumen ACX comprobado.
- **Un pódcast o un diálogo** — el director escribe un pódcast sobre un tema para dos o tres voces, o pega líneas como «Ana: …»; la línea de tiempo muestra cada solapamiento.
- **Una locución para un vídeo** — pon voz a un guion o a subtítulos dentro de sus tiempos, añade música y sonidos en el editor de pistas y exporta al volumen del vídeo.
- **La transcripción de una entrevista o una clase** — quién dijo qué y cuándo, tiempos de cada palabra, subtítulos según las normas de las cadenas, la grabación editada por su texto.
- **La revisión de un doblaje** — superpón el guion a la grabación y verás cada palabra dicha mal u omitida.
- **Tu propia biblioteca de voces** — graba o importa voces, límpialas y crea voces a partir de los hablantes de una grabación.
- **Una voz para tu agente de IA** — habla con Claude Code o cualquier agente MCP en Live, o deja que locute y transcriba archivos por MCP o por la API compatible con OpenAI.

Ejemplos de todos los motores y voces, con los acentos rusos antes y después, en la [página del proyecto](https://timoncool.github.io/speech-studio/es.html).

## Tres modos

| Modo | Qué hace |
|------|--------------|
| 🔊 **Speech** | Texto, diálogo, pódcast o libro → bloques locutados con tomas, revisados de oído y exportados en WAV, FLAC, MP3, OGG, M4A o M4B con capítulos, subtítulos y texto sincronizado |
| 👂 **Listening** | Audio, vídeo o enlace → una transcripción con hablantes y tiempos por palabra, un reproductor karaoke, edición por texto, exportación a SRT, VTT, ASS, TXT o JSON y comparación con un texto de referencia |
| 🎙️ **Live** | Una conversación por voz con un agente de IA: el agente conectado por MCP dice cada frase en cuanto la escribe, usted responde al micrófono y se reconoce frase a frase, una frase dicha encima lo detiene; solo usted enciende el micrófono, pausa con M o Ctrl+Mayús+Espacio, un widget sobre todas las ventanas |

## Funciones

**Voces y motores**

- **Seis motores locales**: [Higgs Audio v3](https://huggingface.co/bosonai) (clonación, emociones y sonidos), **Fish Audio Drama 3** (la voz abierta más potente; entera en una tarjeta de 24 GB, desde 6 GB con sus expertos en la RAM), **Audio8 TTS** (habla natural en 23 idiomas, clonación o la voz propia del modelo), **Supertonic 3** (rápido, diez voces integradas), **Qwen3-TTS 1.7B** (nueve voces integradas, clonación, una instrucción por bloque) y **Qwen3-TTS con acento ruso**, la LoRA de siriusfreak aplicada al vuelo.
- **Motor por personaje y por bloque** sobre el del proyecto; un motor que no puede hablar con la voz elegida dice qué voz propia usará.
- **Biblioteca de voces**: un paquete de voces con un clic, 743 voces rusas del catálogo en la nube, grabación con micrófono, importación de archivos y ZIP, foto, género, edad, idioma, etiquetas y consentimiento; la transcripción de la referencia se reconoce una vez y se guarda.
- **Limpieza de referencias**: Mel-Band Roformer separa la voz de la música y **DPDFNet** quita el ruido con un control de intensidad para que la voz siga viva; el original se conserva.
- **El mejor fragmento de la referencia**: ventanas en las pausas entre palabras reconocidas, cada una con su duración, relación señal/ruido, saturación y pausa más larga.
- **Voz por descripción**: Qwen3-TTS VoiceDesign crea una voz a partir de «una anciana con voz suave y algo ronca…»: escucha varios candidatos y guarda uno en la biblioteca.

- **Voces en la nube** — OpenRouter, Google Gemini (peticiones en paralelo o Batch a mitad de precio), fal.ai y Runware, cada una con su clave y una confirmación antes de la primera llamada.

**Texto que suena bien**

- **Normalizador del ruso**: números en el caso y género correctos, fechas, horas, dinero, unidades, números romanos, marcas latinas y siglas.
- **Acentos, «ё» y homógrafos** con RUAccent: el modelo grande de homógrafos (big_poetry en int8, las mismas elecciones que fp32) elige por la frase; los dudosos se resaltan para ti y tus `+` se respetan.
- **Diccionario de pronunciación**: sustitución, regex y fonemas, con ámbito de proyecto, idioma, voz o motor, importación de Balabolka `.rex`/`.dic`/`.bxd` y prueba de oído desde el propio diccionario.
- **Marcado compatible con SpeechKit**: acento `+`, pausas `sil<[300]>` y `[pause 500ms]`, tempo `[rate:1.2]`, `**énfasis**`; lo que un motor ignora se marca, nunca se pierde en silencio.

- **Aún más texto bien leído** — un subconjunto de SSML, pistas de prosodia en color palabra por palabra, teléfonos, correos y códigos, un fragmento en otro idioma con la misma voz y una revisión del guion antes de locutar.

**Proyectos**

- **El texto como línea de tiempo**: bloques con hablantes, tomas (hasta cinco), control de obsoletos, deshacer y rehacer, arrastrar para reordenar, buscar y reemplazar con regex, notas y marcadores.
- **Control de calidad**: cada toma se reconoce de vuelta y se compara palabra a palabra, además de comprobar saturación, silencio, corte, chasquido final, frase repetida, cola larga, música o ruido en vez de voz y parecido con la voz de referencia; varias tomas con la mejor elegida.
- **Diálogos y pódcasts**: las líneas `Ana:` y `[Ana]` se vuelven personajes, el director escribe un pódcast sobre un tema para N hablantes, continúa un guion dentro de un proyecto abierto o reescribe las líneas seleccionadas, las pausas negativas crean interrupciones y la **línea de tiempo** muestra cada solapamiento con cortar, unir o mantener.
- **Audiolibros**: EPUB, FB2, PDF, DOCX, HTML, Markdown y TXT con vista previa de capítulos, reparto por el director, M4B con capítulos y portada, partes para libros largos y comprobación de volumen ACX.
- **Subtítulos como proyecto**: las líneas de SRT, VTT y ASS se locutan en su tiempo: el tempo se sube de antemano, una toma que no cabe se repite y después se acelera y se toma prestado el silencio previo.
- **Comprobación antes de exportar**: se listan los bloques sin audio, obsoletos, fallidos, con el archivo cambiado o más largos que su réplica; solo la duración puede dejarse pasar.
- **Volumen**: preajustes de vídeo (−14 LUFS), pódcast (−16 LUFS) y ACX con limitador true-peak, room tone y respiraciones entre bloques.
- **Archivo de proyecto**: `.speechproj` con texto, voces y audio lleva un proyecto a otro equipo.

- **Montaje** — un editor de pistas bajo el texto: mover, recortar, fundidos y volumen de cada clip, pistas de música y sonidos desde archivos o carpetas, o una canción hecha allí mismo en YuE2, MiniMax Music3 o ACE-Step Studio.
- **Exportación para cada uso** — un archivo por capítulo, una pista por personaje en un ZIP, una tabla CSV de réplicas, efectos de voz (radio, teléfono, megáfono), volumen parejo de los fragmentos, frecuencia y profundidad de bits, plantillas de nombres, un manifiesto SHA-256 y una nota de cómo se hizo.
- **Comparar y aprobar** — A/B de tomas, una comparación a ciegas de motores sobre el mismo texto, bloques aprobados que una nueva locución no toca, un semáforo de calidad y un informe del proyecto.
- **Libros y obras** — el estilo directo aparte del narrador, un reparto de personajes con voces elegidas para ellos, notas al pie limpiadas, los nombres del libro al diccionario, un proyecto actualizado desde un libro corregido sin perder lo ya locutado y un reproductor HTML del libro.

**Listening**

- **Parakeet-TDT v3, Nemotron 3.5 (28 idiomas con puntuación) y CrisperWhisper 2.0 turbo (99 idiomas con tiempos de palabra) en audio.cpp, Parakeet Ultra en transcribe.cpp**, hasta ocho hablantes con Nemotron 3 Diarization, una grabación larga cortada en las pausas por **Silero VAD v6**, ruido quitado por DPDFNet y música por Roformer.
- **Una pista para CrisperWhisper**: tema y grafías, más las correcciones que añadas.
- **Lote**: suelta varios archivos a la vez; cada uno pasa a ser una grabación en la cola de reconocimiento.
- **Un texto conocido sobre la grabación**: el texto de referencia toma el tiempo de cada palabra oída, frase a frase.
- **Voces desde hablantes**: una voz de biblioteca con las palabras más limpias de cada hablante.

- **Mientras hablas** — reconocimiento desde el micrófono frase a frase, una carpeta vigilada reconocida en segundo plano, reconocimiento rápido sin el editor.
- **El modelo adecuado por sí solo** — con el idioma en Auto, VoxLingua107 lo detecta y la grabación va a un reconocedor que lo conoce; el modelo elegido se descarga la primera vez que se usa.
- **Subtítulos según las normas de las cadenas** (Netflix, BBC y otras), quién habla cuánto, números en cifras al exportar.

**Live**

- **Una conversación por voz** — el agente conectado por MCP dice cada frase en cuanto la escribe; respondes al micrófono y se te reconoce frase a frase; una frase de dos palabras o más dicha encima del agente lo detiene, mientras que una tos, una risa o un «ajá» no; una línea escrita le llega como si la hubieras dicho.
- **Cambios al vuelo** — la voz del agente, el reconocedor, el idioma y el micrófono se cambian en mitad de la conversación, por ti o por el agente cuando se lo pides: el reconocedor cambia sin reiniciar nada, un micrófono o un idioma nuevos reinician el micrófono y, mientras carga el motor de la nueva voz, la ventana dice «Cargando la voz…».
- **Un ritmo natural** — una frase del agente se dice entera hasta 300 caracteres, una pausa que el motor deja dentro de una frase dura como mucho 0,6 s y cada frase suena una sola vez.
- **Solo tú enciendes el micrófono** — Live, una grabación en Listening, una grabación de voz y la prueba del micrófono solo empiezan con tu propio clic o tecla; un agente no tiene ninguna herramienta que encienda el micrófono, y la ventana no acepta sus clics en esos botones. La barra superior muestra «Micrófono encendido», para qué y desde hace cuánto, con una cruz que lo apaga; si no hay ninguna ventana del estudio abierta durante cinco segundos, el micrófono se apaga solo.
- **Micrófono y sonido** — pausa con M o Ctrl+Mayús+Espacio desde cualquier ventana (el agente calla y no te oye; mantener para hablar), la cruz termina la conversación y lo que el agente está diciendo, medidores de nivel, escucharte a ti mismo, prueba del altavoz y volumen del agente.
- **Roles: un entrenador para cualquier conversación** — elija a quién interpreta el agente: un entrevistador, un inversor, un cliente que quiere descuento, un escéptico, un jefe, un colega difícil, el público, un compañero de idioma, un oponente de debate. Un rol es una idea lista, no un marco: el agente puede cambiarlo, y entre los dos inventan los suyos hablando — una conversación difícil, un ejercicio de dicción o de idioma, una improvisación, una escena de teatro — y los guardan como archivo en la carpeta `roles`. Tras la conversación, el análisis del agente da su ritmo, pausas, muletillas y lo rápido que respondió, tres frases suyas, tres cosas que cambiar y un objetivo, comparado con su última vez en el mismo rol.
- **Cinco aspectos y un widget** — aura, onda, barras, radial y matriz, y un pequeño widget sobre todas las ventanas; Ctrl+3 abre Live.
- **Medido** — tu frase es texto 0,64–0,74 s después de que dejas de hablar (Parakeet en una RTX 4090), y el primer sonido de la respuesta llega entre 15 ms y 1 s después de que el agente la escribe, según el motor.

**En todas partes**

- **API compatible con OpenAI**, **MCP** con control de la ventana para agentes y un panel de asistente con un modelo de OpenRouter y las mismas herramientas.
- **Diagnóstico**: tarjeta gráfica, controlador, CUDA, bibliotecas, registros y un paquete para informes de errores sin claves.
- **Toda la aplicación en seis idiomas**, tema claro y oscuro, actualización automática y totalmente portátil.
- **El primer arranque pregunta solo lo que quieres** — voces, reconocimiento y extras; los motores, las bibliotecas y la versión para tu tarjeta llegan con ellos, una línea cada uno con su tamaño.
- **Paleta de comandos** (Ctrl+K), atajos propios, un modo simple y una pista «?» en cada control.

## Velocidad medida

En un mismo ordenador: RTX 4090 (24 GB) con Windows 11, Speech Studio tal como se instala.

**Locución** — Speech Studio 0.2.0, el mismo texto ruso de 17 segundos, la segunda ejecución de cada motor (modelo cargado); Higgs, en el motor audio.cpp el 22.10, tres ejecuciones, primer sonido sin medir; Audio8, el 09.11, un texto ruso de 18 segundos con voz clonada: entrega la frase entera, así que su primer sonido llega en 2,3 s en ese texto y en 0,4–1,0 s en una frase dicha en Live; el primer sonido es el tiempo hasta que empieza el audio en la ventana, la memoria de vídeo el pico mientras habla.

| Motor | Más rápido que el tiempo real | Primer sonido | Streaming | Descarga | Memoria de vídeo |
|---|---|---|---|---|---|
| Supertonic 3 | 168× | 103 ms | por frase | 0,1 GB | 1,0 GB |
| Qwen3-TTS | 12× | 19 ms | mientras habla | 2,2 GB | 3,1 GB |
| Qwen3-TTS + ru stress LoRA | 10× | 26 ms | mientras habla | 2,3 GB | 3,5 GB |
| Fish Audio Drama 3 | 7,0× | 326 ms | mientras habla | 21,4 GB | 21,9 GB |
| Audio8 TTS | 5,8× | 2,3 s | por frase | 1,5 GB | 2,0 GB |
| Higgs Audio v3 | 3,4× | — | mientras habla | 5,3 GB | ~7 GB |

**Reconocimiento** — Speech Studio 0.2.1 (CrisperWhisper, en el motor audio.cpp el 22.10: una grabación rusa de 625,8 s, con la carga del motor; en el procesador y su memoria de vídeo sin medir), una grabación rusa de 5 minutos de una sola voz (299,7 s) en Listening sin separar hablantes, con la carga del modelo incluida; el modelo libera la memoria de vídeo al terminar.

| Modelo | En la tarjeta | En el procesador | Idiomas | Descarga | Memoria de vídeo |
|---|---|---|---|---|---|
| Parakeet TDT v3 | 96× (3,1 s) | 33× (9,2 s) | 25 | 0,7 GB | 1,2 GB |
| Parakeet Ultra | 73× (4,1 s) | 33× (9,2 s) | 25 | 0,7 GB | 1,0 GB |
| Nemotron 3.5 | 73× (4,1 s) | 29× (10,3 s) | 28 | 0,7 GB | 1,0 GB |
| CrisperWhisper 2.0 turbo | 96× (6,5 s) | — | 99 | 1,6 GB | — |

**Live** — un turno de conversación.

| Paso | Tiempo |
|---|---|
| Tu frase como texto después de que dejas de hablar (Parakeet en la tarjeta, incluida la pausa que cierra la frase) | 0,64–0,74 s |
| El agente escribe su respuesta (Claude Sonnet) | 2,0–3,9 s |
| Primer sonido de la respuesta (según el motor, ver Locución) | 15 ms – 1,0 s |

## Capturas

La pantalla de inicio: un texto, un libro, subtítulos o un guion de pódcast se convierten en proyecto:

![Pantalla de inicio de Speech Studio](docs/screenshots/es-08-home.webp)

Listening: transcripción con hablantes y tiempos por palabra, reproductor karaoke, edición por texto y exportación:

![Listening en Speech Studio](docs/screenshots/es-06-listening.webp)

Live: una conversación por voz con un agente de IA: lo que dijiste y lo que responde, cinco aspectos, pausa y fin de la conversación:

![Live en Speech Studio](docs/screenshots/es-11-live.webp)

## Requisitos

- **SO:** Windows 10 / 11 (x64); macOS 13 o posterior en Apple silicon y Linux x86-64 como compilaciones experimentales
- **GPU:** NVIDIA con 8 GB de VRAM o más. Las tarjetas desde Turing (GTX 16, RTX 20 y posteriores) con controlador 580 o posterior usan las compilaciones CUDA 13; GTX 900, GTX 10, Volta y los controladores 525–579 usan las compilaciones CUDA 12 de Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, la diarización, Qwen3-TTS, la separación y el director, y Parakeet Ultra por Vulkan. Drama 3 necesita CUDA 13: cabe entero en 24 GB; desde 6 GB sus expertos (18 GB) viven en la RAM, que entonces necesita 32 GB, y la VRAM que toma se elige en Ajustes (Auto o de 5 a 24 GB). Una tarjeta AMD o Intel usa las compilaciones Vulkan, un equipo sin tarjeta las de CPU y un Mac Metal (sin Drama 3); estas configuraciones no las ha probado el autor
- **WebView2**: viene en Windows 11; en Windows 10 lo descarga el instalador
- **Disco:** unos 9 GB para los modelos, motores y bibliotecas por defecto, más lo que añadas (Drama 3 ocupa 23 GB)

En un equipo con NVIDIA solo instalas a mano un **[controlador NVIDIA](https://www.nvidia.com/Download/index.aspx)** reciente.

## Inicio rápido

1. **Descarga** de la [última versión](https://github.com/timoncool/speech-studio/releases/latest): para Windows la portátil `Speech-Studio-<versión>-portable-windows-x64.zip` (descomprímela donde quieras) o el instalador `Speech.Studio_<versión>_x64-setup.exe`; para macOS `Speech-Studio-<versión>-macos-arm64.dmg`; para Linux `Speech-Studio-<versión>-linux-x86_64-experimental.AppImage` o `Speech-Studio-<versión>-linux-amd64-experimental.deb`.
2. **Ejecuta** `Speech Studio.exe` (Windows), Speech Studio desde Aplicaciones (macOS) o el AppImage (Linux).
3. En el panel **Primer inicio** pulsa **Descargar lo marcado**: la aplicación descarga una vez los modelos, motores y bibliotecas.
4. **Pega un texto** y pulsa **Crear**, o cambia a **Listening** y suelta una grabación.

> En Windows todo se descarga y vive **dentro de la carpeta de la aplicación**: modelos, cachés, voces y proyectos no van a ningún otro sitio. En Linux vive en `~/.local/share/speech-studio`, en macOS en `~/Library/Application Support/Speech Studio`.

Qué cambió y cuándo está en [CHANGELOG.md](CHANGELOG.md), y el botón de destellos de la parte superior lo muestra. Reglas para colaboradores y agentes de código: [AGENTS.md](AGENTS.md).

## Todo lo que descarga la aplicación

El panel del primer inicio descarga todo esto con un botón. Detrás de un proxy, o donde Hugging Face está bloqueado, configura el proxy en Ajustes → **Red**. También puedes descargar tú los archivos directos, ponerlos donde dice la última columna (desde la carpeta de la aplicación) y pulsar **Importar de carpeta**; los componentes en archivo comprimido los descarga la aplicación. Cada archivo se comprueba contra su SHA-256 fijado antes de usarse. Los tamaños y hashes vienen del manifiesto de la aplicación, `crates/speech-server/src/setup.rs`, y de las versiones fijadas de los crates compartidos que nombra, y esta tabla se comprueba contra ellos. La tabla lista los archivos de Windows: una tarjeta NVIDIA en la que funciona CUDA 13 toma las compilaciones CUDA 13, una tarjeta NVIDIA o un controlador más antiguos las CUDA 12.9, una tarjeta AMD o Intel las Vulkan y un equipo sin tarjeta las de CPU; una etapa a la que se le elige otro dispositivo en Ajustes → Modelos toma la compilación de ese dispositivo. En Linux y macOS la aplicación descarga las compilaciones de los mismos componentes para ese sistema.

<!-- downloads:start -->
| Componente | Necesidad | Archivos (enlaces directos) | Tamaño | Dónde ponerlo |
|---|---|---|---|---|
| Higgs Audio v3 Q8_0 | obligatorio | [q8_0.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/q8_0.gguf) → `models\higgs-q8_0\q8_0.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/config.json) → `models\higgs-q8_0\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/chat_template.jinja) → `models\higgs-q8_0\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer.json) → `models\higgs-q8_0\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer_config.json) → `models\higgs-q8_0\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q8_0\higgs_audio_v2_tokenizer_config.json` | 5.5 GB | tal cual, en la ruta tras la flecha |
| Audio8 TTS Realistic-Air Q8_0 | opcional | [Audio8-TTS-Realistic-Air-Q8_0.gguf](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF/resolve/10f8bcf54be8b20b53abaaa1a074a62b34089ab2/Audio8-TTS-Realistic-Air-Q8_0.gguf) → `models\audio8\Audio8-TTS-Realistic-Air-Q8_0.gguf` | 1.5 GB | tal cual, en la ruta tras la flecha |
| audio.cpp server, CUDA 13 (Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarization) | obligatorio | [audiocpp-server-win-cuda13-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda13-x64.zip) | 136 MB | descomprimir los archivos, sin subcarpetas, en `tools\audiocpp\` |
| Gemma-4 12B QAT q4_0 + vision | opcional | [gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\gemma-4-12b-it-qat-q4_0.gguf`<br>[mmproj-gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/mmproj-gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\mmproj-gemma-4-12b-it-qat-q4_0.gguf` | 7.2 GB | tal cual, en la ruta tras la flecha |
| Gemma-4 12B Q5_K_M + vision | opcional | [gemma-4-12b-it-Q5_K_M.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q5_K_M.gguf) → `models\mt-q5_0\gemma-4-12b-it-Q5_K_M.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q5_0\mmproj-F16.gguf` | 8.6 GB | tal cual, en la ruta tras la flecha |
| Gemma-4 12B Q6_K + vision | opcional | [gemma-4-12b-it-Q6_K.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q6_K.gguf) → `models\mt-q6_k\gemma-4-12b-it-Q6_K.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q6_k\mmproj-F16.gguf` | 10.0 GB | tal cual, en la ruta tras la flecha |
| Gemma-4 12B Q8_0 + vision | opcional | [gemma-4-12b-it-Q8_0.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q8_0.gguf) → `models\mt-q8_0\gemma-4-12b-it-Q8_0.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q8_0\mmproj-F16.gguf` | 12.8 GB | tal cual, en la ruta tras la flecha |
| Parakeet-TDT 0.6B v3 Q8_0 | obligatorio | [parakeet-tdt-0.6b-v3-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/resolve/d8d8c32cb2017c57e5307767343e97870f8a7f53/parakeet-tdt-0.6b-v3-Q8_0.gguf) → `models\asr-gguf-parakeet-v3\parakeet-tdt-0.6b-v3-Q8_0.gguf` | 740 MB | tal cual, en la ruta tras la flecha |
| Higgs Audio v3 Q6_K | opcional | [q6_k.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/q6_k.gguf) → `models\higgs-q6_k\q6_k.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/config.json) → `models\higgs-q6_k\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/chat_template.jinja) → `models\higgs-q6_k\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer.json) → `models\higgs-q6_k\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer_config.json) → `models\higgs-q6_k\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q6_k\higgs_audio_v2_tokenizer_config.json` | 5.0 GB | tal cual, en la ruta tras la flecha |
| Higgs Audio v3 Q4_K_M | opcional | [q4_k_m.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/q4_k_m.gguf) → `models\higgs-q4_k_m\q4_k_m.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/config.json) → `models\higgs-q4_k_m\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/chat_template.jinja) → `models\higgs-q4_k_m\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer.json) → `models\higgs-q4_k_m\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer_config.json) → `models\higgs-q4_k_m\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q4_k_m\higgs_audio_v2_tokenizer_config.json` | 4.1 GB | tal cual, en la ruta tras la flecha |
| Fish Audio Drama 3 Q4 + drama_engine (6 GB+ GPU, 32 GB RAM under 24 GB) | opcional | [.gitattributes](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/.gitattributes) → `models\drama3\.gitattributes`<br>[LICENSE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/LICENSE) → `models\drama3\LICENSE`<br>[NOTICE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/NOTICE) → `models\drama3\NOTICE`<br>[README.md](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/README.md) → `models\drama3\README.md`<br>[codec.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/codec.safetensors) → `models\drama3\codec.safetensors`<br>[drama.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/drama.json) → `models\drama3\drama.json`<br>[globals.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/globals.safetensors) → `models\drama3\globals.safetensors`<br>[layer00.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer00.safetensors) → `models\drama3\layer00.safetensors`<br>[layer01.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer01.safetensors) → `models\drama3\layer01.safetensors`<br>[layer02.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer02.safetensors) → `models\drama3\layer02.safetensors`<br>[layer03.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer03.safetensors) → `models\drama3\layer03.safetensors`<br>[layer04.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer04.safetensors) → `models\drama3\layer04.safetensors`<br>[layer05.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer05.safetensors) → `models\drama3\layer05.safetensors`<br>[layer06.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer06.safetensors) → `models\drama3\layer06.safetensors`<br>[layer07.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer07.safetensors) → `models\drama3\layer07.safetensors`<br>[layer08.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer08.safetensors) → `models\drama3\layer08.safetensors`<br>[layer09.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer09.safetensors) → `models\drama3\layer09.safetensors`<br>[layer10.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer10.safetensors) → `models\drama3\layer10.safetensors`<br>[layer11.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer11.safetensors) → `models\drama3\layer11.safetensors`<br>[layer12.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer12.safetensors) → `models\drama3\layer12.safetensors`<br>[layer13.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer13.safetensors) → `models\drama3\layer13.safetensors`<br>[layer14.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer14.safetensors) → `models\drama3\layer14.safetensors`<br>[layer15.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer15.safetensors) → `models\drama3\layer15.safetensors`<br>[layer16.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer16.safetensors) → `models\drama3\layer16.safetensors`<br>[layer17.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer17.safetensors) → `models\drama3\layer17.safetensors`<br>[layer18.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer18.safetensors) → `models\drama3\layer18.safetensors`<br>[layer19.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer19.safetensors) → `models\drama3\layer19.safetensors`<br>[layer20.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer20.safetensors) → `models\drama3\layer20.safetensors`<br>[layer21.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer21.safetensors) → `models\drama3\layer21.safetensors`<br>[layer22.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer22.safetensors) → `models\drama3\layer22.safetensors`<br>[layer23.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer23.safetensors) → `models\drama3\layer23.safetensors`<br>[layer24.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer24.safetensors) → `models\drama3\layer24.safetensors`<br>[layer25.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer25.safetensors) → `models\drama3\layer25.safetensors`<br>[layer26.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer26.safetensors) → `models\drama3\layer26.safetensors`<br>[layer27.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer27.safetensors) → `models\drama3\layer27.safetensors`<br>[layer28.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer28.safetensors) → `models\drama3\layer28.safetensors`<br>[layer29.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer29.safetensors) → `models\drama3\layer29.safetensors`<br>[layer30.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer30.safetensors) → `models\drama3\layer30.safetensors`<br>[layer31.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer31.safetensors) → `models\drama3\layer31.safetensors`<br>[layer32.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer32.safetensors) → `models\drama3\layer32.safetensors`<br>[layer33.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer33.safetensors) → `models\drama3\layer33.safetensors`<br>[layer34.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer34.safetensors) → `models\drama3\layer34.safetensors`<br>[layer35.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer35.safetensors) → `models\drama3\layer35.safetensors`<br>[layer36.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer36.safetensors) → `models\drama3\layer36.safetensors`<br>[layer37.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer37.safetensors) → `models\drama3\layer37.safetensors`<br>[layer38.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer38.safetensors) → `models\drama3\layer38.safetensors`<br>[layer39.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer39.safetensors) → `models\drama3\layer39.safetensors`<br>[tokenizer.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tokenizer.json) → `models\drama3\tokenizer.json`<br>[tower.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tower.safetensors) → `models\drama3\tower.safetensors`<br>[drama_engine.dll](https://github.com/timoncool/drama-rs/releases/download/engine-0.1.3/drama_engine.dll) → `models\drama-engine\drama_engine.dll` | 23.0 GB | tal cual, en la ruta tras la flecha |
| Supertonic 3 int8 | opcional | [LICENSE](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/LICENSE) → `models\supertonic3\LICENSE`<br>[duration_predictor.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/duration_predictor.onnx) → `models\supertonic3\onnx\duration_predictor.onnx`<br>[text_encoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/text_encoder.onnx) → `models\supertonic3\onnx\text_encoder.onnx`<br>[vector_estimator.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vector_estimator.onnx) → `models\supertonic3\onnx\vector_estimator.onnx`<br>[vocoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vocoder.onnx) → `models\supertonic3\onnx\vocoder.onnx`<br>[tts.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/tts.json) → `models\supertonic3\onnx\tts.json`<br>[unicode_indexer.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/unicode_indexer.json) → `models\supertonic3\onnx\unicode_indexer.json`<br>[F1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F1.json) → `models\supertonic3\voice_styles\F1.json`<br>[F2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F2.json) → `models\supertonic3\voice_styles\F2.json`<br>[F3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F3.json) → `models\supertonic3\voice_styles\F3.json`<br>[F4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F4.json) → `models\supertonic3\voice_styles\F4.json`<br>[F5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F5.json) → `models\supertonic3\voice_styles\F5.json`<br>[M1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M1.json) → `models\supertonic3\voice_styles\M1.json`<br>[M2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M2.json) → `models\supertonic3\voice_styles\M2.json`<br>[M3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M3.json) → `models\supertonic3\voice_styles\M3.json`<br>[M4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M4.json) → `models\supertonic3\voice_styles\M4.json`<br>[M5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M5.json) → `models\supertonic3\voice_styles\M5.json` | 108 MB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS engine (qwen.dll) and codec | opcional | [qwen-tokenizer-12hz-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-tokenizer-12hz-Q8_0.gguf) → `models\qwen3-tts\qwen-tokenizer-12hz-Q8_0.gguf`<br>[qwen-windows-x64.zip](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/qwen-windows-x64.zip) | 295 MB | tal cual, en la ruta tras la flecha<br>descomprimir los archivos, sin subcarpetas, en `models\qwen3-tts\` |
| Qwen3-TTS 1.7B Q4_K_M (CustomVoice + Base) | opcional | [qwen-talker-1.7b-customvoice-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q4_K_M.gguf`<br>[qwen-talker-1.7b-base-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q4_K_M.gguf` | 2.4 GB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS 1.7B Q8_0 (CustomVoice + Base) | opcional | [qwen-talker-1.7b-customvoice-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q8_0.gguf`<br>[qwen-talker-1.7b-base-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q8_0.gguf` | 4.1 GB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS VoiceDesign Q4_K_M | opcional | [qwen-talker-1.7b-voicedesign-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q4_K_M.gguf` | 1.2 GB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS VoiceDesign Q8_0 | opcional | [qwen-talker-1.7b-voicedesign-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q8_0.gguf` | 2.0 GB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS Russian stress LoRA | opcional | [adapter.safetensors](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf/resolve/c16b3af3db3095120efc1ebb73f6522442bcb3b1/lora/adapter.safetensors) → `models\qwen3-tts\qwen-talker-1.7b-base-ru-stress-lora.safetensors` | 35 MB | tal cual, en la ruta tras la flecha |
| Parakeet Ultra 0.6B Q8_0 (Moondream, transcribe.cpp) | opcional | [parakeet-ultra-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-ultra-gguf/resolve/a1539be1b849ea7463b88343e5277b373d0c4ea9/parakeet-ultra-Q8_0.gguf) → `models\asr-gguf-parakeet-ultra\parakeet-ultra-Q8_0.gguf` | 740 MB | tal cual, en la ruta tras la flecha |
| transcribe.cpp engine (CUDA 13) | obligatorio | [transcribe-windows-cuda13-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cuda13-x64.zip) | 131 MB | descomprimir los archivos, sin subcarpetas, en `models\transcribe-engine\` |
| transcribe.cpp engine (CPU) | opcional | [transcribe-windows-cpu-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cpu-x64.zip) | 5 MB | descomprimir los archivos, sin subcarpetas, en `models\transcribe-engine-cpu\` |
| transcribe.cpp engine (Vulkan) | opcional | [transcribe-windows-vulkan-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-vulkan-x64.zip) | 19 MB | descomprimir los archivos, sin subcarpetas, en `models\transcribe-engine-vulkan\` |
| Nemotron 3.5 ASR 0.6B Q8_0 (NVIDIA) | opcional | [nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf](https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/53ce288f69326b415aeef030cfcb02a08a8096dd/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf) → `models\asr-gguf-nemotron\nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf` | 751 MB | tal cual, en la ruta tras la flecha |
| CrisperWhisper 2.0 turbo (audio.cpp) | opcional | [LICENSE.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/LICENSE.md) → `models\crisperwhisper2-turbo\LICENSE.md`<br>[README.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/README.md) → `models\crisperwhisper2-turbo\README.md`<br>[added_tokens.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/added_tokens.json) → `models\crisperwhisper2-turbo\added_tokens.json`<br>[config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/config.json) → `models\crisperwhisper2-turbo\config.json`<br>[generation_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/generation_config.json) → `models\crisperwhisper2-turbo\generation_config.json`<br>[merges.txt](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/merges.txt) → `models\crisperwhisper2-turbo\merges.txt`<br>[model.safetensors](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/model.safetensors) → `models\crisperwhisper2-turbo\model.safetensors`<br>[normalizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/normalizer.json) → `models\crisperwhisper2-turbo\normalizer.json`<br>[preprocessor_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/preprocessor_config.json) → `models\crisperwhisper2-turbo\preprocessor_config.json`<br>[special_tokens_map.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/special_tokens_map.json) → `models\crisperwhisper2-turbo\special_tokens_map.json`<br>[tokenizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer.json) → `models\crisperwhisper2-turbo\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer_config.json) → `models\crisperwhisper2-turbo\tokenizer_config.json`<br>[vocab.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/vocab.json) → `models\crisperwhisper2-turbo\vocab.json` | 1.6 GB | tal cual, en la ruta tras la flecha |
| Nemotron 3 Diarization | recomendado | [nemotron3_diar_v3.onnx](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/nemotron3_diar_v3.onnx) → `models\nemotron-diar\nemotron3_diar_v3.onnx`<br>[LICENSE](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/LICENSE) → `models\nemotron-diar\LICENSE` | 401 MB | tal cual, en la ruta tras la flecha |
| Mel-Band Roformer voc_fv6 Q8_0 | recomendado | [voc_fv6-Q8_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q8_0.gguf) → `models\bsroformer\voc_fv6-Q8_0.gguf` | 252 MB | tal cual, en la ruta tras la flecha |
| Mel-Band Roformer voc_fv6 Q5_0 | opcional | [voc_fv6-Q5_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q5_0.gguf) → `models\bsroformer\voc_fv6-Q5_0.gguf` | 167 MB | tal cual, en la ruta tras la flecha |
| Mel-Band Roformer voc_fv6 Q4_0 | opcional | [voc_fv6-Q4_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q4_0.gguf) → `models\bsroformer\voc_fv6-Q4_0.gguf` | 139 MB | tal cual, en la ruta tras la flecha |
| WeSpeaker ResNet34-LM (voice similarity) | recomendado | [voxceleb_resnet34_LM.onnx](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM/resolve/f0c48c298fd835726c27956a5d617bad7115627e/voxceleb_resnet34_LM.onnx) → `models\faces\wespeaker\voxceleb_resnet34_LM.onnx` | 27 MB | tal cual, en la ruta tras la flecha |
| Voice gate (ReDimNet2-B3) | opcional | [redimnet2_b3_lm.onnx](https://huggingface.co/diarizeapp/redimnet2-b3-onnx/resolve/96144a5d37ba2900e5ba61e726d7cdd7f0c087a8/redimnet2_b3_lm.onnx) → `models\voice-gate\redimnet2_b3_lm.onnx` | 19 MB | tal cual, en la ruta tras la flecha |
| Wake word (PhoneticXeus int8) | opcional | [model.int8.onnx](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/model.int8.onnx) → `models\wake-word\model.int8.onnx`<br>[vocab.json](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/vocab.json) → `models\wake-word\vocab.json` | 357 MB | tal cual, en la ruta tras la flecha |
| RUAccent (stress, «ё», homographs) | recomendado | [model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/model.onnx) → `models\ruaccent\nn\nn_accent\model.onnx`<br>[config.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/config.json) → `models\ruaccent\nn\nn_accent\config.json`<br>[vocab.txt](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/vocab.txt) → `models\ruaccent\nn\nn_accent\vocab.txt`<br>[model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/model.onnx) → `models\ruaccent\nn\nn_omograph\tiny2\model.onnx`<br>[tokenizer.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/tokenizer.json) → `models\ruaccent\nn\nn_omograph\tiny2\tokenizer.json`<br>[accents.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/accents.json.gz) → `models\ruaccent\dictionary\accents.json.gz`<br>[omographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/omographs.json.gz) → `models\ruaccent\dictionary\omographs.json.gz`<br>[yo_words.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_words.json.gz) → `models\ruaccent\dictionary\yo_words.json.gz`<br>[yo_homographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_homographs.json.gz) → `models\ruaccent\dictionary\yo_homographs.json.gz` | 34 MB | tal cual, en la ruta tras la flecha |
| RUAccent big_poetry homographs int8 | opcional | [model.onnx](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/model.onnx) → `models\ruaccent\nn\nn_omograph\big_poetry\model.onnx`<br>[tokenizer.json](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/tokenizer.json) → `models\ruaccent\nn\nn_omograph\big_poetry\tokenizer.json` | 185 MB | tal cual, en la ruta tras la flecha |
| Smart Turn v3.2 (end of phrase in Live) | opcional | [smart-turn-v3.2-f32.gguf](https://huggingface.co/audio-cpp/audio.cpp-gguf/resolve/73ce4162f6cce370dd533c6782b772135c1b2a8e/Smart-Turn-v3-GGUF/smart-turn-v3.2-f32.gguf) → `models\smart-turn\smart-turn-v3.2-f32.gguf` | 32 MB | tal cual, en la ruta tras la flecha |
| Silero VAD v6.2 | recomendado | [silero-vad-v6.2-F32.gguf](https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/a9b2d61c20ae4ddf3644edb34969e8185a485448/silero-vad-v6.2-F32.gguf) → `models\silero-vad\silero-vad-v6.2-F32.gguf` | 1 MB | tal cual, en la ruta tras la flecha |
| VoxLingua107 language identification Q8_0 | obligatorio | [lang-id-voxlingua107-ecapa-Q8_0.gguf](https://huggingface.co/handy-computer/lang-id-voxlingua107-ecapa-gguf/resolve/e59f0f3847e77723b7d58447ef9456fa0c7e9436/lang-id-voxlingua107-ecapa-Q8_0.gguf) → `models\langid-voxlingua107\lang-id-voxlingua107-ecapa-Q8_0.gguf` | 27 MB | tal cual, en la ruta tras la flecha |
| DPDFNet 48 kHz (noise suppression) | recomendado | [dpdfnet2_48khz_hr.onnx](https://huggingface.co/Ceva-IP/DPDFNet/resolve/dd6818d00f50c836fed43a6243ebe49116de5964/onnx/dpdfnet2_48khz_hr.onnx) → `models\dpdfnet\dpdfnet2_48khz_hr.onnx` | 10 MB | tal cual, en la ruta tras la flecha |
| BSRoformer.cpp (CUDA 13) | recomendado | [BSRoformer-windows-cuda13-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda13-x64.zip) | 119 MB | descomprimir los archivos, sin subcarpetas, en `tools\bsroformer\` |
| BSRoformer.cpp (CPU) | opcional | [BSRoformer-windows-cpu-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cpu-x64.zip) | 3 MB | descomprimir los archivos, sin subcarpetas, en `tools\bsroformer-cpu\` |
| BSRoformer.cpp (Vulkan) | opcional | [BSRoformer-windows-vulkan-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-vulkan-x64.zip) | 26 MB | descomprimir los archivos, sin subcarpetas, en `tools\bsroformer-vulkan\` |
| llama.cpp server (CUDA 13) | opcional | [llama-b11900-bin-win-cuda-13.4-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cuda-13.4-x64.zip) | 165 MB | descomprimir los archivos, sin subcarpetas, en `tools\llama\` |
| ONNX Runtime | obligatorio | [onnxruntime-win-x64-1.28.2.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-1.28.2.zip) | 79 MB | descomprimir con su árbol de carpetas en `models\runtime\` |
| ONNX Runtime GPU (CUDA) | recomendado | [onnxruntime-win-x64-gpu_cuda13-1.28.2.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-gpu_cuda13-1.28.2.zip) | 366 MB | descomprimir con su árbol de carpetas en `models\runtime\` |
| FFmpeg (static build) | obligatorio | [ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip](https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip) | 199 MB | tomar ffmpeg.exe y ffprobe.exe del archivo y ponerlos en `tools\ffmpeg\` |
| yt-dlp + deno | opcional | [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/yt-dlp.exe) → `tools\yt-dlp\yt-dlp.exe`<br>[deno-x86_64-pc-windows-msvc.zip](https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip) | 61 MB | tal cual, en la ruta tras la flecha<br>descomprimir los archivos, sin subcarpetas, en `tools\yt-dlp\` |
| CUDA 13 runtime (cudart, cuBLAS, cuFFT) | obligatorio | [nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/86/00/d5436004268f049214193659ebc36550b5ef3925c3d13b4cc980e13be6f5/nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl)<br>[nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/a3/df/f1246959833e2c437db8be3e5b477f66b87f8817821ed40de6c7561c9a36/nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl)<br>[libcufft-windows-x86_64-12.4.0.43-archive.zip](https://developer.download.nvidia.com/compute/cuda/redist/libcufft/windows-x86_64/libcufft-windows-x86_64-12.4.0.43-archive.zip) | 586 MB | tomar todos los .dll del archivo y ponerlos en `models\higgs-engine\` |
| cuDNN 9 | recomendado | [nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/87/6a/e55ff0ac26a5c6e2b21f41c9d04ad096b4ed6da593fba7e25845c61b0532/nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl) | 436 MB | tomar todos los .dll del archivo y ponerlos en `models\higgs-engine\` |
| audio.cpp server, CUDA 12.9 (cards and drivers CUDA 13 left) | opcional | [audiocpp-server-win-cuda12-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda12-x64.zip) | 184 MB | descomprimir los archivos, sin subcarpetas, en `tools\audiocpp-cuda12\` |
| audio.cpp server, Vulkan | opcional | [audiocpp-server-win-vulkan-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-vulkan-x64.zip) | 36 MB | descomprimir los archivos, sin subcarpetas, en `tools\audiocpp-vulkan\` |
| audio.cpp server, CPU | opcional | [audiocpp-server-win-cpu-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cpu-x64.zip) | 19 MB | descomprimir los archivos, sin subcarpetas, en `tools\audiocpp-cpu\` |
| CUDA 12.9 runtime (cudart, cuBLAS, cuFFT) | opcional | [nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/59/df/e7c3a360be4f7b93cee39271b792669baeb3846c58a4df6dfcf187a7ffab/nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl)<br>[nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/e2/fc9a0e985249d873150276d5afb02e39a66817fedbf1a385724393e505ed/nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl)<br>[nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/ee/29955203338515b940bd4f60ffdbc073428f25ef9bfbce44c9a066aedc5c/nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl) | 757 MB | tomar todos los .dll del archivo y ponerlos en `models\cuda12-runtime\` |
| Qwen3-TTS CUDA 13 module | opcional | [ggml-cuda13-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda13-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 142 MB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS CUDA 12.9 module | opcional | [ggml-cuda12-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda12-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 170 MB | tal cual, en la ruta tras la flecha |
| Qwen3-TTS Vulkan module | opcional | [ggml-vulkan-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-vulkan-windows-x64.dll) → `models\qwen3-tts\ggml-vulkan.dll` | 42 MB | tal cual, en la ruta tras la flecha |
| BSRoformer.cpp (CUDA 12.9) | opcional | [BSRoformer-windows-cuda12-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda12-x64.zip) | 173 MB | descomprimir los archivos, sin subcarpetas, en `tools\bsroformer-cuda12\` |
| llama.cpp server (CUDA 12.4) | opcional | [llama-b11900-bin-win-cuda-12.4-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cuda-12.4-x64.zip) | 279 MB | descomprimir los archivos, sin subcarpetas, en `tools\llama-cuda12\` |
| llama.cpp server (Vulkan) | opcional | [llama-b11900-bin-win-vulkan-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-vulkan-x64.zip) | 37 MB | descomprimir los archivos, sin subcarpetas, en `tools\llama-vulkan\` |
| llama.cpp server (CPU) | opcional | [llama-b11900-bin-win-cpu-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cpu-x64.zip) | 24 MB | descomprimir los archivos, sin subcarpetas, en `tools\llama-cpu\` |
<!-- downloads:end -->

## Qué se ejecuta dónde

Cada etapa tiene su propio dispositivo en Ajustes → Modelos: Auto, GPU (CUDA), Vulkan o CPU. Auto toma la compilación CUDA de la tarjeta: CUDA 13, o CUDA 12.9 en una tarjeta o un controlador más antiguos.

| Etapa | Motor | NVIDIA (CUDA) | Vulkan (AMD, Intel) | CPU | Mac (Apple silicon) | Nube |
|---|---|---|---|---|---|---|
| Voz y clonación | Higgs Audio v3 (audio.cpp) | sí | sí | sí | Metal | OpenRouter, Gemini, fal.ai, Runware |
| Voz y clonación | Qwen3-TTS (qwentts.cpp) | sí | sí | sí | Metal | — |
| Voz y clonación | Audio8 TTS (audio.cpp) | sí | sí | sí | Metal | — |
| Voz y clonación | Fish Audio Drama 3 (drama-rs) | solo CUDA 13 | no | no | no | — |
| Voz | Supertonic 3 (audio.cpp) | sí | sí | sí | Metal | — |
| Reconocimiento de voz | Parakeet-TDT v3, Nemotron 3.5 (audio.cpp) | sí | sí | sí | Metal | OpenRouter, fal.ai |
| Reconocimiento de voz | Parakeet Ultra (transcribe.cpp) | CUDA 13 | sí | sí | Metal | — |
| Reconocimiento de voz | CrisperWhisper 2.0 turbo (audio.cpp) | sí | no | sí | Metal | — |
| Hablantes | Nemotron 3 Diarization (audio.cpp) | sí | sí | sí | Metal | no |
| Pausas y ruido | Silero VAD v6 (transcribe.cpp), DPDFNet | no | no | sí | CPU | no |
| Voz separada de la música | Mel-Band Roformer (BSRoformer.cpp) | sí | sí | sí | Metal | no |
| Acentos y homógrafos | RUAccent | no | no | sí | CPU | no |
| Director (pódcast, personajes) | Gemma-4 12B (llama.cpp) | sí | sí | sí | Metal | OpenRouter |

## Solución de problemas

**El instalador se detiene en WebView2.** Instala WebView2 con el instalador [Evergreen Standalone x64](https://go.microsoft.com/fwlink/p/?LinkId=2124701) de Microsoft y ejecuta de nuevo el instalador de Speech Studio.

**Las descargas se atascan o fallan.** Los archivos grandes continúan donde se quedaron: pulsa el botón otra vez. Si Hugging Face o GitHub están bloqueados, configura un proxy en Ajustes → **Red**, o coloca los archivos de la tabla y pulsa **Importar de carpeta**.

**Un motor de voz habla de una tarjeta o controlador antiguos.** Las compilaciones CUDA 13 necesitan una tarjeta desde Turing (GTX 16, RTX 20 y posteriores) y el controlador 580 o posterior; GTX 900, GTX 10, Volta y los controladores 525–579 reciben solas las compilaciones CUDA 12. Drama 3 solo tiene compilación CUDA 13: en una tarjeta o un controlador más antiguos actualiza el controlador o usa otro motor o las voces en la nube.

**El reconocimiento no encuentra voz.** La grabación está por debajo del umbral de voz: bájalo en **Reconocimiento → Corte en las pausas**.

**Ajustes → Diagnóstico** reúne un paquete para informes de errores sin claves: adjúntalo a un [issue](https://github.com/timoncool/speech-studio/issues).

## Cómo funciona

**Stack:** un shell nativo **Tauri 2 (Rust)** ejecuta `speech-server` (axum) en el mismo proceso en `127.0.0.1:8797` y abre una ventana con la SPA: React 19 + Vite + Tailwind. Los motores viven en el proceso a través de sus ABI de C: Drama 3 (`drama_engine.dll`, [timoncool/drama-rs](https://github.com/timoncool/drama-rs)), Qwen3-TTS (`qwen.dll`, [timoncool/qwentts.cpp](https://github.com/timoncool/qwentts.cpp)); Parakeet Ultra, Silero VAD y VoxLingua107 (`transcribe_engine.dll`, [timoncool/transcribe.cpp](https://github.com/timoncool/transcribe.cpp)); DPDFNet, RUAccent y WeSpeaker sobre ONNX Runtime. Higgs Audio v3, Audio8 TTS, Supertonic 3, Parakeet-TDT v3, Nemotron 3.5, CrisperWhisper y Nemotron 3 Diarization funcionan en un programa auxiliar, el servidor audio.cpp (`audiocpp_server`, [timoncool/audio.cpp](https://github.com/timoncool/audio.cpp)); BSRoformer.cpp y el servidor de llama.cpp también son auxiliares; ffmpeg para el audio. **Ni un solo proceso de Python en ejecución.**

### Compilar desde el código

```bash
git clone https://github.com/timoncool/speech-studio.git
cd speech-studio

cd frontend && npm install && npm run build && cd ..   # 1) SPA
cargo build --release -p speech-server                  # 2) servidor nativo (axum)
cd desktop && npm install && npx tauri build            # 3) shell de escritorio (Tauri)
```

Necesitas Node 20+, Rust (toolchain MSVC) y WebView2. Los motores no hace falta recompilarlos: la aplicación descarga binarios ya compilados. Los paquetes de la versión salen de `scripts/build-release.ps1` (Windows), `scripts/build-linux-docker.sh` (Linux, en un contenedor Ubuntu 22.04) y `scripts/build-release-macos.sh` (macOS, en un Mac con Apple silicon).

### Linux (experimental)

El .deb y el AppImage para Linux x86-64 son **experimentales**: el mismo código con las compilaciones Linux de los mismos motores, sin probar por el autor en un escritorio Linux real. Las correcciones son muy bienvenidas como pull requests. El controlador NVIDIA (580 o posterior para las compilaciones CUDA 13, 525 o posterior para las CUDA 12), `libgomp1` y `libssl3` vienen del sistema; los modelos y motores van a `~/.local/share/speech-studio`.

### macOS (experimental)

El .dmg para macOS 13 o posterior en Apple silicon también es **experimental**: las compilaciones macOS de los mismos motores sobre Metal, sin probar por el autor en un Mac real. Drama 3 no está allí: funciona solo en tarjetas NVIDIA. La aplicación está firmada ad hoc y sin notarizar, así que macOS detiene el primer inicio: ábrela una vez desde Ajustes del Sistema → Privacidad y seguridad → **Abrir igualmente**. Los modelos y motores van a `~/Library/Application Support/Speech Studio`.

## Estadísticas anónimas y novedades

La aplicación pide novedades al servidor del autor al iniciarse y luego cada hora; la petición no lleva ningún identificador. La pantalla del primer inicio tiene una casilla **Enviar estadísticas de uso anónimas**, marcada por defecto; el mismo interruptor está en Ajustes, junto a **Qué se envía**. Mientras está marcada, la aplicación envía el informe del día: un id de instalación aleatorio creado en este equipo, la versión, el SO, el idioma de la ventana, la tarjeta gráfica como fabricante, un rango de memoria de vídeo y si CUDA funciona, cuántas tareas terminaron, fallaron o se cancelaron, los modelos de cada etapa, los segundos de voz que dieron los motores locales y los segundos que trabajaron para ello, y por qué falló una tarea, en una línea sin rutas, nombres ni texto entre comillas (en una descarga fallida, el componente y el sitio del que venía). Nunca: textos, grabaciones, voces, nombres de archivo ni nada personal. `DO_NOT_TRACK=1` o `STUDIO_TELEMETRY=0` desactivan las estadísticas por completo.

## Más aplicaciones de IA portátiles

| Proyecto | Qué es |
|--------|----------|
| [Dub Studio](https://github.com/timoncool/dub-studio) | Doblaje de vídeo con IA: voces clonadas, subtítulos traducidos y texto en pantalla |
| [Higgs Ultimate](https://github.com/timoncool/Higgs-Ultimate) | Síntesis de voz y clonación nativas (Higgs Audio v3) |
| [ACE-Step Studio](https://github.com/timoncool/ACE-Step-Studio) | Estudio de música con IA: canciones, voces, versiones y clips |
| [YuE2 Studio](https://github.com/timoncool/YuE2-Studio) | Generador de canciones con partitura editable |
| [MiniMax Music3 Studio](https://github.com/timoncool/MiniMax-Music3-Studio) | Estudio de música nativo local/nube con MiniMax Music 3 |
| [VideoSOS](https://github.com/timoncool/videosos) | Estudio de vídeo con IA |

## Contribuir y forks

Los colaboradores son muy bienvenidos: correcciones de las compilaciones **macOS y Linux**, **tarjetas AMD e Intel** sobre Vulkan y más idiomas de la interfaz (`frontend/src/locales/`, `crates/speech-server/locales/`): haz un fork y envía un pull request.

## Autores

- **Nerual Dreming** — [Telegram](https://t.me/nerual_dreming) | [neuro-cartel.com](https://neuro-cartel.com) | fundador de [ArtGeneration.me](https://artgeneration.me)
- **Neuro-Soft** — [Telegram](https://t.me/neuroport) | aplicaciones de IA portátiles

## Créditos

- **[Boson AI](https://huggingface.co/bosonai)**: Higgs Audio v3, y **[drbaph / Higgs-Audio-v3-Studio](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio)**: sus cuantizaciones GGUF.
- **[Fish Audio](https://fish.audio)**: Drama 3; **[apiplant/drama-q4](https://huggingface.co/apiplant/drama-q4)**: sus pesos Q4; **[drama-rs](https://github.com/timoncool/drama-rs)**: el motor Rust/CUDA.
- **[Equipo Qwen](https://huggingface.co/Qwen)**: Qwen3-TTS; **[ServeurpersoCom / qwentts.cpp](https://github.com/ServeurpersoCom/qwentts.cpp)** y **[Serveurperso/Qwen3-TTS-GGUF](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF)**: el motor C++ y las cuantizaciones GGUF; **[siriusfreak](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf)**: la LoRA de acento ruso.
- **[Edge0](https://huggingface.co/Edge0/Audio8-TTS-Realistic-Air)**: Audio8 TTS Realistic-Air; **[Hansda/Audio8-TTS-Realistic-Air-GGUF](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF)**: sus cuantizaciones GGUF.
- **[Supertone](https://huggingface.co/Supertone)**: Supertonic 3; **[askurios8/supertonic-3-int8](https://huggingface.co/askurios8/supertonic-3-int8)**: su exportación int8.
- **[RUAccent](https://huggingface.co/ruaccent)**: acentos, «ё» y homógrafos.
- **[NVIDIA](https://huggingface.co/nvidia)**: Parakeet-TDT, Nemotron 3.5 ASR y Nemotron 3 Diarization; el ONNX de la diarización, de [altunenes/parakeet-rs](https://github.com/altunenes/parakeet-rs).
- **[handy-computer / transcribe.cpp](https://github.com/handy-computer/transcribe.cpp)**: el motor transcribe.cpp y los GGUF de Parakeet-TDT, Parakeet Ultra, Nemotron 3.5, Silero VAD v6.2 y VoxLingua107.
- **[Silero](https://github.com/snakers4/silero-vad)**: Silero VAD v6; **[Ceva-IP / DPDFNet](https://huggingface.co/Ceva-IP/DPDFNet)**: supresión de ruido.
- **[nyra health / CrisperWhisper 2.0 turbo](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo)**: reconocimiento Whisper con tiempos de palabra; **[0xShug0 / audio.cpp](https://github.com/0xShug0/audio.cpp)**: el motor en el que funcionan Higgs Audio v3, Audio8 TTS, Supertonic 3, Parakeet-TDT v3, Nemotron 3.5, CrisperWhisper y Nemotron 3 Diarization.
- **[chenmozhijin / BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp)** y **[GaboxR67](https://huggingface.co/GaboxR67)**: separación de voz.
- **[WeSpeaker](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM)**: parecido de voz; **[Google Gemma](https://huggingface.co/google)** y **[llama.cpp](https://github.com/ggml-org/llama.cpp)**: el director.
- **[ONNX Runtime](https://github.com/microsoft/onnxruntime)**, **[FFmpeg](https://ffmpeg.org)**, **[Tauri](https://tauri.app)**, **[ort](https://github.com/pykeio/ort)** y las bibliotecas NVIDIA CUDA.

## Apoya al autor

Hago software libre e investigo en IA; casi todo lo que creo es gratuito. Las donaciones me permiten crear e investigar más.

**[Todas las formas de apoyar](DONATE.md)** | **[dalink.to/nerual_dreming](https://dalink.to/nerual_dreming)** | **[boosty.to/neuro_art](https://boosty.to/neuro_art)**

- **BTC:** `1E7dHL22RpyhJGVpcvKdbyZgksSYkYeEBC`
- **ETH (ERC20):** `0xb5db65adf478983186d4897ba92fe2c25c594a0c`
- **USDT (TRC20):** `TQST9Lp2TjK6FiVkn4fwfGUee7NmkxEE7C`

## Historial de estrellas

<a href="https://github.com/timoncool/speech-studio/stargazers">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="docs/stars-dark.svg" />
   <source media="(prefers-color-scheme: light)" srcset="docs/stars-light.svg" />
   <img alt="Star History Chart" src="docs/stars-light.svg" />
 </picture>
</a>

## Licencia

El código de la aplicación es [MIT](LICENSE). **Los modelos no**: cada uno conserva su licencia. Léelas antes de publicar o vender lo que locutes.

| Componente | Licencia | Qué significa |
|---|---|---|
| Higgs Audio v3 (Boson AI) | [Boson Higgs TTS 3 Research and Non-Commercial License](https://huggingface.co/bosonai/higgs-tts-3-4b/blob/main/LICENSE) | Investigación y uso personal; los creadores pueden monetizar su propio contenido con el Creator Use Grant si citan Higgs Audio de Boson AI; un producto o servicio para otros necesita licencia de Boson |
| Fish Audio Drama 3 | [Fish Audio Community License](https://huggingface.co/fishaudio/drama-3/blob/main/LICENSE) | Investigación y uso no comercial; uso comercial mientras tus ingresos anuales sean menores de 500 000 USD, por encima, licencia de Fish Audio; sin servicio alojado de texto a voz |
| Qwen3-TTS (Qwen) | Apache-2.0 |  |
| LoRA de acento ruso de Qwen3-TTS (siriusfreak) | Apache-2.0 |  |
| Audio8 TTS Realistic-Air (Edge0) | Apache-2.0 |  |
| Supertonic 3 (Supertone) | BigScience OpenRAIL-M | Lee sus restricciones de uso |
| RUAccent y su modelo de homógrafos en int8 | MIT |  |
| Parakeet-TDT, Nemotron 3 Diarization (NVIDIA) | CC-BY-4.0, OpenMDW-1.1 | Cita a NVIDIA |
| Nemotron 3.5 ASR (NVIDIA) | OpenMDW-1.1 | Cita a NVIDIA |
| transcribe.cpp | MIT |  |
| Silero VAD | MIT |  |
| DPDFNet (Ceva) | Apache-2.0 |  |
| CrisperWhisper 2.0 turbo (nyra health) | MIT para el código; nyra health Non-Commercial Research License para los pesos y lo que generan | Solo uso no comercial; el uso comercial necesita una licencia de nyra health |
| audio.cpp | Apache-2.0 |  |
| Mel-Band Roformer voc_fv6, GGUF | Las páginas de los modelos no indican licencia | Pregunta a los autores antes del uso comercial |
| WeSpeaker ResNet34-LM | CC-BY-4.0 | Cita a WeSpeaker |
| Gemma-4 12B (Google) | Términos de Gemma | Léelos |
| llama.cpp, ONNX Runtime, BSRoformer.cpp, qwentts.cpp | MIT |  |
| ElevenLabs UI Orb, SiriWave | MIT |  |
| LiveKit Agents UI Aura | Apache-2.0 |  |
| FFmpeg (compilación BtbN) | Compilación GPL | Programa aparte, no enlazado con la aplicación |
| NVIDIA CUDA runtime, cuBLAS, cuFFT, cuDNN | Términos propios de NVIDIA | Se descargan de NVIDIA y PyPI |

<sub>texto a voz · clonación de voz · TTS en ruso · acentos · creador de audiolibros · pódcast · reconocimiento de voz · transcripción · diarización · sin conexión · local · código abierto · Windows · macOS · Linux</sub>

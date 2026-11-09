<div align="center">

<img src="frontend/public/favicon.svg" width="72" alt="Logo de Speech Studio"/>

# Speech Studio

**Studio vocal IA gratuit et hors ligne pour Windows, macOS (Apple silicon) et Linux — fait parler n’importe quel texte avec des voix clonées ou intégrées et transforme les enregistrements en texte avec leurs locuteurs. Le russe comme il faut : nombres au bon cas, accents toniques, « ё » et homographes. 100 % local, zéro Python : une seule application native (Rust + C++ sur CUDA, Vulkan ou Metal) ; chaque modèle et chaque moteur se télécharge d’un clic.**

[![License](https://img.shields.io/github/license/timoncool/speech-studio?style=flat-square)](LICENSE)
[![Stars](https://img.shields.io/github/stars/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/stargazers)
[![Latest release](https://img.shields.io/github/v/release/timoncool/speech-studio?include_prereleases&style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Downloads](https://img.shields.io/github/downloads/timoncool/speech-studio/total?style=flat-square)](https://github.com/timoncool/speech-studio/releases)
[![Last commit](https://img.shields.io/github/last-commit/timoncool/speech-studio?style=flat-square)](https://github.com/timoncool/speech-studio/commits)

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh.md) · [Español](README.es.md) · [Português](README.pt.md) · **Français**

![Speech Studio — un dialogue avec une voix par personnage](docs/screenshots/fr-01-speech.webp)

</div>

## Ce que c’est

**Speech Studio** a trois modes. **Speech** fait parler un texte : une phrase, un dialogue avec une voix par personnage, un podcast que le réalisateur écrit pour vous ou un livre entier depuis EPUB, FB2, PDF ou DOCX. **Listening** transforme un enregistrement, une vidéo ou un lien en texte avec qui a dit quoi et quand, le monte par ses mots, vérifie un doublage contre le texte qu’il devait dire et crée des voix de bibliothèque à partir de ses locuteurs. **Live** est une conversation à voix haute avec un agent IA connecté par MCP : il dit chaque phrase dès qu’il l’écrit et vous entend au micro.

Tout tourne **sur votre machine** : ni cloud ni abonnement ; vos textes, enregistrements et empreintes vocales ne quittent pas l’ordinateur. Pour une voix que votre carte ne peut pas faire tourner, les voix cloud d’**OpenRouter**, **Google Gemini**, **fal.ai** et **Runware** sont là aussi, chacune avec sa clé et une confirmation avant le premier appel ; elles sont désactivées par défaut.

C’est une **application entièrement native** : ni Python embarqué, ni torch, ni wheels CUDA. Tout le studio est **Rust + moteurs natifs C++ (GGUF/ONNX) sur CUDA, Vulkan ou Metal**. Les modèles, les moteurs, les bibliothèques CUDA et VC++ et ffmpeg sont **téléchargés par l’application elle-même** au premier lancement, et un modèle qu’un autre studio de l’auteur possède déjà est repris au lieu d’être retéléchargé. **L’application est conçue et testée pour les cartes NVIDIA** ; pour le reste, voir *Ce qui tourne où*.

## Pour les agents IA

Tant que Speech Studio est ouvert, il sert MCP sur `http://127.0.0.1:8797/mcp` : un agent comme Claude Code, Claude Desktop, Cursor ou Codex fait tout ce que fait la fenêtre, avec le même code — fait parler un texte dans un fichier en un seul appel, crée et modifie des projets bloc par bloc, attribue des voix aux personnages, conçoit une voix d’après une description, transcrit et vérifie des enregistrements, exporte des livres audio avec chapitres et sous-titres. Dans Paramètres, **Agent IA (MCP)** indique si un agent est connecté et quoi coller dans le client. La seule chose qu’un agent ne peut pas faire, c’est allumer le micro : Live, un enregistrement et le test du micro ne démarrent que sur votre propre clic ou touche.

1. Installez le studio depuis la [dernière version](https://github.com/timoncool/speech-studio/releases/latest) et lancez-le.
2. Connectez-vous à son serveur MCP :
   ```bash
   claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp
   ```
   Autres clients : `{ "mcpServers": { "speech-studio": { "type": "streamable-http", "url": "http://127.0.0.1:8797/mcp" } } }`
3. Lisez la compétence servie par le serveur (ressource `studio://skill`), le même texte que [docs/mcp-skill.md](docs/mcp-skill.md), et commencez par l’outil `studio_status`.

Sur le même port, le studio parle l’API audio d’OpenAI — `POST /v1/audio/speech`, `POST /v1/audio/transcriptions`, `GET /v1/models` —, donc les outils écrits pour OpenAI parlent et transcrivent à travers lui, avec le nom d’une voix de la bibliothèque comme `voice`. [llms.txt](llms.txt) dit la même chose pour les outils qui le cherchent. Seuls les programmes de cet ordinateur peuvent se connecter.

## Ce que vous pouvez en faire

- **Un livre audio** — déposez un EPUB ou un FB2 : chapitres, personnages avec leurs voix, un M4B avec chapitres et couverture, volume ACX vérifié.
- **Un podcast ou un dialogue** — le réalisateur écrit un podcast sur un sujet pour deux ou trois voix, ou collez des lignes comme « Anne : … » ; la frise montre chaque chevauchement.
- **Une voix off pour une vidéo** — faites dire un script ou des sous-titres dans leurs temps, posez musique et sons dessous dans l’éditeur de pistes, exportez au volume de la vidéo.
- **La transcription d’une interview ou d’un cours** — qui a dit quoi et quand, le temps de chaque mot, des sous-titres selon les règles des diffuseurs, l’enregistrement monté par son texte.
- **La vérification d’un doublage** — posez le script sur l’enregistrement et voyez chaque mot mal dit ou sauté.
- **Votre propre bibliothèque de voix** — enregistrez ou importez des voix, nettoyez-les, créez des voix à partir des locuteurs d’un enregistrement.
- **Une voix pour votre agent IA** — parlez avec Claude Code ou tout agent MCP dans Live, ou laissez-le doubler et transcrire des fichiers par MCP ou par l’API compatible OpenAI.

Des exemples de tous les moteurs et voix, avec l’accentuation russe avant et après, sont sur la [page du projet](https://timoncool.github.io/speech-studio/fr.html).

## Trois modes

| Mode | Ce qu’il fait |
|------|--------------|
| 🔊 **Speech** | Texte, dialogue, podcast ou livre → blocs doublés avec prises, vérifiés à l’oreille, exportés en WAV, FLAC, MP3, OGG, M4A ou M4B avec chapitres, sous-titres et texte synchronisé |
| 👂 **Listening** | Audio, vidéo ou lien → une transcription avec locuteurs et temps par mot, un lecteur karaoké, le montage par le texte, l’export SRT, VTT, ASS, TXT ou JSON et la vérification contre un texte de référence |
| 🎙️ **Live** | Une conversation vocale avec un agent IA : l’agent connecté par MCP dit chaque phrase dès qu’il l’écrit, vous répondez au micro et êtes reconnu phrase par phrase, une phrase dite par-dessus l’arrête ; vous seul allumez le micro, pause avec M ou Ctrl+Maj+Espace, un widget au-dessus de toutes les fenêtres |

## Fonctionnalités

**Voix et moteurs**

- **Six moteurs locaux** — [Higgs Audio v3](https://huggingface.co/bosonai) (clonage, émotions et sons), **Fish Audio Drama 3** (la voix ouverte la plus forte ; entière sur une carte de 24 Go, dès 6 Go avec ses experts en mémoire vive), **Audio8 TTS** (parole naturelle en 23 langues, clonage ou la voix propre du modèle), **Supertonic 3** (rapide, dix voix intégrées), **Qwen3-TTS 1.7B** (neuf voix intégrées, clonage, une consigne par bloc) et **Qwen3-TTS avec l’accent tonique russe**, la LoRA de siriusfreak appliquée à la volée.
- **Moteur par personnage et par bloc** au-dessus de celui du projet ; un moteur qui ne peut pas parler avec la voix choisie dit quelle voix intégrée il prendra.
- **Bibliothèque de voix** — un pack de voix en un clic, 743 voix russes du catalogue cloud, enregistrement au micro, import de fichiers et de ZIP, photo, genre, âge, langue, tags et consentement ; la transcription de la référence est reconnue une fois et gardée.
- **Nettoyage d’une référence** — Mel-Band Roformer sépare la voix de la musique, **DPDFNet** retire le bruit avec un réglage d’intensité pour que la voix reste vivante ; l’original est conservé.
- **Le meilleur extrait d’une référence** — des fenêtres aux pauses entre les mots reconnus, chacune avec sa durée, son rapport signal/bruit, l’écrêtage et la plus longue pause.
- **Voix sur description** — Qwen3-TTS VoiceDesign crée une voix d’après « une vieille dame à la voix douce, un peu rauque… » : écoutez plusieurs candidats et gardez-en un dans la bibliothèque.

- **Voix dans le cloud** — OpenRouter, Google Gemini (requêtes en parallèle ou Batch à moitié prix), fal.ai et Runware, chacune avec sa clé et une confirmation avant le premier appel.

**Un texte qui sonne juste**

- **Normaliseur du russe** — nombres au bon cas et au bon genre, dates, heures, montants, unités, chiffres romains, marques latines et sigles.
- **Accents toniques, « ё » et homographes** par RUAccent — le grand modèle d’homographes (big_poetry en int8, les mêmes choix que le fp32) décide selon la phrase ; les cas douteux sont surlignés pour vous et vos `+` restent.
- **Dictionnaire de prononciation** — remplacement, regex et phonèmes, à l’échelle du projet, d’une langue, d’une voix ou d’un moteur, import des `.rex`/`.dic`/`.bxd` de Balabolka et écoute d’une règle directement depuis le dictionnaire.
- **Balisage compatible SpeechKit** — accent `+`, pauses `sil<[300]>` et `[pause 500ms]`, débit `[rate:1.2]`, `**emphase**` ; ce qu’un moteur ignore est signalé, jamais perdu en silence.

- **Encore plus de texte bien lu** — un sous-ensemble de SSML, des indications de prosodie en couleur mot par mot, téléphones, adresses e-mail et codes, un passage dans une autre langue avec la même voix et une vérification du script avant le doublage.

**Projets**

- **Le texte comme timeline** — blocs avec locuteurs, prises (jusqu’à cinq), suivi des blocs périmés, annuler et rétablir, glisser pour réordonner, rechercher et remplacer en regex, notes et marqueurs.
- **Contrôle qualité** — chaque prise est reconnue en retour et comparée mot à mot, plus des contrôles d’écrêtage, de silence, de coupure, de clic final, de phrase répétée, de longue traîne, de musique ou de bruit à la place de la voix et de ressemblance avec la voix de référence ; plusieurs prises avec la meilleure choisie.
- **Dialogues et podcasts** — les lignes `Anna:` et `[Anna]` deviennent des personnages, le réalisateur écrit un podcast sur un thème pour N locuteurs, les pauses négatives créent des interruptions et la **chronologie** montre chaque chevauchement avec couper, enchaîner ou garder.
- **Livres audio** — EPUB, FB2, PDF, DOCX, HTML, Markdown et TXT avec aperçu des chapitres, distribution par le réalisateur, M4B avec chapitres et couverture, découpage des longs livres et contrôle de volume ACX.
- **Sous-titres comme projet** — les répliques SRT, VTT et ASS sont doublées dans leur créneau : le débit est relevé d’avance, une prise trop longue est refaite, puis viennent l’accélération et l’emprunt du silence qui précède.
- **Vérification avant l’export** — les blocs sans audio, périmés, en échec, au fichier modifié ou plus longs que leur réplique sont listés avant l’export ; seule la durée peut être autorisée.
- **Volume** — préréglages vidéo (−14 LUFS), podcast (−16 LUFS) et ACX avec limiteur true-peak, room tone et respirations entre les blocs.
- **Fichier de projet** — `.speechproj` avec texte, voix et audio emporte un projet sur un autre ordinateur.

- **Montage** — un éditeur de pistes sous le texte : déplacer, rogner, fondus et volume de chaque clip, pistes de musique et de sons depuis des fichiers ou des dossiers, ou une chanson faite sur place dans YuE2, MiniMax Music3 ou ACE-Step Studio.
- **Un export pour chaque usage** — un fichier par chapitre, une piste par personnage dans un ZIP, un tableau CSV des répliques, des effets de voix (radio, téléphone, haut-parleur), un volume égal des morceaux, fréquence et profondeur de bits, des modèles de noms de fichiers, un manifeste SHA-256 et une note sur la façon dont c’est fait.
- **Comparer et valider** — A/B des prises, une comparaison à l’aveugle des moteurs sur le même texte, des blocs validés qu’un nouveau doublage ne touche pas, un feu de qualité et un rapport de projet.
- **Livres et pièces** — le discours direct à part de celui du narrateur, une distribution des personnages avec des voix choisies pour eux, les notes de bas de page nettoyées, les noms du livre dans le dictionnaire, un projet mis à jour depuis un livre corrigé sans perdre ce qui est doublé et un lecteur HTML du livre.

**Listening**

- **Parakeet-TDT v3, Nemotron 3.5 (28 langues avec ponctuation) et CrisperWhisper 2.0 turbo (99 langues avec le temps des mots) sur audio.cpp, Parakeet Ultra sur transcribe.cpp**, jusqu’à huit locuteurs avec Nemotron 3 Diarization, un long enregistrement coupé aux pauses par **Silero VAD v6**, le bruit retiré par DPDFNet et la musique par Roformer.
- **Un indice pour CrisperWhisper** — le sujet et l’orthographe des termes, plus vos corrections.
- **Lot** — déposez plusieurs fichiers d’un coup ; chacun devient un enregistrement dans la file de reconnaissance.
- **Un texte connu posé sur l’enregistrement** — le texte de référence prend le temps de chaque mot entendu, phrase après phrase.
- **Des voix à partir des locuteurs** — une voix de bibliothèque faite des mots les plus nets de chaque locuteur.

- **Pendant que vous parlez** — reconnaissance au micro phrase par phrase, un dossier surveillé reconnu en arrière-plan, une reconnaissance rapide sans l’éditeur.
- **Le bon modèle tout seul** — avec la langue sur Auto, VoxLingua107 la trouve et l’enregistrement va à un module qui la connaît ; le modèle choisi se télécharge à sa première utilisation.
- **Sous-titres selon les règles des diffuseurs** (Netflix, BBC et d’autres), qui parle combien, nombres en chiffres à l’export.

**Live**

- **Une conversation à voix haute** — l’agent connecté par MCP dit chaque phrase dès qu’il l’écrit ; vous répondez au micro et êtes reconnu phrase par phrase ; une phrase d’au moins deux mots dite par-dessus l’agent l’arrête, alors qu’une toux, un rire ou un « mm-hmm » non ; une ligne tapée lui parvient comme si vous l’aviez dite.
- **Des changements à la volée** — la voix de l’agent, le module de reconnaissance, la langue et le micro se changent en pleine conversation, par vous ou par l’agent à votre demande : le module de reconnaissance change sur place, un nouveau micro ou une nouvelle langue redémarrent le micro et, pendant que le moteur de la nouvelle voix se charge, la fenêtre affiche « Chargement de la voix… ».
- **Un rythme naturel** — une phrase de l’agent est dite en entier jusqu’à 300 caractères, une pause que le moteur laisse dans une phrase dure au plus 0,6 s et chaque phrase n’est entendue qu’une fois.
- **Vous seul allumez le micro** — Live, un enregistrement dans Listening, un enregistrement de voix et le test du micro ne démarrent que sur votre propre clic ou touche ; un agent n’a aucun outil qui allume le micro, et la fenêtre refuse ses clics sur ces boutons. La barre du haut affiche « Micro allumé », pour quoi et depuis combien de temps, avec une croix qui l’éteint ; sans aucune fenêtre du studio ouverte pendant cinq secondes, le micro s’éteint tout seul.
- **Micro et son** — pause avec M ou Ctrl+Maj+Espace depuis n’importe quelle fenêtre (l’agent se tait et ne vous entend pas ; maintenir pour parler), la croix termine la conversation et ce que dit l’agent, vumètres, vous réécouter, test du haut-parleur et volume de l’agent.
- **Cinq vues et un widget** — aura, onde, barres, radial et matrice, et un petit widget au-dessus de toutes les fenêtres ; Ctrl+3 ouvre Live.
- **Mesuré** — votre phrase devient du texte 0,64–0,74 s après que vous vous êtes tu (Parakeet sur une RTX 4090), et le premier son de la réponse arrive entre 15 ms et 1 s après que l’agent l’a écrite, selon le moteur.

**Partout**

- **API compatible OpenAI**, **MCP** avec pilotage de la fenêtre pour les agents et un panneau d’assistant sur un modèle OpenRouter avec les mêmes outils.
- **Diagnostic** — carte graphique, pilote, CUDA, bibliothèques, journaux et un paquet pour rapport de bug sans clés.
- **Toute l’application en six langues**, thème clair et sombre, mise à jour automatique, entièrement portable.
- **Le premier lancement ne demande que ce que vous voulez** — voix, reconnaissance et extras ; les moteurs, les bibliothèques et la version pour votre carte viennent avec, une ligne chacun avec sa taille.
- **Palette de commandes** (Ctrl+K), vos propres raccourcis, un mode simple et une aide « ? » sur chaque élément.

## Vitesse mesurée

Sur un même ordinateur : RTX 4090 (24 Go) sous Windows 11, Speech Studio tel qu’il s’installe.

**Doublage** — Speech Studio 0.2.0, le même texte russe de 17 secondes, la deuxième exécution de chaque moteur (modèle chargé) ; Higgs — dans le moteur audio.cpp le 22.10, trois exécutions, premier son non mesuré ; Audio8 — le 09.11, un texte russe de 18 secondes avec une voix clonée : il rend la phrase entière, donc son premier son arrive en 2,3 s sur ce texte et en 0,4–1,0 s pour une phrase dite en Live ; le premier son est le délai avant que l’audio commence dans la fenêtre, la mémoire vidéo le pic pendant la lecture.

| Moteur | Plus rapide que le temps réel | Premier son | Streaming | Téléchargement | Mémoire vidéo |
|---|---|---|---|---|---|
| Supertonic 3 | 168× | 103 ms | par phrase | 0,1 Go | 1,0 Go |
| Qwen3-TTS | 12× | 19 ms | pendant qu’il parle | 2,2 Go | 3,1 Go |
| Qwen3-TTS + ru stress LoRA | 10× | 26 ms | pendant qu’il parle | 2,3 Go | 3,5 Go |
| Fish Audio Drama 3 | 7,0× | 326 ms | pendant qu’il parle | 21,4 Go | 21,9 Go |
| Audio8 TTS | 5,8× | 2,3 s | par phrase | 1,5 Go | 2,0 Go |
| Higgs Audio v3 | 3,4× | — | pendant qu’il parle | 5,3 Go | ~7 Go |

**Reconnaissance** — Speech Studio 0.2.1 (CrisperWhisper — moteur audio.cpp le 22.10 : un enregistrement russe de 625,8 s, chargement du moteur compris ; sur le processeur et sa mémoire vidéo non mesurés), un enregistrement russe de 5 minutes d’une seule voix (299,7 s) dans Listening sans séparer les locuteurs, chargement du modèle compris ; le modèle libère la mémoire vidéo à la fin de la tâche.

| Modèle | Sur la carte | Sur le processeur | Langues | Téléchargement | Mémoire vidéo |
|---|---|---|---|---|---|
| Parakeet TDT v3 | 96× (3,1 s) | 33× (9,2 s) | 25 | 0,7 Go | 1,2 Go |
| Parakeet Ultra | 73× (4,1 s) | 33× (9,2 s) | 25 | 0,7 Go | 1,0 Go |
| Nemotron 3.5 | 73× (4,1 s) | 29× (10,3 s) | 28 | 0,7 Go | 1,0 Go |
| CrisperWhisper 2.0 turbo | 96× (6,5 s) | — | 99 | 1,6 Go | — |

**Live** — un tour de conversation.

| Étape | Temps |
|---|---|
| Votre phrase en texte après que vous vous êtes tu (Parakeet sur la carte, pause de fin de phrase comprise) | 0,64–0,74 s |
| L’agent écrit sa réponse (Claude Sonnet) | 2,0–3,9 s |
| Premier son de la réponse (selon le moteur, voir Doublage) | 15 ms – 1,0 s |

## Captures d’écran

L’écran d’accueil — un texte, un livre, des sous-titres ou un script de podcast deviennent un projet :

![Écran d’accueil de Speech Studio](docs/screenshots/fr-08-home.webp)

Listening — transcription avec locuteurs et temps par mot, lecteur karaoké, montage par le texte et export :

![Listening dans Speech Studio](docs/screenshots/fr-06-listening.webp)

Live — une conversation vocale avec un agent IA : ce que vous avez dit et ce qu’il répond, cinq vues, pause et fin de la conversation :

![Live dans Speech Studio](docs/screenshots/fr-11-live.webp)

## Configuration requise

- **OS :** Windows 10 / 11 (x64) ; macOS 13 ou plus récent sur Apple silicon et Linux x86-64 en versions expérimentales
- **GPU :** NVIDIA avec 8 Go de VRAM ou plus. Les cartes à partir de Turing (GTX 16, RTX 20 et plus récentes) avec le pilote 580 ou plus récent prennent les builds CUDA 13 ; GTX 900, GTX 10, Volta et les pilotes 525–579 prennent les builds CUDA 12 de Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, la diarisation, Qwen3-TTS, la séparation et le réalisateur, et Parakeet Ultra passe par Vulkan. Drama 3 demande CUDA 13 : il tient entier dans 24 Go ; dès 6 Go ses experts (18 Go) restent en mémoire vive, qui doit alors faire 32 Go, et la VRAM qu’il prend se règle dans les Réglages (Auto ou de 5 à 24 Go). Une carte AMD ou Intel prend les builds Vulkan, un ordinateur sans carte ceux du CPU, un Mac Metal (sans Drama 3) ; ces configurations ne sont pas testées par l’auteur
- **WebView2** — préinstallé sur Windows 11 ; sur Windows 10, l’installateur le récupère
- **Disque :** environ 9 Go pour les modèles, moteurs et bibliothèques par défaut, plus ce que vous ajoutez (Drama 3 fait 23 Go)

Sur une machine NVIDIA, la seule chose à installer à la main est un **[pilote NVIDIA](https://www.nvidia.com/Download/index.aspx)** récent.

## Démarrage rapide

1. **Téléchargez** depuis la [dernière version](https://github.com/timoncool/speech-studio/releases/latest) : pour Windows la version portable `Speech-Studio-<version>-portable-windows-x64.zip` (à décompresser où vous voulez) ou l’installeur `Speech.Studio_<version>_x64-setup.exe` ; pour macOS `Speech-Studio-<version>-macos-arm64.dmg` ; pour Linux `Speech-Studio-<version>-linux-x86_64-experimental.AppImage` ou `Speech-Studio-<version>-linux-amd64-experimental.deb`.
2. **Lancez** `Speech Studio.exe` (Windows), Speech Studio depuis Applications (macOS) ou l’AppImage (Linux).
3. Dans le panneau **Premier lancement**, cliquez sur **Télécharger la sélection** — l’application récupère une fois les modèles, moteurs et bibliothèques.
4. **Collez un texte** et cliquez sur **Créer**, ou passez à **Listening** et déposez un enregistrement.

> Sous Windows, tout se télécharge et reste **dans le dossier de l’application** : modèles, caches, voix et projets ne vont nulle part ailleurs. Sous Linux, tout est dans `~/.local/share/speech-studio`, sous macOS dans `~/Library/Application Support/Speech Studio`.

Ce qui a changé et quand se trouve dans [CHANGELOG.md](CHANGELOG.md), et le bouton à étincelles en haut de l’application l’affiche. Règles pour les contributeurs et les agents de code : [AGENTS.md](AGENTS.md).

## Tout ce que l’application télécharge

Le panneau du premier lancement récupère tout ceci d’un clic. Derrière un proxy, ou là où Hugging Face est bloqué, réglez le proxy dans Paramètres → **Réseau**. Vous pouvez aussi télécharger vous-même les fichiers directs, les placer là où l’indique la dernière colonne (à partir du dossier de l’application) et cliquer sur **Importer d’un dossier** ; les composants en archive, l’application les télécharge elle-même. Chaque fichier est vérifié contre son SHA-256 épinglé avant usage. Tailles et empreintes viennent du manifeste de l’application, `crates/speech-server/src/setup.rs`, et des versions épinglées des crates partagés qu’il nomme, et ce tableau est vérifié contre eux. Le tableau donne les fichiers de Windows : une carte NVIDIA où tourne CUDA 13 prend les builds CUDA 13, une carte NVIDIA ou un pilote plus anciens les builds CUDA 12.9, une carte AMD ou Intel les builds Vulkan, un ordinateur sans carte les builds CPU ; une étape réglée sur un autre appareil dans Réglages → Modèles prend le build de cet appareil. Sous Linux et macOS, l’application télécharge les builds des mêmes composants pour ce système.

<!-- downloads:start -->
| Composant | Besoin | Fichiers (liens directs) | Taille | Où le placer |
|---|---|---|---|---|
| Higgs Audio v3 Q8_0 | obligatoire | [q8_0.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/q8_0.gguf) → `models\higgs-q8_0\q8_0.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/config.json) → `models\higgs-q8_0\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/chat_template.jinja) → `models\higgs-q8_0\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer.json) → `models\higgs-q8_0\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/tokenizer_config.json) → `models\higgs-q8_0\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q8_0/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q8_0\higgs_audio_v2_tokenizer_config.json` | 5.5 GB | tel quel, au chemin indiqué après la flèche |
| Audio8 TTS Realistic-Air Q8_0 | facultatif | [Audio8-TTS-Realistic-Air-Q8_0.gguf](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF/resolve/10f8bcf54be8b20b53abaaa1a074a62b34089ab2/Audio8-TTS-Realistic-Air-Q8_0.gguf) → `models\audio8\Audio8-TTS-Realistic-Air-Q8_0.gguf` | 1.5 GB | tel quel, au chemin indiqué après la flèche |
| audio.cpp server, CUDA 13 (Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarization) | obligatoire | [audiocpp-server-win-cuda13-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda13-x64.zip) | 136 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\audiocpp\` |
| Gemma-4 12B QAT q4_0 + vision | facultatif | [gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\gemma-4-12b-it-qat-q4_0.gguf`<br>[mmproj-gemma-4-12b-it-qat-q4_0.gguf](https://huggingface.co/google/gemma-4-12b-it-qat-q4_0-gguf/resolve/2b318d6ebebf093f50ca4376e858325f10703358/mmproj-gemma-4-12b-it-qat-q4_0.gguf) → `models\mt\mmproj-gemma-4-12b-it-qat-q4_0.gguf` | 7.2 GB | tel quel, au chemin indiqué après la flèche |
| Gemma-4 12B Q5_K_M + vision | facultatif | [gemma-4-12b-it-Q5_K_M.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q5_K_M.gguf) → `models\mt-q5_0\gemma-4-12b-it-Q5_K_M.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q5_0\mmproj-F16.gguf` | 8.6 GB | tel quel, au chemin indiqué après la flèche |
| Gemma-4 12B Q6_K + vision | facultatif | [gemma-4-12b-it-Q6_K.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q6_K.gguf) → `models\mt-q6_k\gemma-4-12b-it-Q6_K.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q6_k\mmproj-F16.gguf` | 10.0 GB | tel quel, au chemin indiqué après la flèche |
| Gemma-4 12B Q8_0 + vision | facultatif | [gemma-4-12b-it-Q8_0.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/gemma-4-12b-it-Q8_0.gguf) → `models\mt-q8_0\gemma-4-12b-it-Q8_0.gguf`<br>[mmproj-F16.gguf](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/d997c805aafe035a8024f961c6e1afd6b30d79a5/mmproj-F16.gguf) → `models\mt-q8_0\mmproj-F16.gguf` | 12.8 GB | tel quel, au chemin indiqué après la flèche |
| Parakeet-TDT 0.6B v3 Q8_0 | obligatoire | [parakeet-tdt-0.6b-v3-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/resolve/d8d8c32cb2017c57e5307767343e97870f8a7f53/parakeet-tdt-0.6b-v3-Q8_0.gguf) → `models\asr-gguf-parakeet-v3\parakeet-tdt-0.6b-v3-Q8_0.gguf` | 740 MB | tel quel, au chemin indiqué après la flèche |
| Higgs Audio v3 Q6_K | facultatif | [q6_k.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/q6_k.gguf) → `models\higgs-q6_k\q6_k.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/config.json) → `models\higgs-q6_k\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/chat_template.jinja) → `models\higgs-q6_k\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer.json) → `models\higgs-q6_k\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/tokenizer_config.json) → `models\higgs-q6_k\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q6_k/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q6_k\higgs_audio_v2_tokenizer_config.json` | 5.0 GB | tel quel, au chemin indiqué après la flèche |
| Higgs Audio v3 Q4_K_M | facultatif | [q4_k_m.gguf](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/q4_k_m.gguf) → `models\higgs-q4_k_m\q4_k_m.gguf`<br>[config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/config.json) → `models\higgs-q4_k_m\config.json`<br>[chat_template.jinja](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/chat_template.jinja) → `models\higgs-q4_k_m\chat_template.jinja`<br>[tokenizer.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer.json) → `models\higgs-q4_k_m\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/tokenizer_config.json) → `models\higgs-q4_k_m\tokenizer_config.json`<br>[higgs_audio_v2_tokenizer_config.json](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio/resolve/c6e9db5a2062c15accc1b9bfa54d927bbdb124dc/models/higgs-q4_k_m/higgs_audio_v2_tokenizer_config.json) → `models\higgs-q4_k_m\higgs_audio_v2_tokenizer_config.json` | 4.1 GB | tel quel, au chemin indiqué après la flèche |
| Fish Audio Drama 3 Q4 + drama_engine (6 GB+ GPU, 32 GB RAM under 24 GB) | facultatif | [.gitattributes](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/.gitattributes) → `models\drama3\.gitattributes`<br>[LICENSE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/LICENSE) → `models\drama3\LICENSE`<br>[NOTICE](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/NOTICE) → `models\drama3\NOTICE`<br>[README.md](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/README.md) → `models\drama3\README.md`<br>[codec.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/codec.safetensors) → `models\drama3\codec.safetensors`<br>[drama.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/drama.json) → `models\drama3\drama.json`<br>[globals.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/globals.safetensors) → `models\drama3\globals.safetensors`<br>[layer00.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer00.safetensors) → `models\drama3\layer00.safetensors`<br>[layer01.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer01.safetensors) → `models\drama3\layer01.safetensors`<br>[layer02.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer02.safetensors) → `models\drama3\layer02.safetensors`<br>[layer03.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer03.safetensors) → `models\drama3\layer03.safetensors`<br>[layer04.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer04.safetensors) → `models\drama3\layer04.safetensors`<br>[layer05.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer05.safetensors) → `models\drama3\layer05.safetensors`<br>[layer06.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer06.safetensors) → `models\drama3\layer06.safetensors`<br>[layer07.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer07.safetensors) → `models\drama3\layer07.safetensors`<br>[layer08.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer08.safetensors) → `models\drama3\layer08.safetensors`<br>[layer09.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer09.safetensors) → `models\drama3\layer09.safetensors`<br>[layer10.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer10.safetensors) → `models\drama3\layer10.safetensors`<br>[layer11.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer11.safetensors) → `models\drama3\layer11.safetensors`<br>[layer12.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer12.safetensors) → `models\drama3\layer12.safetensors`<br>[layer13.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer13.safetensors) → `models\drama3\layer13.safetensors`<br>[layer14.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer14.safetensors) → `models\drama3\layer14.safetensors`<br>[layer15.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer15.safetensors) → `models\drama3\layer15.safetensors`<br>[layer16.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer16.safetensors) → `models\drama3\layer16.safetensors`<br>[layer17.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer17.safetensors) → `models\drama3\layer17.safetensors`<br>[layer18.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer18.safetensors) → `models\drama3\layer18.safetensors`<br>[layer19.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer19.safetensors) → `models\drama3\layer19.safetensors`<br>[layer20.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer20.safetensors) → `models\drama3\layer20.safetensors`<br>[layer21.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer21.safetensors) → `models\drama3\layer21.safetensors`<br>[layer22.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer22.safetensors) → `models\drama3\layer22.safetensors`<br>[layer23.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer23.safetensors) → `models\drama3\layer23.safetensors`<br>[layer24.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer24.safetensors) → `models\drama3\layer24.safetensors`<br>[layer25.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer25.safetensors) → `models\drama3\layer25.safetensors`<br>[layer26.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer26.safetensors) → `models\drama3\layer26.safetensors`<br>[layer27.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer27.safetensors) → `models\drama3\layer27.safetensors`<br>[layer28.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer28.safetensors) → `models\drama3\layer28.safetensors`<br>[layer29.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer29.safetensors) → `models\drama3\layer29.safetensors`<br>[layer30.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer30.safetensors) → `models\drama3\layer30.safetensors`<br>[layer31.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer31.safetensors) → `models\drama3\layer31.safetensors`<br>[layer32.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer32.safetensors) → `models\drama3\layer32.safetensors`<br>[layer33.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer33.safetensors) → `models\drama3\layer33.safetensors`<br>[layer34.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer34.safetensors) → `models\drama3\layer34.safetensors`<br>[layer35.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer35.safetensors) → `models\drama3\layer35.safetensors`<br>[layer36.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer36.safetensors) → `models\drama3\layer36.safetensors`<br>[layer37.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer37.safetensors) → `models\drama3\layer37.safetensors`<br>[layer38.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer38.safetensors) → `models\drama3\layer38.safetensors`<br>[layer39.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/layer39.safetensors) → `models\drama3\layer39.safetensors`<br>[tokenizer.json](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tokenizer.json) → `models\drama3\tokenizer.json`<br>[tower.safetensors](https://huggingface.co/apiplant/drama-q4/resolve/1fce9bd21d46a0311294aebfad19f994df90902f/tower.safetensors) → `models\drama3\tower.safetensors`<br>[drama_engine.dll](https://github.com/timoncool/drama-rs/releases/download/engine-0.1.3/drama_engine.dll) → `models\drama-engine\drama_engine.dll` | 23.0 GB | tel quel, au chemin indiqué après la flèche |
| Supertonic 3 int8 | facultatif | [LICENSE](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/LICENSE) → `models\supertonic3\LICENSE`<br>[duration_predictor.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/duration_predictor.onnx) → `models\supertonic3\onnx\duration_predictor.onnx`<br>[text_encoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/text_encoder.onnx) → `models\supertonic3\onnx\text_encoder.onnx`<br>[vector_estimator.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vector_estimator.onnx) → `models\supertonic3\onnx\vector_estimator.onnx`<br>[vocoder.onnx](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/vocoder.onnx) → `models\supertonic3\onnx\vocoder.onnx`<br>[tts.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/tts.json) → `models\supertonic3\onnx\tts.json`<br>[unicode_indexer.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/onnx/unicode_indexer.json) → `models\supertonic3\onnx\unicode_indexer.json`<br>[F1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F1.json) → `models\supertonic3\voice_styles\F1.json`<br>[F2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F2.json) → `models\supertonic3\voice_styles\F2.json`<br>[F3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F3.json) → `models\supertonic3\voice_styles\F3.json`<br>[F4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F4.json) → `models\supertonic3\voice_styles\F4.json`<br>[F5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/F5.json) → `models\supertonic3\voice_styles\F5.json`<br>[M1.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M1.json) → `models\supertonic3\voice_styles\M1.json`<br>[M2.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M2.json) → `models\supertonic3\voice_styles\M2.json`<br>[M3.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M3.json) → `models\supertonic3\voice_styles\M3.json`<br>[M4.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M4.json) → `models\supertonic3\voice_styles\M4.json`<br>[M5.json](https://huggingface.co/askurios8/supertonic-3-int8/resolve/95618f5d61cef4923c3b802a332fd603723718f7/voice_styles/M5.json) → `models\supertonic3\voice_styles\M5.json` | 108 MB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS engine (qwen.dll) and codec | facultatif | [qwen-tokenizer-12hz-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-tokenizer-12hz-Q8_0.gguf) → `models\qwen3-tts\qwen-tokenizer-12hz-Q8_0.gguf`<br>[qwen-windows-x64.zip](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/qwen-windows-x64.zip) | 295 MB | tel quel, au chemin indiqué après la flèche<br>décompresser les fichiers, sans sous-dossiers, dans `models\qwen3-tts\` |
| Qwen3-TTS 1.7B Q4_K_M (CustomVoice + Base) | facultatif | [qwen-talker-1.7b-customvoice-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q4_K_M.gguf`<br>[qwen-talker-1.7b-base-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q4_K_M.gguf` | 2.4 GB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS 1.7B Q8_0 (CustomVoice + Base) | facultatif | [qwen-talker-1.7b-customvoice-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-customvoice-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-customvoice-Q8_0.gguf`<br>[qwen-talker-1.7b-base-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-base-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-base-Q8_0.gguf` | 4.1 GB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS VoiceDesign Q4_K_M | facultatif | [qwen-talker-1.7b-voicedesign-Q4_K_M.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q4_K_M.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q4_K_M.gguf` | 1.2 GB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS VoiceDesign Q8_0 | facultatif | [qwen-talker-1.7b-voicedesign-Q8_0.gguf](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF/resolve/b7ee2e8c7459c3bea99da23e3d178125a7d1713c/qwen-talker-1.7b-voicedesign-Q8_0.gguf) → `models\qwen3-tts\qwen-talker-1.7b-voicedesign-Q8_0.gguf` | 2.0 GB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS Russian stress LoRA | facultatif | [adapter.safetensors](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf/resolve/c16b3af3db3095120efc1ebb73f6522442bcb3b1/lora/adapter.safetensors) → `models\qwen3-tts\qwen-talker-1.7b-base-ru-stress-lora.safetensors` | 35 MB | tel quel, au chemin indiqué après la flèche |
| Parakeet Ultra 0.6B Q8_0 (Moondream, transcribe.cpp) | facultatif | [parakeet-ultra-Q8_0.gguf](https://huggingface.co/handy-computer/parakeet-ultra-gguf/resolve/a1539be1b849ea7463b88343e5277b373d0c4ea9/parakeet-ultra-Q8_0.gguf) → `models\asr-gguf-parakeet-ultra\parakeet-ultra-Q8_0.gguf` | 740 MB | tel quel, au chemin indiqué après la flèche |
| transcribe.cpp engine (CUDA 13) | obligatoire | [transcribe-windows-cuda13-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cuda13-x64.zip) | 131 MB | décompresser les fichiers, sans sous-dossiers, dans `models\transcribe-engine\` |
| transcribe.cpp engine (CPU) | facultatif | [transcribe-windows-cpu-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-cpu-x64.zip) | 5 MB | décompresser les fichiers, sans sous-dossiers, dans `models\transcribe-engine-cpu\` |
| transcribe.cpp engine (Vulkan) | facultatif | [transcribe-windows-vulkan-x64.zip](https://github.com/timoncool/transcribe.cpp/releases/download/engine-0.2.1/transcribe-windows-vulkan-x64.zip) | 19 MB | décompresser les fichiers, sans sous-dossiers, dans `models\transcribe-engine-vulkan\` |
| Nemotron 3.5 ASR 0.6B Q8_0 (NVIDIA) | facultatif | [nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf](https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/53ce288f69326b415aeef030cfcb02a08a8096dd/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf) → `models\asr-gguf-nemotron\nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf` | 751 MB | tel quel, au chemin indiqué après la flèche |
| CrisperWhisper 2.0 turbo (audio.cpp) | facultatif | [LICENSE.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/LICENSE.md) → `models\crisperwhisper2-turbo\LICENSE.md`<br>[README.md](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/README.md) → `models\crisperwhisper2-turbo\README.md`<br>[added_tokens.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/added_tokens.json) → `models\crisperwhisper2-turbo\added_tokens.json`<br>[config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/config.json) → `models\crisperwhisper2-turbo\config.json`<br>[generation_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/generation_config.json) → `models\crisperwhisper2-turbo\generation_config.json`<br>[merges.txt](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/merges.txt) → `models\crisperwhisper2-turbo\merges.txt`<br>[model.safetensors](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/model.safetensors) → `models\crisperwhisper2-turbo\model.safetensors`<br>[normalizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/normalizer.json) → `models\crisperwhisper2-turbo\normalizer.json`<br>[preprocessor_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/preprocessor_config.json) → `models\crisperwhisper2-turbo\preprocessor_config.json`<br>[special_tokens_map.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/special_tokens_map.json) → `models\crisperwhisper2-turbo\special_tokens_map.json`<br>[tokenizer.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer.json) → `models\crisperwhisper2-turbo\tokenizer.json`<br>[tokenizer_config.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/tokenizer_config.json) → `models\crisperwhisper2-turbo\tokenizer_config.json`<br>[vocab.json](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo/resolve/de0369c8a68025b7f6e86387b6eb5a3b369787c8/vocab.json) → `models\crisperwhisper2-turbo\vocab.json` | 1.6 GB | tel quel, au chemin indiqué après la flèche |
| Nemotron 3 Diarization | recommandé | [nemotron3_diar_v3.onnx](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/nemotron3_diar_v3.onnx) → `models\nemotron-diar\nemotron3_diar_v3.onnx`<br>[LICENSE](https://huggingface.co/altunenes/parakeet-rs/resolve/4d2a8bc71f5c896ec40faa59732e6716295edaf2/nemotron-3-diarization/LICENSE) → `models\nemotron-diar\LICENSE` | 401 MB | tel quel, au chemin indiqué après la flèche |
| Mel-Band Roformer voc_fv6 Q8_0 | recommandé | [voc_fv6-Q8_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q8_0.gguf) → `models\bsroformer\voc_fv6-Q8_0.gguf` | 252 MB | tel quel, au chemin indiqué après la flèche |
| Mel-Band Roformer voc_fv6 Q5_0 | facultatif | [voc_fv6-Q5_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q5_0.gguf) → `models\bsroformer\voc_fv6-Q5_0.gguf` | 167 MB | tel quel, au chemin indiqué après la flèche |
| Mel-Band Roformer voc_fv6 Q4_0 | facultatif | [voc_fv6-Q4_0.gguf](https://huggingface.co/chenmozhijin/BSRoformer-GGUF/resolve/df802a6773d25ba6ef785ff619daa3e510503168/GaboxR67/MelBandRoformers/melbandroformers/vocals/voc_fv6-Q4_0.gguf) → `models\bsroformer\voc_fv6-Q4_0.gguf` | 139 MB | tel quel, au chemin indiqué après la flèche |
| WeSpeaker ResNet34-LM (voice similarity) | recommandé | [voxceleb_resnet34_LM.onnx](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM/resolve/f0c48c298fd835726c27956a5d617bad7115627e/voxceleb_resnet34_LM.onnx) → `models\faces\wespeaker\voxceleb_resnet34_LM.onnx` | 27 MB | tel quel, au chemin indiqué après la flèche |
| Voice gate (ReDimNet2-B3) | facultatif | [redimnet2_b3_lm.onnx](https://huggingface.co/diarizeapp/redimnet2-b3-onnx/resolve/96144a5d37ba2900e5ba61e726d7cdd7f0c087a8/redimnet2_b3_lm.onnx) → `models\voice-gate\redimnet2_b3_lm.onnx` | 19 MB | tel quel, au chemin indiqué après la flèche |
| Wake word (PhoneticXeus int8) | facultatif | [model.int8.onnx](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/model.int8.onnx) → `models\wake-word\model.int8.onnx`<br>[vocab.json](https://huggingface.co/allan-valin/wav2vec2phoneme-onnx/resolve/73370fc1a1a47d64886f83e949f96c9ef1619bd4/vocab.json) → `models\wake-word\vocab.json` | 357 MB | tel quel, au chemin indiqué après la flèche |
| RUAccent (stress, «ё», homographs) | recommandé | [model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/model.onnx) → `models\ruaccent\nn\nn_accent\model.onnx`<br>[config.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/config.json) → `models\ruaccent\nn\nn_accent\config.json`<br>[vocab.txt](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_accent/vocab.txt) → `models\ruaccent\nn\nn_accent\vocab.txt`<br>[model.onnx](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/model.onnx) → `models\ruaccent\nn\nn_omograph\tiny2\model.onnx`<br>[tokenizer.json](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/nn/nn_omograph/tiny2/tokenizer.json) → `models\ruaccent\nn\nn_omograph\tiny2\tokenizer.json`<br>[accents.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/accents.json.gz) → `models\ruaccent\dictionary\accents.json.gz`<br>[omographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/omographs.json.gz) → `models\ruaccent\dictionary\omographs.json.gz`<br>[yo_words.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_words.json.gz) → `models\ruaccent\dictionary\yo_words.json.gz`<br>[yo_homographs.json.gz](https://huggingface.co/ruaccent/accentuator/resolve/b78ae5ea1e62beaf138bed1865cd8c3b0b5ca855/dictionary/yo_homographs.json.gz) → `models\ruaccent\dictionary\yo_homographs.json.gz` | 34 MB | tel quel, au chemin indiqué après la flèche |
| RUAccent big_poetry homographs int8 | facultatif | [model.onnx](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/model.onnx) → `models\ruaccent\nn\nn_omograph\big_poetry\model.onnx`<br>[tokenizer.json](https://huggingface.co/nerualdreming/ruaccent-omograph-int8/resolve/d5c4eb6864d55cfc24ae45530eb4ef96843efb2b/big_poetry/tokenizer.json) → `models\ruaccent\nn\nn_omograph\big_poetry\tokenizer.json` | 185 MB | tel quel, au chemin indiqué après la flèche |
| Silero VAD v6.2 | recommandé | [silero-vad-v6.2-F32.gguf](https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/a9b2d61c20ae4ddf3644edb34969e8185a485448/silero-vad-v6.2-F32.gguf) → `models\silero-vad\silero-vad-v6.2-F32.gguf` | 1 MB | tel quel, au chemin indiqué après la flèche |
| VoxLingua107 language identification Q8_0 | obligatoire | [lang-id-voxlingua107-ecapa-Q8_0.gguf](https://huggingface.co/handy-computer/lang-id-voxlingua107-ecapa-gguf/resolve/e59f0f3847e77723b7d58447ef9456fa0c7e9436/lang-id-voxlingua107-ecapa-Q8_0.gguf) → `models\langid-voxlingua107\lang-id-voxlingua107-ecapa-Q8_0.gguf` | 27 MB | tel quel, au chemin indiqué après la flèche |
| DPDFNet 48 kHz (noise suppression) | recommandé | [dpdfnet2_48khz_hr.onnx](https://huggingface.co/Ceva-IP/DPDFNet/resolve/dd6818d00f50c836fed43a6243ebe49116de5964/onnx/dpdfnet2_48khz_hr.onnx) → `models\dpdfnet\dpdfnet2_48khz_hr.onnx` | 10 MB | tel quel, au chemin indiqué après la flèche |
| BSRoformer.cpp (CUDA 13) | recommandé | [BSRoformer-windows-cuda13-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda13-x64.zip) | 119 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\bsroformer\` |
| BSRoformer.cpp (CPU) | facultatif | [BSRoformer-windows-cpu-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cpu-x64.zip) | 3 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\bsroformer-cpu\` |
| BSRoformer.cpp (Vulkan) | facultatif | [BSRoformer-windows-vulkan-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-vulkan-x64.zip) | 26 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\bsroformer-vulkan\` |
| llama.cpp server (CUDA 13) | facultatif | [llama-b11900-bin-win-cuda-13.4-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cuda-13.4-x64.zip) | 165 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\llama\` |
| ONNX Runtime | obligatoire | [onnxruntime-win-x64-1.28.2.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-1.28.2.zip) | 79 MB | décompresser avec son arborescence dans `models\runtime\` |
| ONNX Runtime GPU (CUDA) | recommandé | [onnxruntime-win-x64-gpu_cuda13-1.28.2.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-gpu_cuda13-1.28.2.zip) | 366 MB | décompresser avec son arborescence dans `models\runtime\` |
| FFmpeg (static build) | obligatoire | [ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip](https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-N-127021-ge0c94b2d1c-win64-gpl.zip) | 199 MB | prendre ffmpeg.exe et ffprobe.exe de l'archive et les mettre dans `tools\ffmpeg\` |
| yt-dlp + deno | facultatif | [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.21/yt-dlp.exe) → `tools\yt-dlp\yt-dlp.exe`<br>[deno-x86_64-pc-windows-msvc.zip](https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip) | 61 MB | tel quel, au chemin indiqué après la flèche<br>décompresser les fichiers, sans sous-dossiers, dans `tools\yt-dlp\` |
| CUDA 13 runtime (cudart, cuBLAS, cuFFT) | obligatoire | [nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/86/00/d5436004268f049214193659ebc36550b5ef3925c3d13b4cc980e13be6f5/nvidia_cuda_runtime-13.4.92-py3-none-win_amd64.whl)<br>[nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/a3/df/f1246959833e2c437db8be3e5b477f66b87f8817821ed40de6c7561c9a36/nvidia_cublas-13.8.0.4-py3-none-win_amd64.whl)<br>[libcufft-windows-x86_64-12.4.0.43-archive.zip](https://developer.download.nvidia.com/compute/cuda/redist/libcufft/windows-x86_64/libcufft-windows-x86_64-12.4.0.43-archive.zip) | 586 MB | prendre tous les .dll de l'archive et les mettre dans `models\higgs-engine\` |
| cuDNN 9 | recommandé | [nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/87/6a/e55ff0ac26a5c6e2b21f41c9d04ad096b4ed6da593fba7e25845c61b0532/nvidia_cudnn_cu13-9.27.0.42-py3-none-win_amd64.whl) | 436 MB | prendre tous les .dll de l'archive et les mettre dans `models\higgs-engine\` |
| audio.cpp server, CUDA 12.9 (cards and drivers CUDA 13 left) | facultatif | [audiocpp-server-win-cuda12-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cuda12-x64.zip) | 184 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\audiocpp-cuda12\` |
| audio.cpp server, Vulkan | facultatif | [audiocpp-server-win-vulkan-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-vulkan-x64.zip) | 36 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\audiocpp-vulkan\` |
| audio.cpp server, CPU | facultatif | [audiocpp-server-win-cpu-x64.zip](https://github.com/timoncool/audio.cpp/releases/download/server-2fd98400/audiocpp-server-win-cpu-x64.zip) | 19 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\audiocpp-cpu\` |
| CUDA 12.9 runtime (cudart, cuBLAS, cuFFT) | facultatif | [nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/59/df/e7c3a360be4f7b93cee39271b792669baeb3846c58a4df6dfcf187a7ffab/nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl)<br>[nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/e2/fc9a0e985249d873150276d5afb02e39a66817fedbf1a385724393e505ed/nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl)<br>[nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl](https://files.pythonhosted.org/packages/20/ee/29955203338515b940bd4f60ffdbc073428f25ef9bfbce44c9a066aedc5c/nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl) | 757 MB | prendre tous les .dll de l'archive et les mettre dans `models\cuda12-runtime\` |
| Qwen3-TTS CUDA 13 module | facultatif | [ggml-cuda13-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda13-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 142 MB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS CUDA 12.9 module | facultatif | [ggml-cuda12-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-cuda12-windows-x64.dll) → `models\qwen3-tts\ggml-cuda.dll` | 170 MB | tel quel, au chemin indiqué après la flèche |
| Qwen3-TTS Vulkan module | facultatif | [ggml-vulkan-windows-x64.dll](https://github.com/timoncool/qwentts.cpp/releases/download/engine-0.2.2/ggml-vulkan-windows-x64.dll) → `models\qwen3-tts\ggml-vulkan.dll` | 42 MB | tel quel, au chemin indiqué après la flèche |
| BSRoformer.cpp (CUDA 12.9) | facultatif | [BSRoformer-windows-cuda12-x64.zip](https://github.com/timoncool/BSRoformer.cpp/releases/download/studio-0.1.1/BSRoformer-windows-cuda12-x64.zip) | 173 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\bsroformer-cuda12\` |
| llama.cpp server (CUDA 12.4) | facultatif | [llama-b11900-bin-win-cuda-12.4-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cuda-12.4-x64.zip) | 279 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\llama-cuda12\` |
| llama.cpp server (Vulkan) | facultatif | [llama-b11900-bin-win-vulkan-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-vulkan-x64.zip) | 37 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\llama-vulkan\` |
| llama.cpp server (CPU) | facultatif | [llama-b11900-bin-win-cpu-x64.zip](https://github.com/ggml-org/llama.cpp/releases/download/b11900/llama-b11900-bin-win-cpu-x64.zip) | 24 MB | décompresser les fichiers, sans sous-dossiers, dans `tools\llama-cpu\` |
<!-- downloads:end -->

## Ce qui tourne où

Chaque étape a son propre appareil dans Réglages → Modèles : Auto, GPU (CUDA), Vulkan ou CPU. Auto prend le build CUDA de la carte : CUDA 13, ou CUDA 12.9 sur une carte ou un pilote plus anciens.

| Étape | Moteur | NVIDIA (CUDA) | Vulkan (AMD, Intel) | CPU | Mac (Apple silicon) | Cloud |
|---|---|---|---|---|---|---|
| Voix et clonage | Higgs Audio v3 (audio.cpp) | oui | oui | oui | Metal | OpenRouter, Gemini, fal.ai, Runware |
| Voix et clonage | Qwen3-TTS (qwentts.cpp) | oui | oui | oui | Metal | — |
| Voix et clonage | Audio8 TTS (audio.cpp) | oui | oui | oui | Metal | — |
| Voix et clonage | Fish Audio Drama 3 (drama-rs) | CUDA 13 seulement | non | non | non | — |
| Voix | Supertonic 3 (audio.cpp) | oui | oui | oui | Metal | — |
| Reconnaissance vocale | Parakeet-TDT v3, Nemotron 3.5 (audio.cpp) | oui | oui | oui | Metal | OpenRouter, fal.ai |
| Reconnaissance vocale | Parakeet Ultra (transcribe.cpp) | CUDA 13 | oui | oui | Metal | — |
| Reconnaissance vocale | CrisperWhisper 2.0 turbo (audio.cpp) | oui | non | oui | Metal | — |
| Locuteurs | Nemotron 3 Diarization (audio.cpp) | oui | oui | oui | Metal | non |
| Pauses et bruit | Silero VAD v6 (transcribe.cpp), DPDFNet | non | non | oui | CPU | non |
| Voix séparée de la musique | Mel-Band Roformer (BSRoformer.cpp) | oui | oui | oui | Metal | non |
| Accents et homographes | RUAccent | non | non | oui | CPU | non |
| Réalisateur (podcast, personnages) | Gemma-4 12B (llama.cpp) | oui | oui | oui | Metal | OpenRouter |

## Dépannage

**L’installateur s’arrête sur WebView2.** Installez WebView2 avec l’installateur [Evergreen Standalone x64](https://go.microsoft.com/fwlink/p/?LinkId=2124701) de Microsoft, puis relancez l’installateur de Speech Studio.

**Les téléchargements bloquent ou échouent.** Les gros fichiers reprennent là où ils s’étaient arrêtés : cliquez de nouveau. Si Hugging Face ou GitHub sont bloqués chez vous, réglez un proxy dans Paramètres → **Réseau**, ou placez les fichiers du tableau et cliquez sur **Importer d’un dossier**.

**Un moteur de voix parle d’une carte ou d’un pilote trop anciens.** Les builds CUDA 13 demandent une carte à partir de Turing (GTX 16, RTX 20 et plus récentes) et le pilote 580 ou plus récent ; GTX 900, GTX 10, Volta et les pilotes 525–579 reçoivent d’eux-mêmes les builds CUDA 12. Drama 3 n’a qu’un build CUDA 13 : sur une carte ou un pilote plus anciens, mettez à jour le pilote ou prenez un autre moteur ou les voix cloud.

**La reconnaissance ne trouve pas de parole.** L’enregistrement est sous le seuil de parole : baissez-le dans **Reconnaissance → Découpe aux pauses**.

**Paramètres → Diagnostic** rassemble un paquet pour rapport de bug sans clés : joignez-le à une [issue](https://github.com/timoncool/speech-studio/issues).

## Comment ça marche

**Stack :** un shell natif **Tauri 2 (Rust)** lance `speech-server` (axum) dans le même processus sur `127.0.0.1:8797` et ouvre une fenêtre sur la SPA — React 19 + Vite + Tailwind. Les moteurs vivent dans le processus via leurs ABI C : Drama 3 (`drama_engine.dll`, [timoncool/drama-rs](https://github.com/timoncool/drama-rs)), Qwen3-TTS (`qwen.dll`, [timoncool/qwentts.cpp](https://github.com/timoncool/qwentts.cpp)) ; Parakeet Ultra, Silero VAD et VoxLingua107 (`transcribe_engine.dll`, [timoncool/transcribe.cpp](https://github.com/timoncool/transcribe.cpp)) ; DPDFNet, RUAccent et WeSpeaker sur ONNX Runtime. Higgs Audio v3, Audio8 TTS, Supertonic 3, Parakeet-TDT v3, Nemotron 3.5, CrisperWhisper et Nemotron 3 Diarization tournent dans un programme auxiliaire, le serveur audio.cpp (`audiocpp_server`, [timoncool/audio.cpp](https://github.com/timoncool/audio.cpp)) ; BSRoformer.cpp et le serveur de llama.cpp sont aussi des auxiliaires ; ffmpeg pour l’audio. **Aucun processus Python à l’exécution.**

### Compiler depuis les sources

```bash
git clone https://github.com/timoncool/speech-studio.git
cd speech-studio

cd frontend && npm install && npm run build && cd ..   # 1) SPA
cargo build --release -p speech-server                  # 2) serveur natif (axum)
cd desktop && npm install && npx tauri build            # 3) shell de bureau (Tauri)
```

Il faut Node 20+, Rust (chaîne MSVC) et WebView2. Les moteurs n’ont pas besoin d’être recompilés : l’application télécharge des binaires prêts. Les paquets de la version sortent de `scripts/build-release.ps1` (Windows), `scripts/build-linux-docker.sh` (Linux, dans un conteneur Ubuntu 22.04) et `scripts/build-release-macos.sh` (macOS, sur un Mac Apple silicon).

### Linux (expérimental)

Le .deb et l’AppImage pour Linux x86-64 sont **expérimentaux** : le même code avec les builds Linux des mêmes moteurs, non testés par l’auteur sur un vrai bureau Linux. Les corrections sont les bienvenues en pull request. Le pilote NVIDIA (580 ou plus récent pour les builds CUDA 13, 525 ou plus récent pour les builds CUDA 12), `libgomp1` et `libssl3` viennent du système ; modèles et moteurs vont dans `~/.local/share/speech-studio`.

### macOS (expérimental)

Le .dmg pour macOS 13 ou plus récent sur Apple silicon est lui aussi **expérimental** : les builds macOS des mêmes moteurs sur Metal, non testés par l’auteur sur un vrai Mac. Drama 3 n’y est pas proposé, il ne tourne que sur les cartes NVIDIA. L’application est signée ad hoc et non notariée, donc macOS bloque le premier lancement : ouvrez-la une fois depuis Réglages Système → Confidentialité et sécurité → **Ouvrir quand même**. Modèles et moteurs vont dans `~/Library/Application Support/Speech Studio`.

## Statistiques anonymes et actualités

L’application demande les actualités au serveur de l’auteur au démarrage puis toutes les heures ; la requête ne porte aucun identifiant. L’écran du premier lancement a une case **Envoyer des statistiques d’utilisation anonymes**, cochée par défaut ; le même interrupteur est dans les Paramètres, à côté de **Ce qui est envoyé**. Tant qu’elle est cochée, l’application envoie le rapport du jour : un identifiant d’installation aléatoire créé sur cet ordinateur, la version, l’OS, la langue de la fenêtre, la carte graphique comme constructeur, une tranche de mémoire vidéo et si CUDA fonctionne, combien de tâches ont abouti, échoué ou été annulées, les modèles de chaque étape, les secondes de voix produites par les moteurs locaux et les secondes de travail qu’elles leur ont demandé, et pourquoi une tâche a échoué, en une ligne sans chemins, noms ni texte entre guillemets (pour un téléchargement échoué, le composant et le site d’où il venait). Jamais : textes, enregistrements, voix, noms de fichiers ni rien de personnel. `DO_NOT_TRACK=1` ou `STUDIO_TELEMETRY=0` coupent entièrement les statistiques.

## Autres applications IA portables

| Projet | Ce que c’est |
|--------|----------|
| [Dub Studio](https://github.com/timoncool/dub-studio) | Doublage vidéo par IA : voix clonées, sous-titres traduits et texte à l’écran |
| [Higgs Ultimate](https://github.com/timoncool/Higgs-Ultimate) | Synthèse vocale et clonage de voix natifs (Higgs Audio v3) |
| [ACE-Step Studio](https://github.com/timoncool/ACE-Step-Studio) | Studio de musique IA — chansons, voix, reprises, clips |
| [YuE2 Studio](https://github.com/timoncool/YuE2-Studio) | Générateur de chansons avec partition modifiable |
| [MiniMax Music3 Studio](https://github.com/timoncool/MiniMax-Music3-Studio) | Studio de musique natif local/cloud avec MiniMax Music 3 |
| [VideoSOS](https://github.com/timoncool/videosos) | Studio vidéo IA |

## Contribuer et forks

Les contributeurs sont les bienvenus : corrections des builds **macOS et Linux**, **cartes AMD et Intel** sur Vulkan et nouvelles langues d’interface (`frontend/src/locales/`, `crates/speech-server/locales/`) — forkez et envoyez une pull request.

## Auteurs

- **Nerual Dreming** — [Telegram](https://t.me/nerual_dreming) | [neuro-cartel.com](https://neuro-cartel.com) | fondateur d’[ArtGeneration.me](https://artgeneration.me)
- **Neuro-Soft** — [Telegram](https://t.me/neuroport) | applications IA portables

## Remerciements

- **[Boson AI](https://huggingface.co/bosonai)** — Higgs Audio v3, et **[drbaph / Higgs-Audio-v3-Studio](https://huggingface.co/drbaph/Higgs-Audio-v3-Studio)** — ses quantifications GGUF.
- **[Fish Audio](https://fish.audio)** — Drama 3 ; **[apiplant/drama-q4](https://huggingface.co/apiplant/drama-q4)** — ses poids Q4 ; **[drama-rs](https://github.com/timoncool/drama-rs)** — le moteur Rust/CUDA.
- **[Équipe Qwen](https://huggingface.co/Qwen)** — Qwen3-TTS ; **[ServeurpersoCom / qwentts.cpp](https://github.com/ServeurpersoCom/qwentts.cpp)** et **[Serveurperso/Qwen3-TTS-GGUF](https://huggingface.co/Serveurperso/Qwen3-TTS-GGUF)** — le moteur C++ et les quantifications GGUF ; **[siriusfreak](https://huggingface.co/siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf)** — la LoRA d’accent tonique russe.
- **[Edge0](https://huggingface.co/Edge0/Audio8-TTS-Realistic-Air)** — Audio8 TTS Realistic-Air ; **[Hansda/Audio8-TTS-Realistic-Air-GGUF](https://huggingface.co/Hansda/Audio8-TTS-Realistic-Air-GGUF)** — ses quantifications GGUF.
- **[Supertone](https://huggingface.co/Supertone)** — Supertonic 3 ; **[askurios8/supertonic-3-int8](https://huggingface.co/askurios8/supertonic-3-int8)** — son export int8.
- **[RUAccent](https://huggingface.co/ruaccent)** — accents toniques, « ё » et homographes.
- **[NVIDIA](https://huggingface.co/nvidia)** — Parakeet-TDT, Nemotron 3.5 ASR et Nemotron 3 Diarization ; l’ONNX de la diarisation vient de [altunenes/parakeet-rs](https://github.com/altunenes/parakeet-rs).
- **[handy-computer / transcribe.cpp](https://github.com/handy-computer/transcribe.cpp)** — le moteur transcribe.cpp et les GGUF de Parakeet-TDT, Parakeet Ultra, Nemotron 3.5, Silero VAD v6.2 et VoxLingua107.
- **[Silero](https://github.com/snakers4/silero-vad)** — Silero VAD v6 ; **[Ceva-IP / DPDFNet](https://huggingface.co/Ceva-IP/DPDFNet)** — réduction de bruit.
- **[nyra health / CrisperWhisper 2.0 turbo](https://huggingface.co/nyralabs/CrisperWhisper2.0_turbo)** — reconnaissance Whisper avec le temps des mots ; **[0xShug0 / audio.cpp](https://github.com/0xShug0/audio.cpp)** — le moteur où tournent Higgs Audio v3, Audio8 TTS, Supertonic 3, Parakeet-TDT v3, Nemotron 3.5, CrisperWhisper et Nemotron 3 Diarization.
- **[chenmozhijin / BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp)** et **[GaboxR67](https://huggingface.co/GaboxR67)** — séparation de la voix.
- **[WeSpeaker](https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM)** — ressemblance de voix ; **[Google Gemma](https://huggingface.co/google)** et **[llama.cpp](https://github.com/ggml-org/llama.cpp)** — le réalisateur.
- **[ONNX Runtime](https://github.com/microsoft/onnxruntime)**, **[FFmpeg](https://ffmpeg.org)**, **[Tauri](https://tauri.app)**, **[ort](https://github.com/pykeio/ort)** et les bibliothèques NVIDIA CUDA.

## Soutenir l’auteur

Je fais du logiciel libre et de la recherche en IA — presque tout ce que je crée est gratuit. Les dons me permettent de créer et de chercher davantage.

**[Toutes les façons de soutenir](DONATE.md)** | **[dalink.to/nerual_dreming](https://dalink.to/nerual_dreming)** | **[boosty.to/neuro_art](https://boosty.to/neuro_art)**

- **BTC :** `1E7dHL22RpyhJGVpcvKdbyZgksSYkYeEBC`
- **ETH (ERC20) :** `0xb5db65adf478983186d4897ba92fe2c25c594a0c`
- **USDT (TRC20) :** `TQST9Lp2TjK6FiVkn4fwfGUee7NmkxEE7C`

## Historique des étoiles

<a href="https://github.com/timoncool/speech-studio/stargazers">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="docs/stars-dark.svg" />
   <source media="(prefers-color-scheme: light)" srcset="docs/stars-light.svg" />
   <img alt="Star History Chart" src="docs/stars-light.svg" />
 </picture>
</a>

## Licence

Le code de l’application est sous [MIT](LICENSE). **Les modèles non** : chacun garde sa licence. Lisez-les avant de publier ou de vendre ce que vous doublez.

| Composant | Licence | Ce que cela veut dire |
|---|---|---|
| Higgs Audio v3 (Boson AI) | [Boson Higgs TTS 3 Research and Non-Commercial License](https://huggingface.co/bosonai/higgs-tts-3-4b/blob/main/LICENSE) | Recherche et usage personnel ; les créateurs peuvent monétiser leur propre contenu avec le Creator Use Grant s’ils citent Higgs Audio de Boson AI ; un produit ou service pour autrui exige une licence de Boson |
| Fish Audio Drama 3 | [Fish Audio Community License](https://huggingface.co/fishaudio/drama-3/blob/main/LICENSE) | Recherche et usage non commercial ; usage commercial tant que votre chiffre d’affaires annuel reste sous 500 000 USD, au-delà une licence de Fish Audio ; pas de service hébergé de synthèse vocale |
| Qwen3-TTS (Qwen) | Apache-2.0 |  |
| LoRA d’accent russe de Qwen3-TTS (siriusfreak) | Apache-2.0 |  |
| Audio8 TTS Realistic-Air (Edge0) | Apache-2.0 |  |
| Supertonic 3 (Supertone) | BigScience OpenRAIL-M | Lisez ses restrictions d’usage |
| RUAccent et son modèle d’homographes en int8 | MIT |  |
| Parakeet-TDT, Nemotron 3 Diarization (NVIDIA) | CC-BY-4.0, OpenMDW-1.1 | Citez NVIDIA |
| Nemotron 3.5 ASR (NVIDIA) | OpenMDW-1.1 | Citez NVIDIA |
| transcribe.cpp | MIT |  |
| Silero VAD | MIT |  |
| DPDFNet (Ceva) | Apache-2.0 |  |
| CrisperWhisper 2.0 turbo (nyra health) | MIT pour le code ; nyra health Non-Commercial Research License pour les poids et ce qu’ils produisent | Usage non commercial uniquement ; l’usage commercial demande une licence de nyra health |
| audio.cpp | Apache-2.0 |  |
| Mel-Band Roformer voc_fv6, GGUF | Aucune licence indiquée sur les pages des modèles | Demandez aux auteurs avant un usage commercial |
| WeSpeaker ResNet34-LM | CC-BY-4.0 | Citez WeSpeaker |
| Gemma-4 12B (Google) | Conditions de Gemma | Lisez-les |
| llama.cpp, ONNX Runtime, BSRoformer.cpp, qwentts.cpp | MIT |  |
| ElevenLabs UI Orb, SiriWave | MIT |  |
| LiveKit Agents UI Aura | Apache-2.0 |  |
| FFmpeg (build BtbN) | Build GPL | Programme séparé, non lié à l’application |
| NVIDIA CUDA runtime, cuBLAS, cuFFT, cuDNN | Conditions propres à NVIDIA | Téléchargés depuis NVIDIA et PyPI |

<sub>synthèse vocale · clonage de voix · TTS russe · accents toniques · création de livres audio · podcast · reconnaissance vocale · transcription · diarisation · hors ligne · local · open source · Windows · macOS · Linux</sub>

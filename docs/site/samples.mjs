// The examples on the project page, as the studio exported them (speak_text of the installed studio, each checked back by
// recognition). GROUPS: one text per language in two actors' voices, a row each, on every engine; Supertonic does not
// clone and reads with its nearest built-in voice. STRESS: the same Russian text without and with the studio's stress marks.
export const ENGINES = {
  "higgs": "Higgs Audio v3",
  "drama3": "Fish Audio Drama 3",
  "qwen3-tts": "Qwen3-TTS",
  "qwen3-tts-ru": "Qwen3-TTS + ru stress LoRA",
  "audio8": "Audio8 TTS",
  "supertonic": "Supertonic 3"
};

const NAMES = {
  "RU_Male_Gabidullin_ruslan": {
    "ru": "Руслан Габидуллин",
    "zh": "鲁斯兰·加比杜林",
    "*": "Ruslan Gabidullin"
  },
  "RU_Female_IngridOlerinskaya_Judy": {
    "ru": "Ингрид Олеринская",
    "zh": "英格丽德·奥列林斯卡娅",
    "*": "Ingrid Olerinskaya"
  },
  "RU_Male_Goblin_Puchkov": {
    "ru": "Дмитрий Пучков (Гоблин)",
    "*": "Dmitry Puchkov (Goblin)"
  },
  "RU_Male_Livanov_SherlockHolmes": {
    "ru": "Василий Ливанов (Шерлок Холмс)",
    "*": "Vasily Livanov (Sherlock Holmes)"
  },
  "RU_Male_Nagiev": {
    "ru": "Дмитрий Нагиев",
    "*": "Dmitry Nagiev"
  },
  "": {
    "en": "the model's own voice",
    "ru": "собственный голос модели",
    "es": "voz propia del modelo",
    "fr": "voix propre du modèle",
    "pt": "voz própria do modelo",
    "zh": "模型自带声音"
  }
};

/** A voice as a reader knows it: a known person's name in the page's language, a pack voice by its words, a preset by its name. */
export function voiceName(voice, lang) {
  const known = NAMES[voice];
  if (known) return known[lang] ?? known['*'] ?? known.en;
  return voice.replace(/_/g, ' ').replace(/([a-z]{2,})(\d)/gi, '$1 $2').replace(/\b([a-z])/g, (c) => c.toUpperCase());
}

export const GROUPS = [
  {
    "lang": "ru",
    "key": "pushkin",
    "text": "Я помню чудное мгновенье — передо мной явилась ты, как мимолётное виденье, как гений чистой красоты.",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "ru-pushkin-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-qwen3-tts-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-qwen3-tts-ru-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts-ru",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-supertonic-m1.mp3",
            "engine": "supertonic",
            "voice": "M1",
            "kind": "preset"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "ru-pushkin-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-drama3-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "drama3",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-qwen3-tts-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-qwen3-tts-ru-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts-ru",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "ru-pushkin-supertonic-f2.mp3",
            "engine": "supertonic",
            "voice": "F2",
            "kind": "preset"
          }
        ]
      }
    ]
  },
  {
    "lang": "en",
    "key": "sonnet18",
    "text": "Shall I compare thee to a summer's day? Thou art more lovely and more temperate: rough winds do shake the darling buds of May, and summer's lease hath all too short a date.",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "en-sonnet18-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-qwen3-tts-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-supertonic-m1.mp3",
            "engine": "supertonic",
            "voice": "M1",
            "kind": "preset"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "en-sonnet18-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-drama3-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "drama3",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-qwen3-tts-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "en-sonnet18-supertonic-f2.mp3",
            "engine": "supertonic",
            "voice": "F2",
            "kind": "preset"
          }
        ]
      }
    ]
  },
  {
    "lang": "es",
    "key": "becquer",
    "text": "¿Qué es poesía?, dices mientras clavas en mi pupila tu pupila azul. ¿Qué es poesía? ¿Y tú me lo preguntas? Poesía… eres tú.",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "es-becquer-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "es-becquer-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "es-becquer-qwen3-tts-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "es-becquer-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "es-becquer-supertonic-m1.mp3",
            "engine": "supertonic",
            "voice": "M1",
            "kind": "preset"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "es-becquer-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "es-becquer-drama3-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "drama3",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "es-becquer-qwen3-tts-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "es-becquer-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "es-becquer-supertonic-f2.mp3",
            "engine": "supertonic",
            "voice": "F2",
            "kind": "preset"
          }
        ]
      }
    ]
  },
  {
    "lang": "fr",
    "key": "verlaine",
    "text": "Il pleure dans mon cœur comme il pleut sur la ville ; quelle est cette langueur qui pénètre mon cœur ?",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "fr-verlaine-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-qwen3-tts-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-supertonic-m1.mp3",
            "engine": "supertonic",
            "voice": "M1",
            "kind": "preset"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "fr-verlaine-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-drama3-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "drama3",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-qwen3-tts-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "fr-verlaine-supertonic-f2.mp3",
            "engine": "supertonic",
            "voice": "F2",
            "kind": "preset"
          }
        ]
      }
    ]
  },
  {
    "lang": "pt",
    "key": "camoes",
    "text": "Amor é fogo que arde sem se ver, é ferida que dói, e não se sente; é um contentamento descontente, é dor que desatina sem doer.",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "pt-camoes-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-qwen3-tts-ru-male-gabidullin-ruslan.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-supertonic-m1.mp3",
            "engine": "supertonic",
            "voice": "M1",
            "kind": "preset"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "pt-camoes-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-drama3-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "drama3",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-qwen3-tts-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "qwen3-tts",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "pt-camoes-supertonic-f2.mp3",
            "engine": "supertonic",
            "voice": "F2",
            "kind": "preset"
          }
        ]
      }
    ]
  },
  {
    "lang": "zh",
    "key": "libai",
    "text": "床前明月光，疑是地上霜。举头望明月，低头思故乡。",
    "rows": [
      {
        "actor": "RU_Male_Gabidullin_ruslan",
        "clips": [
          {
            "file": "zh-libai-higgs-ru-male-gabidullin-ruslan.mp3",
            "engine": "higgs",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "zh-libai-drama3-ru-male-gabidullin-ruslan.mp3",
            "engine": "drama3",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          },
          {
            "file": "zh-libai-audio8-ru-male-gabidullin-ruslan.mp3",
            "engine": "audio8",
            "voice": "RU_Male_Gabidullin_ruslan",
            "kind": "clone"
          }
        ]
      },
      {
        "actor": "RU_Female_IngridOlerinskaya_Judy",
        "clips": [
          {
            "file": "zh-libai-higgs-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "higgs",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          },
          {
            "file": "zh-libai-audio8-ru-female-ingridolerinskaya-judy.mp3",
            "engine": "audio8",
            "voice": "RU_Female_IngridOlerinskaya_Judy",
            "kind": "clone"
          }
        ]
      }
    ],
    "note": "qwenZhNote"
  }
];

export const STRESS = [
  {
    "key": "homographs",
    "text": "Старинный замок стоял на холме, а дверь в него запирали на тяжёлый замок. Из этой муки пекут хлеб, но для пекаря это настоящие муки.",
    "pairs": [
      {
        "engine": "supertonic",
        "voice": "F3",
        "kind": "preset",
        "before": "ru-homographs-supertonic-before.mp3",
        "after": "ru-homographs-supertonic-after.mp3"
      },
      {
        "engine": "qwen3-tts-ru",
        "voice": "RU_Male_Nagiev",
        "kind": "clone",
        "before": "ru-homographs-qwen3-tts-ru-before.mp3",
        "after": "ru-homographs-qwen3-tts-ru-after.mp3"
      }
    ]
  },
  {
    "key": "homographs2",
    "text": "Мы долго стояли у берега и смотрели, как белки прыгают по веткам. В яйце много белка, а в этой задаче много лишних белков.",
    "pairs": [
      {
        "engine": "supertonic",
        "voice": "F3",
        "kind": "preset",
        "before": "ru-homographs2-supertonic-before.mp3",
        "after": "ru-homographs2-supertonic-after.mp3"
      },
      {
        "engine": "qwen3-tts-ru",
        "voice": "RU_Male_Nagiev",
        "kind": "clone",
        "before": "ru-homographs2-qwen3-tts-ru-before.mp3",
        "after": "ru-homographs2-qwen3-tts-ru-after.mp3"
      }
    ]
  }
];

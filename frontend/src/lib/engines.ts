import { useEffect, useState } from "react";
import { speechApi, type EngineCaps } from "./speech";
import { libraryVoiceName } from "./voices";

/** Engines that voice through a provider's servers: languages and voices are the chosen model's. */
export const CLOUD_ENGINES: readonly string[] = ["openrouter", "google", "fal", "runware"];

let cache: EngineCaps[] | null = null;
let loading: Promise<EngineCaps[]> | null = null;

function load(): Promise<EngineCaps[]> {
  loading ??= speechApi.engines().then(
    (r) => { cache = r.engines; loading = null; return r.engines; },
    (e: unknown) => { loading = null; throw e; },
  );
  return loading;
}

/** The engines' manifests (languages, parameters, own voices, why one is unavailable): the last answer at once, read
 * again whenever a picker opens, since a download in Models can make an engine available. */
export function useEngines(): EngineCaps[] | null {
  const [engines, setEngines] = useState<EngineCaps[] | null>(cache);
  useEffect(() => {
    let mounted = true;
    load().then((e) => { if (mounted) setEngines(e); }, () => undefined);
    return () => { mounted = false; };
  }, []);
  return engines;
}

/** A voice as people read it: the engine's name for one of its own voices (`engine` is an engine id, or a take's
 * engine tag such as `qwen3-tts:Q4_K_M`), else the library voice's pretty name. */
export function voiceName(voice: string, engines: EngineCaps[] | null, engine?: string): string {
  const id = engine?.split(":")[0];
  return engines?.find((e) => e.id === id)?.presets.find((p) => p.id === voice)?.name ?? libraryVoiceName(voice);
}

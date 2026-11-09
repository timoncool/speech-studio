import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ROLE_CHANGED, useChanged } from "../../lib/mcpBridge";
import { talkApi, type LiveRole, type LiveRoles } from "../../lib/talk";
import { errorText } from "../../lib/voices";
import Hint from "../Hint";

/** The role of the Live conversation: the person picks one of the studio's folder, and sees the one the agent took. */
export default function RolePicker() {
  const { t, i18n } = useTranslation();
  const [roles, setRoles] = useState<LiveRoles | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  const load = useCallback(() => {
    talkApi.roles().then((r) => { setRoles(r); setError(null); }, (e: unknown) => setError(errorText(e)));
  }, []);
  useEffect(load, [load, i18n.language]);
  useChanged(ROLE_CHANGED, load);

  const playing = roles?.playing ?? null;
  const listed = roles?.roles.find((r) => r.id === playing?.id);
  const pick = (id: string) => {
    talkApi.setRole(id || null).then(load, (e: unknown) => setError(errorText(e)));
  };
  const about: LiveRole | null = playing;
  const lastGoal = listed?.last_review?.goal;

  return (
    <div className="w-full max-w-3xl shrink-0 space-y-1.5 text-[12px]">
      <div className="flex items-center gap-2">
        <label className="flex items-center gap-2 min-w-0">
          <span className="shrink-0 text-[var(--color-muted)]">{t("talk.role")} <Hint text={t("talk.roleHint")} /></span>
          <select value={playing?.id ?? ""} onChange={(e) => pick(e.target.value)} className="min-w-0 max-w-[16rem] bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[12px]">
            <option value="">{t("talk.roleNone")}</option>
            {playing && !listed && <option value={playing.id}>{playing.name}</option>}
            {roles?.roles.map((r) => <option key={r.id} value={r.id} disabled={!!r.problem} title={r.problem ? t("talk.roleBroken", { problem: r.problem }) : r.goal ?? undefined}>{r.name}</option>)}
          </select>
        </label>
        {about?.goal && <p className="min-w-0 flex-1 truncate text-[var(--color-muted)]" title={about.goal}>{about.goal}</p>}
        {about && (
          <button onClick={() => setOpen((v) => !v)} aria-expanded={open} className="shrink-0 rounded-md border border-[var(--color-border)] px-2 py-1 hover:border-[var(--color-accent)]">{t("talk.roleDetails")}</button>
        )}
      </div>
      {about && open && (
        <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-3 space-y-2">
          <p className="text-[var(--color-muted)]">
            {t("talk.roleDifficulty", { n: about.difficulty ?? 1 })} · {t("talk.roleTurns", { n: about.turns_taken ?? 0, of: about.turns ?? 0 })}
          </p>
          {about.opens && <p><span className="text-[var(--color-muted)]">{t("talk.roleOpens")}: </span>{about.opens}</p>}
          {!!about.checks?.length && (
            <div>
              <p className="text-[var(--color-muted)]">{t("talk.roleChecks")}</p>
              <ul className="list-disc pl-4">{about.checks.map((c) => <li key={c}>{c}</li>)}</ul>
            </div>
          )}
          {!!about.notes?.length && (
            <div>
              <p className="text-[var(--color-muted)]">{t("talk.roleNotes")}</p>
              <ul className="list-disc pl-4">{about.notes.map((n) => <li key={n}>{n}</li>)}</ul>
            </div>
          )}
          {lastGoal && <p>{t("talk.roleLastGoal", { goal: lastGoal })}</p>}
        </div>
      )}
      {error && <p role="alert" className="text-[var(--color-warn)]">{error}</p>}
    </div>
  );
}

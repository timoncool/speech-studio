import { flash } from "../../lib/mcpBridge";

/** From this many blocks a project draws only the rows near the screen. */
export const LAZY_FROM = 200;

export const rowId = (id: string) => `block-${id}`;

/** The attribute of the area the block list scrolls in. */
export const SCROLLER = "data-block-scroll";

const ROW_BASE = 22;
const ROW_LINE = 21;
/** Letters a line of a row holds, between what the drawn rows have shown: the most a line was seen to hold and the
 * least that was seen to need another line. */
let lineHolds = 60;
let lineBreaksAt = Infinity;

/** A drawn row teaches how many letters fit a line at this width. */
export function learnRow(chars: number, height: number) {
  const lines = Math.max(1, Math.round((height - ROW_BASE) / ROW_LINE));
  if (lines > 6 || chars < 20) return;
  lineHolds = Math.max(lineHolds, chars / lines);
  if (lines > 1) lineBreaksAt = Math.min(lineBreaksAt, chars / (lines - 1));
}

/** A row's height before it is drawn, by how much text it holds. */
export function guessRow(chars: number): number {
  const perLine = Number.isFinite(lineBreaksAt) && lineBreaksAt > lineHolds ? (lineHolds + lineBreaksAt) / 2 : lineHolds * 1.2;
  return ROW_BASE + Math.max(1, Math.ceil(chars / perLine)) * ROW_LINE;
}

/** Brings a block's row into view, marks it and, with `focus`, puts the cursor in its text — once the row is drawn;
 * the rows drawn around it may differ from their guessed heights, so the row is brought to the middle again then. */
export function showBlockRow(id: string, how: { smooth?: boolean; focus?: boolean } = {}) {
  document.getElementById(rowId(id))?.scrollIntoView({ block: "center", behavior: how.smooth ? "smooth" : "auto" });
  let tries = 0;
  const settle = () => {
    const row = document.getElementById(rowId(id));
    const field = row?.querySelector("textarea");
    if (!field && tries++ < 40) {
      requestAnimationFrame(settle);
      return;
    }
    if (!how.smooth) row?.scrollIntoView({ block: "center" });
    flash(row);
    if (how.focus) field?.focus();
  };
  requestAnimationFrame(settle);
}

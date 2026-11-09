import { memo, useEffect, useState, type ReactNode } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { guessRow, learnRow, rowId, SCROLLER } from "./blockRows";

/** How far past the screen's edge a row is still drawn, so scrolling meets rows already there. */
const MARGIN = "700px 0px";

type Seen = (near: boolean, element: Element) => void;
const watched = new Map<Element, Seen>();
const observers = new Map<Element | null, IntersectionObserver>();

/** The margin counts from the list's own scrolling area: a row its area clips is out of sight whatever the window's
 * margin is. */
function watch(element: Element, seen: Seen): () => void {
  const root = element.closest(`[${SCROLLER}]`);
  let observer = observers.get(root);
  if (!observer) {
    observer = new IntersectionObserver((entries) => {
      for (const e of entries) watched.get(e.target)?.(e.isIntersecting, e.target);
    }, { root, rootMargin: MARGIN });
    observers.set(root, observer);
  }
  watched.set(element, seen);
  observer.observe(element);
  return () => {
    watched.delete(element);
    observer.unobserve(element);
  };
}

/** A block's place in a long project: the row itself while it is near the screen or in use (`keep`), an empty row of
 * the same height and id otherwise, so a book of thousands of blocks draws and redraws only what is seen. */
export default function LazyBlock({ id, chars, keep, label, children }: { id: string; chars: number; keep: boolean; label: string; children: () => ReactNode }) {
  const [near, setNear] = useState(false);
  const [height, setHeight] = useState(0);
  const shown = near || keep;
  useEffect(() => {
    const el = document.getElementById(rowId(id));
    if (!el) return;
    if (shown && !keep) learnRow(chars, el.offsetHeight);
    return watch(el, (isNear, element) => {
      if (!isNear) {
        if (element.contains(document.activeElement)) return;
        setHeight((element as HTMLElement).offsetHeight);
      }
      setNear(isNear);
    });
  }, [id, shown, keep, chars]);
  if (shown) return children();
  return <Place id={id} label={label} height={height || guessRow(chars)} />;
}

/** The empty row: it still takes part in reordering, so a block dragged past it knows where it is. */
const Place = memo(function Place({ id, label, height }: { id: string; label: string; height: number }) {
  const { setNodeRef, transform, transition } = useSortable({ id });
  return <li id={rowId(id)} ref={setNodeRef} aria-label={label} style={{ height, transform: CSS.Transform.toString(transform), transition }} />;
});

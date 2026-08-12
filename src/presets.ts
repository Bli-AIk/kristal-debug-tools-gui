// Pure helpers for the chapter-preset page: which preset the current
// content matches, and loading a preset into the staged edits.

export interface PresetItem {
  key: string;
  current: { label: string; value: unknown };
  chValues?: Record<string, { label: string; value: unknown }>;
}

export type Edits = Record<string, unknown>;

/** Effective value of an item: a staged edit wins over the applied one. */
export function effValue(item: PresetItem, edits: Edits): unknown {
  return edits[item.key] !== undefined ? edits[item.key] : item.current.value;
}

/**
 * Which chapter preset the content (edits + applied values) equals.
 * A chapter is a match when every item that HAS a preset value for it
 * equals that value; items without a value for the chapter are skipped.
 * Returns 0 when no chapter matches (★ custom).
 */
export function matchingChapter(items: PresetItem[], edits: Edits): number {
  for (let ch = 1; ch <= 4; ch++) {
    const relevant = items.filter((it) => it.chValues?.[String(ch)] !== undefined);
    if (relevant.length === 0) continue;
    if (relevant.every((it) => String(effValue(it, edits)) === String(it.chValues![String(ch)].value))) {
      return ch;
    }
  }
  return 0;
}

/** Stage every preset value of chapter `ch` as edits (missing ones skipped). */
export function applyPresetValues(items: PresetItem[], ch: number): Edits {
  const next: Edits = {};
  for (const it of items) {
    if (it.chValues?.[String(ch)]) next[it.key] = it.chValues[String(ch)].value;
  }
  return next;
}

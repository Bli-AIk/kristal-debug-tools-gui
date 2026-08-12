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
 * Whether the content equals chapter `ch`'s preset. A chapter matches
 * when every item that HAS a preset value for it equals that value;
 * items without a value for the chapter are skipped.
 */
export function matchesChapter(items: PresetItem[], edits: Edits, ch: number): boolean {
  const relevant = items.filter((it) => it.chValues?.[String(ch)] !== undefined);
  if (relevant.length === 0) return false;
  return relevant.every((it) => String(effValue(it, edits)) === String(it.chValues![String(ch)].value));
}

/**
 * Which chapter preset the content (edits + applied values) equals.
 * Note: many items are identical across chapters, so several chapters
 * may match — this returns the first one. The UI prefers an explicit
 * chapter pick for that reason.
 * Returns 0 when no chapter matches (★ custom).
 */
export function matchingChapter(items: PresetItem[], edits: Edits): number {
  for (let ch = 1; ch <= 4; ch++) {
    if (matchesChapter(items, edits, ch)) return ch;
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

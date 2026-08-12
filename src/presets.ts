// Pure state helpers for the chapter page. Kristal uses `chapter` as the
// baseline and only treats config.kristal entries as overrides; never turn a
// chapter's defaults into a large set of explicit config values.

export interface PresetValue {
  label: string;
  value: unknown;
}

export interface PresetItem {
  key: string;
  current: PresetValue;
  chValues?: Record<string, PresetValue>;
  isOverride?: boolean;
}

// null is intentional: it means "remove this override on save". An absent
// key means no staged change for that property.
export type Edits = Record<string, unknown | null>;

export function sameValue(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function hasEdit(edits: Edits, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(edits, key);
}

export function chapterDefault(item: PresetItem, chapter: number): PresetValue {
  return item.chValues?.[String(chapter)] ?? item.current;
}

function savedOverride(item: PresetItem): unknown | undefined {
  return item.isOverride ? item.current.value : undefined;
}

/** The explicit override after staged changes, if any. */
export function overrideValue(item: PresetItem, edits: Edits): unknown | null | undefined {
  return hasEdit(edits, item.key) ? edits[item.key] : savedOverride(item);
}

export function isCustom(item: PresetItem, edits: Edits): boolean {
  const value = overrideValue(item, edits);
  return value !== undefined && value !== null;
}

export function effectiveValue(item: PresetItem, chapter: number, edits: Edits): unknown {
  const override = overrideValue(item, edits);
  return override === undefined || override === null
    ? chapterDefault(item, chapter).value
    : override;
}

/**
 * Compute the edit required to show `value` against `chapter`'s baseline.
 * Returning null removes an override; returning undefined cancels a staged
 * edit because the saved state already has the requested result.
 */
export function editForValue(item: PresetItem, chapter: number, value: unknown): unknown | null | undefined {
  const next = sameValue(value, chapterDefault(item, chapter).value) ? null : value;
  const saved = savedOverride(item);

  if (next === null && saved === undefined) return undefined;
  if (next !== null && saved !== undefined && sameValue(next, saved)) return undefined;
  return next;
}

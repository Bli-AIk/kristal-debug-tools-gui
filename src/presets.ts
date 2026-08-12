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
 *
 * Once an item has a staged user override, clicking any value only changes
 * that override — even when the value equals the current chapter default.
 * A `null` edit is an explicit deletion produced by the reset action, not
 * by clicking a value that happens to match the chapter default.
 */
export function editForValue(
  item: PresetItem,
  chapter: number,
  value: unknown,
  edits: Edits = {},
): unknown | null | undefined {
  const staged = hasEdit(edits, item.key) ? edits[item.key] : undefined;
  const saved = savedOverride(item);

  if (staged !== undefined) {
    return sameValue(value, staged) ? staged : value;
  }
  if (saved !== undefined && sameValue(value, saved)) return undefined;
  if (saved === undefined && sameValue(value, chapterDefault(item, chapter).value)) return undefined;
  return value;
}

/**
 * Explicitly cancel a pending edit or delete a saved override. Returns
 * undefined when the pending state should be cleared, or null when a saved
 * override should be removed on save.
 */
export function resetEdit(item: PresetItem, edits: Edits = {}): unknown | null | undefined {
  if (hasEdit(edits, item.key)) return undefined;
  return savedOverride(item) !== undefined ? null : undefined;
}

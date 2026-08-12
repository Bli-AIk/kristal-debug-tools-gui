import { describe, expect, it } from "vitest";
import { matchingChapter, applyPresetValues } from "./presets";

// A minimal item: chValues holds per-chapter preset values.
function item(key: string, chValues: Record<string, unknown> | undefined) {
  return {
    key,
    current: { label: String(chValues?.["1"] ?? ""), value: chValues?.["1"] ?? null },
    chValues: chValues
      ? Object.fromEntries(Object.entries(chValues).map(([ch, v]) => [ch, { label: String(v), value: v }]))
      : undefined,
  };
}

const items = [
  // full 4-chapter boolean
  item("growStronger", { "1": false, "2": true, "3": true, "4": false }),
  // only ch1/ch2 defined (like enableStorage)
  item("enableStorage", { "1": false, "2": true }),
  // free string outside the presets (like default_encounter)
  item("default_encounter", undefined),
];

describe("matchingChapter", () => {
  it("returns the chapter whose preset the content equals", () => {
    // content = ch.2's values (edits staged from the ch.2 preset)
    const edits = applyPresetValues(items, 2);
    expect(matchingChapter(items, edits)).toBe(2);
  });

  it("returns ch.1 when content equals ch.1 (no edits)", () => {
    // defaults: current values come from ch.1 — matchingChapter must see them
    const defaults = items.map((it) => ({ ...it, current: { label: String(it.chValues?.["1"]?.value ?? ""), value: it.chValues?.["1"]?.value ?? null } }));
    expect(matchingChapter(defaults, {})).toBe(1);
  });

  it("returns 0 (custom) when one item differs from every preset", () => {
    const edits = { growStronger: 999 }; // not any chapter's value
    expect(matchingChapter(items, edits)).toBe(0);
  });

  it("returns the matching chapter when the edit equals an existing preset", () => {
    // true: ch.3 matches (enableStorage has no ch.3 value, ch.2 needs it true)
    expect(matchingChapter(items, { growStronger: true })).toBe(3);
  });

  it("treats ch.3 as matching even though enableStorage has no ch.3 value", () => {
    const edits = applyPresetValues(items, 3);
    expect(matchingChapter(items, edits)).toBe(3);
  });

  it("returns 0 when nothing but preset-less items exist (encounter only)", () => {
    expect(matchingChapter([item("default_encounter", undefined)], { default_encounter: "dummy" })).toBe(0);
  });
});

describe("applyPresetValues", () => {
  it("loads every chapter value into the edits, skipping missing ones", () => {
    const edits = applyPresetValues(items, 3);
    expect(edits.growStronger).toBe(true);
    expect(edits.enableStorage).toBeUndefined(); // no ch.3 preset
    expect(edits.default_encounter).toBeUndefined(); // no presets at all
  });
});

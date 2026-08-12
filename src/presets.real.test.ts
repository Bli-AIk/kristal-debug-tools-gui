import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { applyPresetValues, matchesChapter, matchingChapter } from "./presets";

// Rebuild items the way commands.rs does, from the real config-features
// rows (ch1..ch4 semantic labels) with raw values inferred.
const features = JSON.parse(
  readFileSync(new URL("../src-tauri/resources/config-features.json", import.meta.url), "utf8"),
) as { key: string; ch1?: string; ch2?: string; ch3?: string; ch4?: string }[];

function inferRaw(label: string | undefined): unknown {
  switch (label) {
    case "是": return true;
    case "否": return false;
    case "未设置": return null;
    default: return label;
  }
}

const items = features.map((f) => {
  // The real backend fills chapters 3/4 from the chapter json files
  // (raw values with string labels) when the feature table only has
  // ch1/ch2 — mimic that by carrying the last known value forward.
  const chValues: Record<string, { label: string; value: unknown }> = {};
  let last: { label: string; value: unknown } | undefined;
  for (const ch of ["1", "2", "3", "4"]) {
    const label = (f as unknown as Record<string, string | undefined>)[`ch${ch}`];
    if (label !== undefined) {
      last = { label, value: inferRaw(label) };
      chValues[ch] = last;
    } else if (last) {
      chValues[ch] = { label: String(last.value), value: last.value };
    }
  }
  return {
    key: f.key,
    current: { label: Object.values(chValues)[0]?.label ?? "", value: Object.values(chValues)[0]?.value ?? null },
    chValues: Object.keys(chValues).length ? chValues : undefined,
  };
});

describe("real data", () => {
  it("loading ch.3 makes the content match ch.3 (several chapters may also match, since most items are identical across chapters)", () => {
    const edits = applyPresetValues(items, 3);
    expect(matchesChapter(items, edits, 3)).toBe(true);
  });

  // Note: chapters mostly share values, so loading ch.3 may also match
  // ch.4 — that ambiguity is why the UI keeps an explicit chapter pick.

  it("no edits: defaults match ch.1", () => {
    expect(matchingChapter(items, {})).toBe(1);
  });

  it("defaults do NOT match ch.4 (that's the reported bug)", () => {
    // defaults come from ch.1; ch.4 must not match
    expect(matchingChapter(items, {})).not.toBe(4);
  });
});

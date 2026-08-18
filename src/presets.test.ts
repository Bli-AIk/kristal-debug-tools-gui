import { describe, expect, it } from "vitest";
import {
  chapterDefault,
  editForValue,
  effectiveValue,
  hasEdit,
  isCustom,
  overrideValue,
  previewDiff,
  resetEdit,
} from "./presets";

const item = {
  key: "growStronger",
  current: { label: "否", value: false },
  chValues: {
    "1": { label: "否", value: false },
    "2": { label: "是", value: true },
    "3": { label: "是", value: true },
    "4": { label: "否", value: false },
    "5": { label: "是", value: true },
  },
};

// The exact scenario from the bug report: ch4 default true, ch3 default false.
const awake = {
  key: "awakeMessages",
  current: { label: "是", value: true },
  chValues: {
    "1": { label: "是", value: true },
    "2": { label: "是", value: true },
    "3": { label: "否", value: false },
    "4": { label: "是", value: true },
    "5": { label: "是", value: true },
  },
};

describe("chapter baseline and overrides", () => {
  it("uses the selected chapter as the baseline without generating edits", () => {
    expect(chapterDefault(item, 4).value).toBe(false);
    expect(chapterDefault(item, 5).value).toBe(true);
    expect(effectiveValue(item, 4, {})).toBe(false);
    expect(effectiveValue(item, 2, {})).toBe(true);
  });

  it("keeps an explicit override when the chapter changes", () => {
    const overridden = { ...item, current: { label: "否", value: false }, isOverride: true };
    expect(isCustom(overridden, {})).toBe(true);
    expect(effectiveValue(overridden, 2, {})).toBe(false);
    expect(effectiveValue(overridden, 4, {})).toBe(false);
  });

  it("keeps a staged override when clicking a value equal to the current chapter default", () => {
    const staged = editForValue(awake, 4, false);
    expect(staged).toBe(false);
    const edits = { awakeMessages: staged! };

    // Switch to ch3 (default false) and click false again: the user override
    // must survive, not be silently cancelled back to a plain default.
    expect(editForValue(awake, 3, false, edits)).toBe(false);
    expect(overrideValue(awake, edits)).toBe(false);
    expect(isCustom(awake, edits)).toBe(true);
  });

  it("clicking another value while staged only changes that override", () => {
    const edits = { awakeMessages: true };
    expect(editForValue(awake, 3, false, edits)).toBe(false);
    expect(editForValue(awake, 3, true, edits)).toBe(true);
  });

  it("does not stage a no-op when a default value is chosen without an override", () => {
    expect(editForValue(item, 4, false)).toBeUndefined();
    expect(hasEdit({}, item.key)).toBe(false);
  });

  it("does not stage an edit just because another chapter's default is previewed", () => {
    expect(editForValue(item, 2, true)).toBeUndefined();
  });

  it("does not delete a saved override by clicking the current chapter default", () => {
    const overridden = { ...item, current: { label: "是", value: true }, isOverride: true };
    expect(editForValue(overridden, 4, false)).toBe(false);
  });

  it("does not turn a null baseline into a literal null override", () => {
    const nullBaseline = {
      ...item,
      chValues: { ...item.chValues, "1": { label: "未设置", value: null } },
    };
    expect(editForValue(nullBaseline, 1, null)).toBeUndefined();

    const overridden = {
      ...nullBaseline,
      current: { label: "未设置", value: null },
      isOverride: true,
    };
    expect(editForValue(overridden, 1, null)).toBeUndefined();
  });

  it("explicit reset cancels a staged edit or deletes a saved override", () => {
    expect(resetEdit(awake, { awakeMessages: false })).toBeUndefined();

    const overridden = { ...item, current: { label: "是", value: true }, isOverride: true };
    expect(resetEdit(overridden, {})).toBeNull();
    expect(overrideValue(overridden, { growStronger: null })).toBeNull();
    expect(isCustom(overridden, { growStronger: null })).toBe(false);
  });

  it("stages only the changed property, not an entire chapter preset", () => {
    const edit = editForValue(item, 4, true);
    expect(edit).toBe(true);
    expect(effectiveValue(item, 4, { growStronger: edit! })).toBe(true);
  });

  it("returns a target chapter value when it differs from the saved baseline", () => {
    expect(previewDiff(item, 5)).toEqual({ label: "是", value: true });
    expect(previewDiff(item, 1)).toBeUndefined();
  });

  it("supports numeric chapter diffs such as the Ch.5 storage preset", () => {
    const storage = {
      key: "storageSlots",
      current: { label: "36", value: 36 },
      chValues: {
        "4": { label: "36", value: 36 },
        "5": { label: "48", value: 48 },
      },
    };
    expect(previewDiff(storage, 5)).toEqual({ label: "48", value: 48 });
  });

  it("suppresses chapter diffs for saved overrides and staged edits", () => {
    const overridden = { ...item, isOverride: true };
    expect(previewDiff(overridden, 5)).toBeUndefined();
    expect(previewDiff(item, 5, { growStronger: true })).toBeUndefined();
    expect(previewDiff(item, 5, { growStronger: null })).toBeUndefined();
  });
});

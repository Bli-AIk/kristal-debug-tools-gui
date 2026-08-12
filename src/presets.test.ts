import { describe, expect, it } from "vitest";
import {
  chapterDefault,
  editForValue,
  effectiveValue,
  hasEdit,
  isCustom,
  overrideValue,
} from "./presets";

const item = {
  key: "growStronger",
  current: { label: "否", value: false },
  chValues: {
    "1": { label: "否", value: false },
    "2": { label: "是", value: true },
    "3": { label: "是", value: true },
    "4": { label: "否", value: false },
  },
};

describe("chapter baseline and overrides", () => {
  it("uses the selected chapter as the baseline without generating edits", () => {
    expect(chapterDefault(item, 4).value).toBe(false);
    expect(effectiveValue(item, 4, {})).toBe(false);
    expect(effectiveValue(item, 2, {})).toBe(true);
  });

  it("keeps an explicit override when the chapter changes", () => {
    const overridden = { ...item, current: { label: "否", value: false }, isOverride: true };
    expect(isCustom(overridden, {})).toBe(true);
    expect(effectiveValue(overridden, 2, {})).toBe(false);
    expect(effectiveValue(overridden, 4, {})).toBe(false);
  });

  it("stages removal when a user selects the current chapter default", () => {
    const overridden = { ...item, current: { label: "否", value: false }, isOverride: true };
    const edit = editForValue(overridden, 4, false);
    expect(edit).toBeNull();
    expect(overrideValue(overridden, { growStronger: edit! })).toBeNull();
    expect(isCustom(overridden, { growStronger: edit! })).toBe(false);
  });

  it("does not stage a no-op when a default value is chosen again", () => {
    expect(editForValue(item, 4, false)).toBeUndefined();
    expect(hasEdit({}, item.key)).toBe(false);
  });

  it("does not stage an edit just because another chapter's default is previewed", () => {
    expect(editForValue(item, 2, true)).toBeUndefined();
  });

  it("cancels a staged edit by choosing the saved override again", () => {
    const overridden = {
      key: "darkCandyForm",
      current: { label: "dark", value: "dark" },
      chValues: {
        "1": { label: "dark", value: "dark" },
        "2": { label: "darker", value: "darker" },
        "3": { label: "dark", value: "dark" },
        "4": { label: "darker", value: "darker" },
      },
      isOverride: true,
    };
    const staged = editForValue(overridden, 2, "round");
    expect(staged).toBe("round");
    expect(editForValue(overridden, 2, "dark")).toBeUndefined();
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
    expect(editForValue(overridden, 1, null)).toBeNull();
  });

  it("stages only the changed property, not an entire chapter preset", () => {
    const edit = editForValue(item, 4, true);
    expect(edit).toBe(true);
    expect(effectiveValue(item, 4, { growStronger: edit! })).toBe(true);
  });
});

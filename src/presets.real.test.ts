import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { effectiveValue, isCustom } from "./presets";

// The GUI's feature catalog contains every engine option that can be shown.
// This guards the data path used by the UI without pretending it is a preset
// that should be written into mod.json.
const features = JSON.parse(
  readFileSync(new URL("../src-tauri/resources/config-features.json", import.meta.url), "utf8"),
) as { key: string }[];

describe("real chapter option catalog", () => {
  it("contains unique keys", () => {
    const keys = features.map((feature) => feature.key);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("treats a saved config value as an override over any chapter baseline", () => {
    const item = {
      key: "enemyAuras",
      current: { label: "否", value: false },
      isOverride: true,
      chValues: {
        "1": { label: "否", value: false },
        "2": { label: "是", value: true },
        "3": { label: "是", value: true },
        "4": { label: "是", value: true },
        "5": { label: "是", value: true },
      },
    };
    expect(isCustom(item, {})).toBe(true);
    expect(effectiveValue(item, 4, {})).toBe(false);
    expect(effectiveValue(item, 5, {})).toBe(false);
  });
});

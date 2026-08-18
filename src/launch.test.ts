import { describe, expect, it } from "vitest";
import { buildLaunchOptions, supportsLanguageSelection, type LaunchForm } from "./launch";

const form: LaunchForm = {
  lang: "zh-hans",
  encounter: "spamton",
  wave: "2",
  waveForce: "",
  tp: "40",
  mercy: "",
  passthrough: "--debug --trace",
};

describe("language launch capability", () => {
  it("only enables the language selector for a confirmed i18n capability", () => {
    expect(supportsLanguageSelection()).toBe(false);
    expect(supportsLanguageSelection({})).toBe(false);
    expect(supportsLanguageSelection({ i18n: false })).toBe(false);
    expect(supportsLanguageSelection({ i18n: true })).toBe(true);
  });

  it("does not send a language argument when i18n is unavailable", () => {
    const options = buildLaunchOptions(form, false);
    expect(options).not.toHaveProperty("lang");
    expect(options.passthrough).toEqual(["--debug", "--trace"]);
  });

  it("sends the selected language when i18n is available", () => {
    expect(buildLaunchOptions(form, true)).toMatchObject({ lang: "zh-hans" });
  });
});

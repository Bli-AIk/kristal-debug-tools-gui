export interface LaunchCapabilities {
  i18n?: boolean;
}

export interface LaunchForm {
  lang: string;
  encounter: string;
  wave: string;
  waveForce: string;
  tp: string;
  mercy: string;
  passthrough: string;
}

export function supportsLanguageSelection(capabilities?: LaunchCapabilities): boolean {
  return capabilities?.i18n === true;
}

export function buildLaunchOptions(
  form: LaunchForm,
  languageEnabled: boolean,
): Record<string, unknown> {
  const options: Record<string, unknown> = {
    encounter: form.encounter || undefined,
    wave: form.wave || undefined,
    waveForce: form.waveForce || undefined,
    tp: form.tp || undefined,
    mercy: form.mercy || undefined,
    passthrough: form.passthrough.split(/\s+/).filter(Boolean),
  };
  if (languageEnabled && form.lang) options.lang = form.lang;
  return options;
}

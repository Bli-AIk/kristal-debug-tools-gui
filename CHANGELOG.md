# Changelog

## [0.1.2](https://github.com/Bli-AIk/kristal-debug-tools-gui/compare/v0.1.1...v0.1.2) (2026-08-14)


### Bug Fixes

* Windows launch game exits immediately + sidecar never found ([#4](https://github.com/Bli-AIk/kristal-debug-tools-gui/issues/4)) ([45623f2](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/45623f2b7cd0c271c4eb017ee1490a3851dd5657))

## [0.1.1](https://github.com/Bli-AIk/kristal-debug-tools-gui/compare/v0.1.0...v0.1.1) (2026-08-13)


### chore

* force release 0.1.1 ([46af2dc](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/46af2dcc0e44566f9f5236bfe90e7a6698a36602))


### Features

* add icon configuration panel (Unity-style multi-resolution icons) ([98a737d](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/98a737da0c904da73709ce8cf9668ac8600aa653))
* remove compile-only mode setting ([28ee598](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/28ee59800234f742dc4cb89a63e18986ead2da14))


### Bug Fixes

* embed frontend into the raw release binary ([4e907eb](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/4e907eb89d719702a2b1f4ba41eba9fa84ec8494))
* keep the UI responsive while generating icons ([2ab0f49](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/2ab0f49910e5561217c8bd3578db800a302c0312))

## [0.1.0](https://github.com/Bli-AIk/kristal-debug-tools-gui/compare/v0.1.0...v0.1.0) (2026-08-12)


### chore

* release 0.1.0 ([36bcd8e](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/36bcd8e07f84c95acb980f9d3b8d3e04e985c18f))


### Features

* after confirming init, the button shows 星之 行者 / Star Walker for 3 seconds (per language) ([cf70f07](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/cf70f078de93af5aeba2eab90ad5761367bb1124))
* attach raw binaries + checksums to releases (no-toolchain launcher) ([0906bc5](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/0906bc594e3e9c10860b070b114a70da30e674f0))
* chapter buttons apply a preset to the staged content — the active indicator is derived (Ch.N when content equals that preset, ★ custom otherwise) ([3963933](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/39639334ad46632f7fd4721323e89c0ad3eb276a))
* chapter config rows show Kristal's option names — 'Name - description' with semantic value controls ([61eb89c](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/61eb89c0186af04849cd2d884e0cf7a2237a1988))
* chapter preview shows the selected chapter's values in yellow (green overrides win); add a ★ custom indicator ([ee8e77f](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/ee8e77f5960235a36bdbda106bd40235ff84148e))
* Deltarune-style frontend with DPR zoom and chapter config ([52e9996](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/52e99962ee9ace7d457882aa781dfbd8b76af002))
* free-form string configs get an editable field; use source registerOption candidates; mercy 条 naming ([58e92c8](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/58e92c815f6d4ba2dcff7e464523e54537d20246))
* language-prefixed doc comments from justfiles (# zh_hans: / # en:) ([fc4c4c1](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/fc4c4c16467e9992f15cb22ad6d1fb305f28ce35))
* non-menu configs (lightCurrency etc.) + an encounter field in an 'other config' group at the bottom ([0e40c8d](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/0e40c8d7ec89aee3d8d6e7a173b20529c7b62823))
* Rust backend with just crate compiled into kristal-run sidecar ([6c8b61a](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/6c8b61a6dcdac2d80e954273d72f16a0f19f1c48))
* settings — compile-only mode toggle (writes .tools/gui/.mode) ([e9dc13c](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/e9dc13cb6889a1085d18e7614b8ad671b96fc2b8))
* settings dropdown (language + keep-terminal-open), tasks close by default ([737e277](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/737e2779ec5429f6b5849593c1c835542c91e65e))
* staged chapter-config edits — click only stages (yellow), the top SAVE button writes them all with one 'saved' flash; ★ custom auto-updates after saving ([c95d6d3](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/c95d6d34326a4860a4810c0a5f96e947dff90cfc))
* symmetric layout with stacked chapter/project cards, task list full-width ([b4af38d](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/b4af38d027f3f79193922ac4b92e8e4d0d1c17c3))
* symmetric two-column layout, full-width chapter config page ([5bf602c](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/5bf602c01ca5610adaf79ce7bea834952f3b24f1))
* task rows are the run button — name+params left, doc right-aligned, hover yellow, click runs ([ea8b4f9](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/ea8b4f9af39e841dd3f898e5c0473897aa4fe4c6))
* unify settings into .tools/gui/settings.json (lang/scale/keepOpen/mode); launchers read it; expand boolean options from registerOption candidates ([cdae401](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/cdae401d6c5e3932e5b893b6d71f862eb54ca963))


### Bug Fixes

* ★ custom dims to dark green while previewing other chapters, like everything else ([1afd4eb](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/1afd4eb82d682ffd93bdf2446498ee028985a89e))
* ★ custom never dims while previewing other chapters — it is the current state ([77b0eff](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/77b0effc59f3dcb904a8434539d9d8e2929a2ddd))
* ★ custom now compares overrides against the current chapter's preset — changing ch.4's 否→是 and saving flips the custom indicator instead of staying on ch.4 ([ba25654](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/ba2565413eb53dd44383b7f7f10329b5c8050fef))
* boolean non-menu configs (enableStorage) get 是/否 buttons in the other-config group ([b7ed3bc](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/b7ed3bc1c4a00aa4d4c7636306b2776497556f54))
* compile-only checkbox kept a local state — status.guiMode never updated on click ([6569b4d](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/6569b4decac85a31dbf91cbe8ffbede7836efc1f))
* current/preview values fall back to any chapter's semantic label and infer the raw value — enableStorage gets its green highlight back ([5f75f3e](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/5f75f3e810eaef706ad7381996392f5f798dde6f))
* custom checkbox in the settings menu — native one was invisible on black ([5c413a1](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/5c413a1a1a2af0b589dfa4eb2c64353071c7f5dd))
* default-run for cargo run with the sidecar bin ([4f8f226](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/4f8f226e0725b8eb34ecaf628a9b3b161b7c5504))
* dim the green applied state to dark green while a yellow preview is active (chapter bar + option rows) ([44b9623](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/44b962318f82502c689ccce55c5f4f58b2df53e2))
* drop the per-row 已保存/重置 clutter — toggle, save, and a difference from the current preset is simply custom (★) ([290017e](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/290017e036e59cb6cfcc62daeb80135fed72b062))
* four-space gap in 星之    行者 / Star     Walker (deliberate pixel spacing) ([b8806c5](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/b8806c567de6110b882863051a75e9335a60a2f0))
* infer raw values from semantic labels (是/否/未设置) when a config key is missing from the chapter files — enableStorage gets its 是/否 buttons ([b49a818](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/b49a81846ae2ad39c88386566523e09ca993adce))
* keep the green current-value highlight while previewing — preview shows as a yellow Ch.N tag instead of replacing it ([699cfc4](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/699cfc41a5ec8b11b63ad952fd91c18dee2194f2))
* move the yellow preview-diff tag to the left of the control ([86b9db5](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/86b9db534c9b424585d8a017fc28586940df2eb5))
* navbar z-index above the status bar so the settings dropdown overlays it ([3cd4644](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/3cd464426fac7fdd6eea7b151ca1fe044c7ce407))
* no wrap in cc-control so the preview tag sits on the same line, left of the control ([236ed32](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/236ed32bd7dda0a99e5c0441d51c6c1d87922d40))
* preview diff — green border stays, yellow preview value is plain text (no tag box) ([f62145a](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/f62145a83591ee41fde9680b7df3b35d9b3e1ef9))
* preview highlights live on the option buttons — yellow on the previewed value, green on the current one ([3a7af97](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/3a7af971182f453f1ab7ca70356451d5dfb78c03))
* preview value reuses the control look (same box), just yellow ([6925dde](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/6925ddea1189f5085d43c15c935bf7a9863f6019))
* restore the Map import, drop dead desc_map/config_feature_descs — zero warnings ([8271333](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/8271333576690a3487765d26a1542d636ae9d3e5))
* right-align all task run buttons to one column ([364b3a0](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/364b3a031dd67b66b9de758e5312a246f533cc8a))
* select/text preview diff — control stays on the current value (dim green), a yellow tag shows the previewed chapter's value only when it differs ([ae5db4a](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/ae5db4a32320d096a34c781a0c80798cbe514c41))
* selects and text inputs show the previewed chapter's value (yellow) when switching presets ([56d8bd4](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/56d8bd43dae10386068f93aedad56fd9c9765766))
* single SAVE button — staged property edits and the chapter pick are written together (no separate apply) ([42d1ae9](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/42d1ae96b115b3102d86d58bbb97d4823543668a))
* strip JSON quotes from option/preview labels ("noelle" → noelle, "dark_old" → dark_old) ([c324f13](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/c324f1374aad6ccea9e653009186617e0c9d78f7))
* task doc pinned to the last grid column — right-aligned even without params; no more ellipsis truncation ([1ff2398](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/1ff23980779862ff7d2bd73e61238806eebfc734))
* task rows as one grid line — params and run button adjacent at the right ([bd365f1](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/bd365f127f94a021ba155118913a64c2f2c5d53e))
* when ★ custom is active the chapter button no longer stays highlighted — one state, one highlight ([77d2f09](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/77d2f0954e374cb03299d93be99b096557b444e8))
* wrap invoke payloads in req for launch_game/run_task/chapter_config_set/template_init ([df86518](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/df86518844eba7bb471c655cafdb802d917150c4))
* **章节:** 用 JSONC CST 保存章节预设与覆盖 ([e5b9ab8](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/e5b9ab83a21d07fc8da2a7be59e080d3bd758319))
* **章节:** 用户覆盖不再被当前章节默认值吞掉 ([8628c1a](https://github.com/Bli-AIk/kristal-debug-tools-gui/commit/8628c1aaec6b6de4266937bb5fb868315bea9038))

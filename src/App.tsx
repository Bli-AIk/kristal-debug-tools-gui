import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./assets/style.css";

/* ---------- i18n ---------- */

const I18N: Record<string, Record<string, string>> = {
  zh: {
    tasks: "运行项列表（高级）", launch: "启动游戏", runs: "运行记录",
    project: "项目信息", system: "系统", libraries: "依赖库", projectBuilds: "项目构建",
    initProject: "项目初始化", projectName: "项目名", initBtn: "初始化项目", initConfirm: "再次点击确认",
    initDone: "初始化已在终端窗口启动，完成后建议重启 GUI", initFail: "初始化失败",
    customConfig: "自定义配置", configure: "配置", chapterConfig: "章节预设",
    currentChapter: "当前章节预设", overridesActive: "项配置覆盖生效中", noOverrides: "无配置覆盖（用默认值）",
    fLang: "语言", fEncounter: "遭遇", fWave: "波次", fWaveForce: "强制波次",
    fTp: "初始 TP", fMercy: "初始 mercy", fExtra: "额外参数",
    love: "love", engine: "引擎", mod: "模组", just: "just", justEmbedded: "内置 (just crate)",
    loveMissing: "未找到 love（请安装 LÖVE 并加入 PATH）",
    noEngine: "未找到 Kristal 引擎", noMod: "未找到 mod", justNone: "不可用",
    empty: "（空）", noTasks: "没有可运行的任务",
    launchOk: "游戏已在新终端窗口中启动", taskStarted: "任务已在新终端窗口中启动",
    taskFail: "启动失败", apply: "应用", overrideDefault: "默认", overrideOn: "开", overrideOff: "关",
    overrideSaved: "已保存", overrideReset: "重置", back: "返回", chapterSaved: "默认章节已保存",
    save: "保存",
    run: "运行", refresh: "刷新",
    otherConfig: "其他配置", encounterEmpty: "留空为无",
    settings: "设置", language: "语言", keepOpen: "任务运行完保持窗口打开",
    compileOnly: "仅本地编译模式（下次启动生效）",
  },
  en: {
    tasks: "RUN LIST (ADVANCED)", launch: "LAUNCH GAME", runs: "RUNS",
    project: "PROJECT", system: "system", libraries: "libraries", projectBuilds: "PROJECT BUILDS",
    initProject: "INITIALIZE PROJECT", projectName: "PROJECT NAME", initBtn: "INITIALIZE", initConfirm: "CLICK AGAIN TO CONFIRM",
    initDone: "initialization started in a terminal window — restart the GUI when done", initFail: "init failed",
    customConfig: "CUSTOM CONFIG", configure: "CONFIGURE", chapterConfig: "CHAPTER PRESETS",
    currentChapter: "CURRENT PRESET", overridesActive: "overrides active", noOverrides: "no overrides (defaults)",
    fLang: "LANGUAGE", fEncounter: "ENCOUNTER", fWave: "WAVE", fWaveForce: "WAVE FORCE",
    fTp: "TP", fMercy: "MERCY", fExtra: "EXTRA ARGS",
    love: "love", engine: "engine", mod: "mod", just: "just", justEmbedded: "builtin (just crate)",
    loveMissing: "love not found (install LÖVE and add it to PATH)",
    noEngine: "Kristal engine not found", noMod: "mod not found", justNone: "unavailable",
    empty: "(empty)", noTasks: "no runnable tasks",
    launchOk: "game launched in a new terminal window", taskStarted: "task started in a new terminal window",
    taskFail: "start failed", apply: "APPLY", overrideDefault: "default", overrideOn: "on", overrideOff: "off",
    overrideSaved: "saved", overrideReset: "reset", back: "BACK", chapterSaved: "default chapter saved",
    save: "SAVE",
    run: "RUN", refresh: "REFRESH",
    otherConfig: "OTHER CONFIG", encounterEmpty: "empty = none",
    settings: "SETTINGS", language: "LANGUAGE", keepOpen: "keep the task window open after it finishes",
    compileOnly: "compile from source only (next launch)",
  },
};

// Settings live in .tools/gui/settings.json (loaded via status.settings).
let lang = (navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en") as string;
const t = (key: string) => I18N[lang]?.[key] ?? I18N.en[key] ?? key;

/* ---------- types ---------- */

interface Status {
  modRoot: string; modID: string; engineRoot: string;
  engine?: { version?: string; hash?: string };
  love: { found: boolean; path: string };
  just: { found: boolean; path: string; mode?: string };
  project?: { id: string; name?: string; subtitle?: string };
  libraries?: { id: string; version?: string }[];
  template?: { isTemplate: boolean; name?: string; chapter?: number } | null;
  os: string; arch: string;
  guiMode?: boolean;
  settings?: Record<string, unknown>;
}
interface TaskItem {
  name: string; doc?: string; private?: boolean;
  params?: { name: string; kind: string }[];
  aliases?: string[];
}
interface TasksResult { source: string; tasks: TaskItem[]; mod?: { tasks: TaskItem[] } | null }
interface ChapterItem {
  key: string; name?: string; desc?: string; descEn?: string;
  options: { label: string; value: unknown }[];
  current: { label: string; value: unknown };
  chValues?: Record<string, { label: string; value: unknown }>;
  isOverride?: boolean;
  standard?: boolean;
}
interface ChapterConfig { chapter: number; items: ChapterItem[] }
interface RunEntry { id: number; label: string; command: string }

/* ---------- App ---------- */

export default function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [tasks, setTasks] = useState<TasksResult | null>(null);
  const [chapterConfig, setChapterConfig] = useState<ChapterConfig | null>(null);
  const [view, setView] = useState<"main" | "chapter">("main");
  const [flash, setFlash] = useState<{ msg: string; err?: boolean } | null>(null);
  const [runs, setRuns] = useState<RunEntry[]>([]);
  const [, force] = useState(0);
  const flashTimer = useRef<number>(0);

  const refresh = useCallback(() => force((n) => n + 1), []);

  const showFlash = useCallback((msg: string, err = false) => {
    setFlash({ msg, err });
    window.clearTimeout(flashTimer.current);
    flashTimer.current = window.setTimeout(() => setFlash(null), 4000);
  }, []);

  const loadAll = useCallback(() => {
    invoke<Status>("status").then(setStatus).catch((e) => showFlash(String(e), true));
    invoke<TasksResult>("tasks", { lang: lang === "zh" ? "zh_hans" : "en" }).then(setTasks).catch(() => setTasks(null));
    invoke<ChapterConfig>("chapter_config").then(setChapterConfig).catch(() => setChapterConfig(null));
  }, [showFlash]);

  useEffect(() => { loadAll(); }, [loadAll]);

  // High-DPI aware zoom: default scales with devicePixelRatio (>= 1.25,
  // capped at 1.6), user-adjustable via A−/A+ and persisted in settings.
  const [scaleLabel, setScaleLabel] = useState("");
  const dprScale = () => {
    const dpr = window.devicePixelRatio || 1;
    return Math.min(1.6, Math.max(1.25, 0.6 + dpr * 0.5));
  };
  useEffect(() => {
    const s = parseFloat(String(status?.settings?.scale ?? ""));
    const v = s >= 0.75 && s <= 3 ? s : dprScale();
    document.documentElement.style.zoom = v.toFixed(3);
    setScaleLabel(Math.round(v * 100) + "%");
  }, [status]);
  const setScale = (delta: number) => {
    const cur = parseFloat(document.documentElement.style.zoom) || dprScale();
    const s = Math.min(3, Math.max(0.75, cur + delta));
    document.documentElement.style.zoom = s.toFixed(3);
    setScaleLabel(Math.round(s * 100) + "%");
    invoke("set_settings", { patch: { scale: s } }).catch(() => {});
  };

  const addRun = (label: string, command: string) =>
    setRuns((rs) => [{ id: rs.length + 1, label, command }, ...rs].slice(0, 50));

  // Settings: language + "keep the task terminal open after it finishes"
  // + compile-only launch mode — all in .tools/gui/settings.json.
  const [menuOpen, setMenuOpen] = useState(false);
  const [keepOpen, setKeepOpen] = useState(false);
  const [compileOnly, setCompileOnly] = useState(false);
  useEffect(() => {
    const s = status?.settings;
    if (!s) return;
    setKeepOpen(s.keepOpen === true);
    setCompileOnly(s.mode === "compile");
    if (typeof s.lang === "string" && s.lang !== lang) {
      lang = s.lang;
      refresh();
    }
  }, [status]);

  const overrideCount =
    chapterConfig?.items.filter((i) => i.isOverride).length ?? 0;

  return (
    <div className="app">
      <header className="navbar">
        <div className="nav-inner">
          <span className="title">KRISTAL DEBUG TOOLS</span>
          <span className="nav-right">
            <span className="scale-group">
              <button className="btn small" title="A−" onClick={() => setScale(-0.15)}>A−</button>
              <span className="scale-label">{scaleLabel}</span>
              <button className="btn small" title="A+" onClick={() => setScale(0.15)}>A+</button>
            </span>
            <div className="settings">
              <button className="btn" onClick={() => setMenuOpen(!menuOpen)}>⚙ {t("settings")}</button>
              {menuOpen && (
                <div className="settings-menu">
                  <label className="set-row">
                    <span>{t("language")}</span>
                    <select value={lang} onChange={(e) => {
                      lang = e.target.value;
                      invoke("set_settings", { patch: { lang } }).catch(() => {});
                      refresh();
                    }}>
                      <option value="zh">中文</option>
                      <option value="en">English</option>
                    </select>
                  </label>
                  <label className="set-row check">
                    <span>{t("keepOpen")}</span>
                    <input type="checkbox" checked={keepOpen}
                      onChange={(e) => {
                        setKeepOpen(e.target.checked);
                        invoke("set_settings", { patch: { keepOpen: e.target.checked } }).catch(() => {});
                      }} />
                  </label>
                  <label className="set-row check">
                    <span>{t("compileOnly")}</span>
                    <input type="checkbox" checked={compileOnly}
                      onChange={(e) => {
                        setCompileOnly(e.target.checked);
                        invoke("set_settings", { patch: { mode: e.target.checked ? "compile" : "bin" } })
                          .catch((err) => { setCompileOnly(!e.target.checked); showFlash(String(err), true); });
                      }} />
                  </label>
                </div>
              )}
            </div>
          </span>
        </div>
      </header>

      <div className="statusbar">
        <StatusBar status={status} />
        {flash && <span className={"flash" + (flash.err ? " err" : "")}>{flash.msg}</span>}
      </div>

      {view === "main" ? (
        <main className="layout">
          {/* grid rows align the two columns: launch ⇄ project info,
              init ⇄ runs log, task list ⇄ chapter config */}
          <div className="cell c1r1">
            <LaunchPanel status={status} onLaunch={async (opts) => {
              try {
                await invoke("launch_game", { req: opts });
                addRun("launch game", `love ${status?.engineRoot ?? ""} --mod ${status?.modID ?? ""}`);
                showFlash(t("launchOk"));
              } catch (e) { showFlash(t("taskFail") + ": " + e, true); }
            }} />
          </div>
          <div className="cell c2r1">
            <ChapterEntry chapter={chapterConfig?.chapter ?? 0} count={overrideCount}
              onOpen={() => { loadAll(); setView("chapter"); }} />
            <ProjectPanel status={status} />
          </div>

          {status?.template?.isTemplate && (
            <div className="cell c1r2">
              <InitPanel onInit={async (name) => {
                try {
                  await invoke("template_init", { req: { name } });
                  showFlash(t("initDone"));
                  addRun("initialize project", `bash start.sh --name ${name}`);
                } catch (e) { showFlash(t("initFail") + ": " + e, true); }
              }} />
            </div>
          )}
          <div className="cell c2r2"><RunsLog runs={runs} /></div>

          <div className="cell c3">
            <TaskList tasks={tasks} onRun={async (task, args, justfile) => {
              try {
                await invoke("run_task", { req: { task, args, justfile, pause: keepOpen } });
                showFlash(t("taskStarted"));
                addRun(task, `just ${task} ${args.join(" ")}`);
              } catch (e) { showFlash(t("taskFail") + ": " + e, true); }
            }} onRefresh={loadAll} />
          </div>
        </main>
      ) : (
        <ChapterConfigPage
          config={chapterConfig}
          onBack={() => setView("main")}
          onPickChapter={async (n) => {
            try {
              await invoke("template_chapter", { chapter: n });
              showFlash(t("chapterSaved") + " — Chapter " + n);
              loadAll();
            } catch (e) { showFlash(String(e), true); }
          }}
          onSaveAll={async (edits) => {
            try {
              for (const [key, value] of Object.entries(edits)) {
                // an emptied field removes the override (encounter: 空为无)
                await invoke("chapter_config_set", { req: { key, value: value === "" ? null : value } });
              }
              showFlash(t("overrideSaved"));
              loadAll();
            } catch (e) { showFlash(String(e), true); }
          }}
        />
      )}
    </div>
  );
}

/* ---------- status bar ---------- */

function StatusBar({ status }: { status: Status | null }) {
  if (!status) return <span className="bad">status…</span>;
  const engineTag = status.engineRoot
    ? `${status.engineRoot}${status.engine?.version ? ` (${status.engine.version}${status.engine.hash ? " @ " + status.engine.hash : ""})` : ""}`
    : t("noEngine");
  const justTag = status.just.mode === "embedded"
    ? t("justEmbedded")
    : status.just.found ? status.just.path : t("justNone");
  return (
    <>
      <span className={status.love.found ? "" : "bad"}>
        {t("love")}: {status.love.found ? status.love.path : t("loveMissing")}
      </span>
      <span className={status.engineRoot ? "" : "bad"}>{t("engine")}: {engineTag}</span>
      <span className={status.modRoot ? "" : "bad"}>{t("mod")}: {status.modRoot || t("noMod")}</span>
      <span className={status.just.found ? "" : "bad"}>{t("just")}: {justTag}</span>
      <span>{t("system")}: {status.os} {status.arch}</span>
    </>
  );
}

/* ---------- launch panel ---------- */

interface LaunchForm {
  lang: string; encounter: string; wave: string; waveForce: string; tp: string; mercy: string; passthrough: string;
}

function LaunchPanel({ status, onLaunch }: { status: Status | null; onLaunch: (o: Record<string, unknown>) => void }) {
  const [form, setForm] = useState<LaunchForm>({
    lang: "", encounter: "", wave: "", waveForce: "", tp: "", mercy: "", passthrough: "",
  });
  const set = (k: keyof LaunchForm) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));

  const launch = () => {
    const passthrough = form.passthrough.split(/\s+/).filter(Boolean);
    onLaunch({
      lang: form.lang || undefined,
      encounter: form.encounter || undefined,
      wave: form.wave || undefined,
      waveForce: form.waveForce || undefined,
      tp: form.tp || undefined,
      mercy: form.mercy || undefined,
      passthrough,
    });
  };

  const field = (label: string, key: keyof LaunchForm, placeholder = "") => (
    <label>
      <span>{label}</span>
      <input value={form[key]} onChange={set(key)} placeholder={placeholder} autoComplete="off" />
    </label>
  );

  return (
    <div className="broken-box panel">
      <h2>{t("launch")}</h2>
      <div className="form-grid">
        <label>
          <span>{t("fLang")}</span>
          <select value={form.lang} onChange={set("lang")}>
            <option value="">auto / 自动</option>
            <option value="en">English (en)</option>
            <option value="zh-hans">简体中文 (zh-hans)</option>
          </select>
        </label>
        {field(t("fEncounter"), "encounter", "encounter id")}
        {field(t("fWave"), "wave", "2 / wave id")}
        {field(t("fWaveForce"), "waveForce")}
        {field(t("fTp"), "tp", "0-100")}
        {field(t("fMercy"), "mercy", "0-100")}
        <label className="wide">
          <span>{t("fExtra")}</span>
          <input value={form.passthrough} onChange={set("passthrough")} placeholder="--any --game flags" autoComplete="off" />
        </label>
      </div>
      <div className="launch-row">
        <button className="btn big" onClick={launch} disabled={!status?.love.found}>▶ {t("launch")}</button>
      </div>
    </div>
  );
}

/* ---------- init panel ---------- */

function InitPanel({ onInit }: { onInit: (name: string) => void }) {
  const [name, setName] = useState("");
  const [armed, setArmed] = useState(false);
  const [done, setDone] = useState(false);
  const timer = useRef(0);

  const click = () => {
    if (!name.trim()) return;
    if (!armed) {
      setArmed(true);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setArmed(false), 5000);
      return;
    }
    setArmed(false);
    setDone(true);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setDone(false), 3000);
    onInit(name.trim());
  };

  return (
    <div className="broken-box panel">
      <h2>{t("initProject")}</h2>
      <div className="form-grid">
        <label className="wide">
          <span>{t("projectName")}</span>
          <input value={name} onChange={(e) => setName(e.target.value)} autoComplete="off" />
        </label>
      </div>
      <div className="launch-row">
        <button className={"btn big" + (armed ? " armed" : "")} onClick={click}>
          ★ {done ? (lang === "zh" ? "星之    行者" : "Star     Walker") : armed ? t("initConfirm") : t("initBtn")}
        </button>
      </div>
    </div>
  );
}

/* ---------- chapter entry ---------- */

function ChapterEntry({ chapter, count, onOpen }: {
  chapter: number; count: number; onOpen: () => void;
}) {
  return (
    <div className="broken-box panel">
      <div className="panel-head">
        <h2>{t("chapterConfig")}</h2>
        <button className="btn small" onClick={onOpen}>{t("configure")}{count > 0 ? ` ✎${count}` : ""}</button>
      </div>
      <div className="chapter-info">
        <span className="ci-row">
          {t("currentChapter")}: {chapter > 0 ? `Ch.${chapter}` : "—"}
        </span>
        <span className="ci-row ci-sub">
          {count > 0 ? `${count} ${t("overridesActive")}` : t("noOverrides")}
        </span>
      </div>
    </div>
  );
}

/* ---------- task list ---------- */

function TaskList({ tasks, onRun, onRefresh }: {
  tasks: TasksResult | null;
  onRun: (task: string, args: string[], justfile: string) => void;
  onRefresh: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [args, setArgs] = useState<Record<string, string>>({});

  const row = (task: TaskItem, group: string, justfile: string) => {
    const id = `${group}-${task.name}`;
    const val = args[id] ?? "";
    const run = () => {
      const parts = val.split(/\s+/).filter(Boolean);
      onRun(task.name, parts, justfile);
    };
    return (
      <div className="task-row" key={id} onClick={run} title={t("run")}>
        <span className="task-name">
          {task.name}
          {task.aliases?.length ? <span className="alias"> [{task.aliases.join(", ")}]</span> : null}
        </span>
        {task.params?.length ? (
          <span className="task-params" onClick={(e) => e.stopPropagation()}>
            {task.params.map((p) => (
              <input key={p.name} type="text"
                placeholder={p.kind === "many" ? "a, b, c" : p.kind === "star" ? "arg1 arg2" : p.name}
                value={val} onChange={(e) => setArgs((a) => ({ ...a, [id]: e.target.value }))} />
            ))}
          </span>
        ) : null}
        <span className="task-doc">{task.doc ?? ""}</span>
      </div>
    );
  };

  return (
    <details className="broken-box panel" open={open} onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}>
      <summary>
        <h2>{t("tasks")}</h2>
        <span className="task-summary-right">
          <button className="btn small" onClick={onRefresh}>{t("refresh")}</button>
        </span>
      </summary>
      <div className="task-list">
        {tasks?.source === "builtin" ? <p className="hint err">{t("justNone")}</p>
          : tasks?.tasks.map((tk) => row(tk, "lib", ""))}
        {tasks?.mod?.tasks.length ? (
          <>
            <div className="task-group">{t("projectBuilds")}</div>
            {tasks.mod.tasks.map((tk) => row(tk, "proj", "project"))}
          </>
        ) : null}
      </div>
    </details>
  );
}

/* ---------- project panel ---------- */

function ProjectPanel({ status }: { status: Status | null }) {
  if (!status?.project?.id) return null;
  return (
    <div className="broken-box panel">
      <h2>{t("project")}</h2>
      <div className="project-info">
        <div className="proj-name">{status.project.name || status.project.id}</div>
        {status.project.subtitle ? <div className="proj-sub">{status.project.subtitle}</div> : null}
        {status.libraries?.length ? (
          <>
            <div className="proj-libs">{t("libraries")}:</div>
            <ul className="proj-list">
              {status.libraries.map((l) => (
                <li key={l.id}><span className="lib-id">{l.id}</span> <span className="lib-ver">{l.version ?? ""}</span></li>
              ))}
            </ul>
          </>
        ) : null}
      </div>
    </div>
  );
}

/* ---------- runs log ---------- */

function RunsLog({ runs }: { runs: RunEntry[] }) {
  return (
    <div className="broken-box panel">
      <div className="panel-head"><h2>{t("runs")}</h2></div>
      <div className="runs-log">
        {runs.length === 0 ? <span className="hint">{t("empty")}</span>
          : runs.map((r) => (
            <div className="run-entry" key={r.id}>
              <span className="r-label">{r.label}</span>
              <span className="r-cmd" title={r.command}>{r.command}</span>
              <span className="r-meta">{t("runs")}</span>
            </div>
          ))}
      </div>
    </div>
  );
}

/* ---------- chapter config page ---------- */

function ChapterConfigPage({ config, onBack, onPickChapter, onSaveAll }: {
  config: ChapterConfig | null;
  onBack: () => void;
  onPickChapter: (n: number) => void;
  onSaveAll: (edits: Record<string, unknown>) => void;
}) {
  const [selected, setSelected] = useState(0);
  // unsaved per-property edits: clicking an option only stages them; the
  // top SAVE button writes them all at once (one "saved" flash)
  const [edits, setEdits] = useState<Record<string, unknown>>({});
  useEffect(() => { if (config) setSelected(config.chapter); }, [config]);
  useEffect(() => { setEdits({}); }, [config]);

  if (!config) {
    return <main className="layout"><div className="cell c1r1"><div className="broken-box panel"><p className="hint">…</p></div></div></main>;
  }

  const pending = selected !== config.chapter && selected !== 0;
  const hasEdits = Object.keys(edits).length > 0;
  const edit = (key: string, value: unknown) =>
    setEdits((e) => ({ ...e, [key]: value }));

  // A custom config: an override differs from the CURRENT chapter's
  // preset (that's what the user is editing against). Saving such a
  // change flips the ★ custom indicator automatically.
  const hasCustom = config.items.some((it) =>
    it.isOverride &&
    String(it.current.value) !== String(it.chValues?.[String(config.chapter)]?.value));

  return (
    <main className="layout">
        <div className="broken-box panel chapter-page">
          <div className="panel-head">
            <h2>{t("chapterConfig")}</h2>
            <span className="chapter-buttons">
              {[1, 2, 3, 4].map((n) => (
                <button key={n}
                  className={"btn small" + (n === config.chapter && !hasCustom ? " applied" : "") + (n === selected && pending ? " pending" : "")}
                  onClick={() => setSelected(n)}>Ch.{n}</button>
              ))}
              <button className={"btn small" + (hasCustom ? " applied" : "")}
                disabled={!hasCustom} title={t("customConfig")}>★ {t("customConfig")}</button>
              {/* One save: staged property edits + the chapter pick, together. */}
              <button className="btn small danger" disabled={!hasEdits && !pending}
                onClick={() => {
                  if (hasEdits) onSaveAll(edits);
                  if (pending) onPickChapter(selected);
                  setEdits({});
                  setSelected(config.chapter);
                }}>
                {t("save")}
              </button>
            </span>
            <button className="btn small" onClick={onBack}>← {t("back")}</button>
          </div>
          <div className="chapter-config-list wide">
            {config.items.filter((i) => i.standard !== false).map((item) => {
              // Staged edits (yellow) preview before saving; the green
              // current value stays highlighted. Chapter preview also
              // shows in yellow until the property is edited.
              const editVal = edits[item.key];
              const hasEdit = editVal !== undefined;
              const prev = (!hasEdit && pending) ? item.chValues?.[String(selected)] : null;
              // for selects / text inputs: the control keeps the current
              // value (dim green), and a yellow tag next to it shows the
              // previewed chapter's value — only when it differs
              const diff = !!prev && String(prev.value) !== String(item.current.value);
              const shownVal = hasEdit ? editVal : item.current.value;
              const shownLabel = hasEdit
                ? (item.options.find((o) => String(o.value) === String(editVal))?.label ?? String(editVal))
                : item.current.label;
              const isPrev = (o: { value: unknown }) =>
                !!prev && String(o.value) === String(prev.value) &&
                String(o.value) !== String(item.current.value);
              const isEdit = (o: { value: unknown }) =>
                hasEdit && String(o.value) === String(editVal) &&
                String(o.value) !== String(item.current.value);
              return (
                <div className="cc-row" key={item.key}>
                  <span className="cc-name" title={item.key}>
                    {item.name ?? item.key}
                    {(item.desc || item.descEn) && (
                      <span className="cc-desc"> — {lang === "zh" ? (item.desc ?? item.descEn) : (item.descEn ?? item.desc)}</span>
                    )}
                  </span>
                  <span className="cc-control">
                    {item.options.length <= 1 && typeof item.current.value === "string" ? (
                      // free-form string config (lightCurrency etc.)
                      <>
                        {diff && !hasEdit && <span className="cc-preview" title={t("chapterConfig") + ` Ch.${selected}`}>{prev!.label}</span>}
                        <input className={"cc-edit" + (hasEdit ? " pending" : "")} type="text"
                          value={String(shownVal)}
                          onChange={(e) => edit(item.key, e.target.value)}
                          onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
                      </>
                    ) : item.options.length <= 1 ? (
                      <span className="cc-value applied">{shownLabel || "—"}</span>
                    ) : item.options.length === 2 ? (
                      <span className="cc-pair">
                        {item.options.map((o) => (
                          <button key={o.label}
                            className={"btn small"
                              + (String(o.value) === String(item.current.value) ? " applied" : "")
                              + (isEdit(o) || isPrev(o) ? " pending" : "")}
                            onClick={() => { if (String(o.value) !== String(shownVal)) edit(item.key, o.value); }}>
                            {o.label}
                          </button>
                        ))}
                      </span>
                    ) : (
                      <>
                        {diff && !hasEdit && <span className="cc-preview" title={t("chapterConfig") + ` Ch.${selected}`}>{prev!.label}</span>}
                        <select className={"cc-select" + (hasEdit ? " pending" : "")} value={String(shownVal)}
                          onChange={(e) => {
                            const o = item.options.find((x) => String(x.value) === e.target.value);
                            if (o) edit(item.key, o.value);
                          }}>
                          {item.options.map((o) => (
                            <option key={String(o.value)} value={String(o.value)}>{o.label}</option>
                          ))}
                        </select>
                      </>
                    )}
                  </span>
                </div>
              );
            })}
            {config.items.some((i) => i.standard === false) && (
              <div className="task-group other-config-head">{t("otherConfig")}</div>
            )}
            {config.items.filter((i) => i.standard === false).map((item) => {
              const editVal = edits[item.key];
              const hasEdit = editVal !== undefined;
              const shownVal = hasEdit ? editVal : item.current.value;
              return (
                <div className="cc-row" key={item.key}>
                  <span className="cc-name" title={item.key}>
                    {item.name ?? item.key}
                    {(item.desc || item.descEn) && (
                      <span className="cc-desc"> — {lang === "zh" ? (item.desc ?? item.descEn) : (item.descEn ?? item.desc)}</span>
                    )}
                  </span>
                  <span className="cc-control">
                    {typeof item.current.value === "boolean" ? (
                      <span className="cc-pair">
                        {[true, false].map((bv) => (
                          <button key={String(bv)}
                            className={"btn small" + (String(shownVal) === String(bv) ? " applied" : "")}
                            onClick={() => { if (String(shownVal) !== String(bv)) edit(item.key, bv); }}>
                            {bv ? "是" : "否"}
                          </button>
                        ))}
                      </span>
                    ) : typeof item.current.value === "string" ? (
                      <>
                        <input className={"cc-edit" + (hasEdit ? " pending" : "")} type="text"
                          placeholder={item.key === "default_encounter" ? t("encounterEmpty") : ""}
                          value={String(shownVal)}
                          onChange={(e) => edit(item.key, e.target.value)}
                          onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
                      </>
                    ) : (
                      <span className="cc-value applied">{item.current.label || "—"}</span>
                    )}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
    </main>
  );
}

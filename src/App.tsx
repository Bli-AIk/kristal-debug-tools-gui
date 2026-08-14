import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { IconType } from "react-icons";
import { FaAndroid, FaBox, FaBroom, FaHammer, FaHeart, FaMobile, FaWindows, FaWrench } from "react-icons/fa6";
import {
  editForValue,
  effectiveValue,
  hasEdit as hasStagedEdit,
  isCustom,
  resetEdit,
  sameValue,
} from "./presets";
import "./assets/style.css";

/* ---------- i18n ---------- */

const I18N: Record<string, Record<string, string>> = {
  zh: {
    tasks: "运行项列表（高级）", launch: "启动游戏", runs: "运行记录",
    project: "项目信息", system: "系统", libraries: "依赖库", projectBuilds: "项目构建", build: "构建",
    initProject: "项目初始化", projectName: "项目名", initBtn: "初始化项目", initConfirm: "再次点击确认",
    initDone: "初始化已在终端窗口启动，完成后建议重启 GUI", initFail: "初始化失败",
    customConfig: "自定义配置", configure: "配置", chapterConfig: "章节预设",
    currentChapter: "当前章节预设", overridesActive: "项自定义覆盖生效中", noOverrides: "无配置覆盖（使用章节默认值）",
    basedOnChapter: "基于 Ch.{chapter} 的自定义", chapterSaved: "章节预设已保存", overridesSaved: "章节预设与自定义覆盖已保存",
    fLang: "语言", fEncounter: "遭遇", fWave: "波次", fWaveForce: "强制波次",
    fTp: "初始 TP", fMercy: "初始 mercy", fExtra: "额外参数",
    love: "love", engine: "引擎", mod: "模组", just: "just",
    loveMissing: "未找到 love（请安装 LÖVE 并加入 PATH）",
    noEngine: "未找到 Kristal 引擎", noMod: "未找到 mod", justNone: "不可用",
    empty: "（空）", noTasks: "没有可运行的任务",
    launchOk: "游戏已在新终端窗口中启动", taskStarted: "任务已在新终端窗口中启动",
    taskFail: "启动失败", apply: "应用", overrideDefault: "默认", overrideOn: "开", overrideOff: "关",
    overrideSaved: "已保存", overrideReset: "重置", back: "返回",
    save: "保存",
    run: "运行", refresh: "刷新",
    otherConfig: "其他配置", encounterEmpty: "留空为无",
    settings: "设置", language: "语言", keepOpen: "任务运行完保持窗口打开",
    icons: "自定义图标", iconsGenerate: "从一张大图生成全部", iconsPick: "选择",
    iconsClear: "清除", iconsGroupWindow: "游戏窗口", iconsGroupWin: "Windows (.exe)",
    iconsGroupAndroid: "Android 启动图标", iconsRebuildHint: "保存后需在 thrash-machine 里重新构建（just build）才会生效",
    iconsScopeHint: "仅对 win 和 android 构建生效",
    iconsAndroidHint: "Android 按屏幕密度取图标：某档缺失时，构建会用最接近的一档自动补位。",
    iconsSaved: "图标已更新", iconsCleared: "已清除", iconsBadImage: "不是可用的图片",
    iconsEmpty: "未设置",
  },
  en: {
    tasks: "RUN LIST (ADVANCED)", launch: "LAUNCH GAME", runs: "RUNS",
    project: "PROJECT", system: "system", libraries: "libraries", projectBuilds: "PROJECT BUILDS", build: "BUILD",
    initProject: "INITIALIZE PROJECT", projectName: "PROJECT NAME", initBtn: "INITIALIZE", initConfirm: "CLICK AGAIN TO CONFIRM",
    initDone: "initialization started in a terminal window — restart the GUI when done", initFail: "init failed",
    customConfig: "CUSTOM CONFIG", configure: "CONFIGURE", chapterConfig: "CHAPTER PRESETS",
    currentChapter: "CURRENT PRESET", overridesActive: "custom overrides active", noOverrides: "no overrides (chapter defaults)",
    basedOnChapter: "customized from Ch.{chapter}", chapterSaved: "chapter preset saved", overridesSaved: "chapter preset and overrides saved",
    fLang: "LANGUAGE", fEncounter: "ENCOUNTER", fWave: "WAVE", fWaveForce: "WAVE FORCE",
    fTp: "TP", fMercy: "MERCY", fExtra: "EXTRA ARGS",
    love: "love", engine: "engine", mod: "mod", just: "just",
    loveMissing: "love not found (install LÖVE and add it to PATH)",
    noEngine: "Kristal engine not found", noMod: "mod not found", justNone: "unavailable",
    empty: "(empty)", noTasks: "no runnable tasks",
    launchOk: "game launched in a new terminal window", taskStarted: "task started in a new terminal window",
    taskFail: "start failed", apply: "APPLY", overrideDefault: "default", overrideOn: "on", overrideOff: "off",
    overrideSaved: "saved", overrideReset: "reset", back: "BACK",
    save: "SAVE",
    run: "RUN", refresh: "REFRESH",
    otherConfig: "OTHER CONFIG", encounterEmpty: "empty = none",
    settings: "SETTINGS", language: "LANGUAGE", keepOpen: "keep the task window open after it finishes",
    icons: "CUSTOM ICONS", iconsGenerate: "GENERATE ALL FROM ONE IMAGE", iconsPick: "CHOOSE",
    iconsClear: "CLEAR", iconsGroupWindow: "GAME WINDOW", iconsGroupWin: "WINDOWS (.EXE)",
    iconsGroupAndroid: "ANDROID LAUNCHER", iconsRebuildHint: "takes effect after rebuilding the mod (just build)",
    iconsScopeHint: "applies only to win and android builds",
    iconsAndroidHint: "Android picks icons by screen density; a missing slot falls back to the nearest one at build time.",
    iconsSaved: "icons updated", iconsCleared: "cleared", iconsBadImage: "not a usable image",
    iconsEmpty: "EMPTY",
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
interface ChapterSave {
  chapter: number;
  changes: Record<string, unknown | null>;
}
interface RunEntry { id: number; label: string; command: string }

/* ---------- App ---------- */

export default function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [tasks, setTasks] = useState<TasksResult | null>(null);
  const [chapterConfig, setChapterConfig] = useState<ChapterConfig | null>(null);
  const [view, setView] = useState<"main" | "chapter" | "icons">("main");
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

  // Runs any justfile task in a terminal window (the launch/init/build list
  // and the quick build targets all share this one path).
  const runTask = async (task: string, args: string[], justfile: string) => {
    try {
      await invoke("run_task", { req: { task, args, justfile, pause: keepOpen } });
      showFlash(t("taskStarted"));
      addRun(task, `just ${task} ${args.join(" ")}`);
    } catch (e) { showFlash(t("taskFail") + ": " + e, true); }
  };

  // Settings: language + "keep the task terminal open after it finishes",
  // stored in .tools/gui/settings.json.
  const [menuOpen, setMenuOpen] = useState(false);
  const [keepOpen, setKeepOpen] = useState(false);
  useEffect(() => {
    const s = status?.settings;
    if (!s) return;
    setKeepOpen(s.keepOpen === true);
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
            <ProjectPanel status={status} onIcons={() => setView("icons")} />
          </div>

          <div className="cell c1r2">
            {status?.template?.isTemplate && (
              <InitPanel onInit={async (name) => {
                try {
                  await invoke("template_init", { req: { name } });
                  showFlash(t("initDone"));
                  addRun("initialize project", `bash start.sh --name ${name}`);
                } catch (e) { showFlash(t("initFail") + ": " + e, true); }
              }} />
            )}
            <BuildPanel tasks={tasks} onRun={runTask} />
          </div>
          <div className="cell c2r2"><RunsLog runs={runs} /></div>

          <div className="cell c3">
            <TaskList tasks={tasks} onRun={runTask} onRefresh={loadAll} />
          </div>
        </main>
      ) : view === "chapter" ? (
        <ChapterConfigPage
          config={chapterConfig}
          onBack={() => setView("main")}
          onSaveAll={async (save) => {
            try {
              await invoke("chapter_config_save", { req: save });
              showFlash(Object.keys(save.changes).length ? t("overridesSaved") : t("chapterSaved"));
              loadAll();
            } catch (e) { showFlash(String(e), true); }
          }}
        />
      ) : (
        <IconConfigPage onBack={() => setView("main")} onFlash={showFlash} />
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
  return (
    <>
      <span className={status.love.found ? "" : "bad"}>
        {t("love")}: {status.love.found ? status.love.path : t("loveMissing")}
      </span>
      <span className={status.engineRoot ? "" : "bad"}>{t("engine")}: {engineTag}</span>
      <span className={status.modRoot ? "" : "bad"}>{t("mod")}: {status.modRoot || t("noMod")}</span>
      {status.just.mode !== "embedded" && (
        <span className={status.just.found ? "" : "bad"}>{t("just")}: {status.just.found ? status.just.path : t("justNone")}</span>
      )}
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
  const customLabel = t("basedOnChapter").replace("{chapter}", String(chapter));
  const custom = count > 0;
  return (
    <div className={"broken-box panel" + (custom ? " custom" : "")}>
      <div className="panel-head">
        <h2>{t("chapterConfig")}</h2>
        <button className="btn small" onClick={onOpen}>{t("configure")}{count > 0 ? ` ✎${count}` : ""}</button>
      </div>
      <div className={"chapter-info" + (custom ? " custom" : "")}>
        <span className="ci-row">
          {t("currentChapter")}: {chapter > 0 ? `Ch.${chapter}` : "—"}
        </span>
        <span className="ci-row ci-sub">
          {custom ? `${customLabel} (${count})` : t("noOverrides")}
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

function ProjectPanel({ status, onIcons }: { status: Status | null; onIcons: () => void }) {
  if (!status?.project?.id) return null;
  return (
    <div className="broken-box panel">
      <div className="panel-head">
        <h2>{t("project")}</h2>
        <button className="btn small" onClick={onIcons}>{t("icons")}</button>
      </div>
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

/* ---------- build panel (quick build targets) ---------- */

// The mod's build targets, shown as white icons + captions in a block to the
// left of the runs log. Keyed by justfile task name; any new build* task gets
// a wrench fallback so it keeps working without a code change.
const BUILD_ICONS: Record<string, IconType> = {
  "build": FaHammer,
  "build-love": FaHeart,
  "build-win": FaWindows,
  "build-mod": FaBox,
  "build-android": FaAndroid,
  "build-android-wrap": FaMobile,
  "clean-build": FaBroom,
};
const isBuildTarget = (tk: TaskItem) =>
  tk.name === "build" || tk.name.startsWith("build-") || tk.name === "clean-build";

function BuildPanel({ tasks, onRun }: {
  tasks: TasksResult | null;
  onRun: (task: string, args: string[], justfile: string) => void;
}) {
  const builds = tasks?.mod?.tasks.filter(isBuildTarget) ?? [];
  if (!builds.length) return null;
  return (
    <div className="broken-box panel">
      <div className="panel-head"><h2>{t("build")}</h2></div>
      <div className="build-grid">
        {builds.map((tk) => {
          const Icon = BUILD_ICONS[tk.name] ?? FaWrench;
          return (
            <button className="build-target" key={tk.name}
              title={tk.doc ?? tk.name}
              onClick={() => onRun(tk.name, [], "project")}>
              <span className="build-icon"><Icon /></span>
              <span className="build-label">{tk.name}</span>
            </button>
          );
        })}
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

function ChapterConfigPage({ config, onBack, onSaveAll }: {
  config: ChapterConfig | null;
  onBack: () => void;
  onSaveAll: (save: ChapterSave) => void;
}) {
  if (!config) {
    return <main className="layout"><div className="cell c1r1"><div className="broken-box panel"><p className="hint">…</p></div></div></main>;
  }

  return <ChapterConfigEditor config={config} onBack={onBack} onSaveAll={onSaveAll} />;
}

function ChapterConfigEditor({ config, onBack, onSaveAll }: {
  config: ChapterConfig;
  onBack: () => void;
  onSaveAll: (save: ChapterSave) => void;
}) {

  // A chapter is a baseline, not a bag of config values. Property edits are
  // only explicit Kristal overrides; null means restore the baseline.
  const [chapter, setChapter] = useState(config.chapter);
  const [edits, setEdits] = useState<Record<string, unknown | null>>({});
  useEffect(() => {
    setChapter(config.chapter);
    setEdits({});
  }, [config]);

  const chapterChanged = chapter !== config.chapter;
  const hasChanges = chapterChanged || Object.keys(edits).length > 0;
  const stageValue = (item: ChapterItem, value: unknown) => {
    setEdits((previous) => {
      const change = editForValue(item, chapter, value, previous);
      const next = { ...previous };
      if (change === undefined) delete next[item.key];
      else next[item.key] = change;
      return next;
    });
  };
  const resetValue = (item: ChapterItem) => {
    setEdits((previous) => {
      const change = resetEdit(item, previous);
      const next = { ...previous };
      if (change === undefined) delete next[item.key];
      else next[item.key] = change;
      return next;
    });
  };
  const pendingCount = Object.keys(edits).length;
  const pending = chapterChanged || pendingCount > 0;
  const userOverrideCount = config.items.filter((item) => isCustom(item, edits)).length;
  const hasSummary = userOverrideCount > 0;
  const basedOn = t("basedOnChapter").replace("{chapter}", String(chapter));

  return (
    <main className="layout">
        <div className="broken-box panel chapter-page">
          <div className="panel-head">
            <h2>{t("chapterConfig")}</h2>
            <span className="chapter-buttons">
              {[1, 2, 3, 4].map((n) => (
                <button key={n}
                  className={"btn small" + (chapter === n ? " applied" : "")}
                  onClick={() => setChapter(n)}>Ch.{n}</button>
              ))}
              {hasSummary && <span className={"chapter-custom" + (pending ? " pending" : "")}>★ {basedOn}</span>}
              <button className="btn small danger" disabled={!hasChanges}
                onClick={() => onSaveAll({ chapter, changes: edits })}>
                {t("save")}
              </button>
            </span>
            <button className="btn small" onClick={onBack}>← {t("back")}</button>
          </div>
          <div className="chapter-config-list wide">
            {config.items.filter((i) => i.standard !== false).map((item) => {
              const staged = hasStagedEdit(edits, item.key);
              const custom = isCustom(item, {}) && !staged;
              const shownVal = effectiveValue(item, chapter, edits);
              const shownLabel = item.options.find((option) => sameValue(option.value, shownVal))?.label
                ?? item.chValues?.[String(chapter)]?.label
                ?? String(shownVal);
              const isChosen = (option: { value: unknown }) => sameValue(option.value, shownVal);
              const previousVal = effectiveValue(item, chapter, {});
              const isPrevious = (option: { value: unknown }) => sameValue(option.value, previousVal);
              return (
                <div className={"cc-row" + (staged ? " pending" : custom ? " custom" : "")} key={item.key}>
                  <span className="cc-name" title={item.key}>
                    {isCustom(item, edits) ? "★ " : ""}{item.name ?? item.key}
                    {(item.desc || item.descEn) && (
                      <span className="cc-desc"> — {lang === "zh" ? (item.desc ?? item.descEn) : (item.descEn ?? item.desc)}</span>
                    )}
                  </span>
                  <span className="cc-control">
                    {item.options.length <= 1 && typeof shownVal === "string" ? (
                      <>
                        <input className={"cc-edit" + (staged ? " pending" : custom ? " custom" : "")} type="text"
                          value={String(shownVal)}
                          onChange={(e) => stageValue(item, e.target.value)}
                          onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
                      </>
                    ) : item.options.length <= 1 ? (
                      <span className={"cc-value" + (staged ? " pending" : custom ? " custom" : "")}>{shownLabel || "—"}</span>
                    ) : item.options.length === 2 ? (
                      <span className="cc-pair">
                        {item.options.map((o) => (
                          <button key={o.label}
                            className={"btn small"
                              + (staged
                                ? (isChosen(o) ? " pending" : isPrevious(o) ? " applied" : "")
                                : (isChosen(o) ? (custom ? " custom" : " applied") : ""))}
                            onClick={() => stageValue(item, o.value)}>
                            {o.label}
                          </button>
                        ))}
                      </span>
                    ) : (
                      <>
                        <select className={"cc-select" + (staged ? " pending" : custom ? " custom" : "")} value={String(shownVal)}
                          onChange={(e) => {
                            const o = item.options.find((x) => String(x.value) === e.target.value);
                            if (o) stageValue(item, o.value);
                          }}>
                          {item.options.map((o) => (
                            <option key={String(o.value)} value={String(o.value)}>{o.label}</option>
                          ))}
                        </select>
                      </>
                    )}
                    {(custom || staged) && (
                      <button className={"btn small" + (staged ? " pending" : " custom")}
                        title={t("overrideReset")}
                        onClick={() => resetValue(item)}>↺</button>
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

/* ---------- icon config page ---------- */

interface IconSlot {
  key: string; group: string; label: string; relPath: string;
  targetSize: [number, number] | null;
  exists: boolean; actualSize: [number, number] | null;
  thumb: string | null; path: string;
}
interface IconStatus { iconDir: string; slots: IconSlot[] }

const readFileAsDataURL = (f: File) =>
  new Promise<string>((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(String(r.result));
    r.onerror = () => reject(r.error);
    r.readAsDataURL(f);
  });

function IconConfigPage({ onBack, onFlash }: {
  onBack: () => void;
  onFlash: (msg: string, err?: boolean) => void;
}) {
  const [icon, setIcon] = useState<IconStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const targetRef = useRef<string>("");

  const load = useCallback(() => {
    invoke<IconStatus>("icon_status").then(setIcon).catch(() => setIcon(null));
  }, []);
  useEffect(() => { load(); }, [load]);

  const pick = (target: string) => {
    targetRef.current = target;
    fileRef.current?.click();
  };

  const onFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-picking the same file
    if (!file) return;
    let dataUrl: string;
    try {
      dataUrl = await readFileAsDataURL(file);
    } catch {
      onFlash(t("iconsBadImage"), true);
      return;
    }
    setBusy(true);
    try {
      if (targetRef.current === "generate") {
        await invoke("icon_generate", { dataUrl });
      } else {
        await invoke("icon_set", { key: targetRef.current, dataUrl });
      }
      onFlash(t("iconsSaved"));
      load();
    } catch (err) {
      onFlash(String(err), true);
    } finally {
      setBusy(false);
    }
  };

  const clear = async (key: string) => {
    setBusy(true);
    try {
      await invoke("icon_clear", { key });
      onFlash(t("iconsCleared"));
      load();
    } catch (err) {
      onFlash(String(err), true);
    } finally {
      setBusy(false);
    }
  };

  if (!icon) {
    return (
      <main className="layout">
        <div className="cell c1r1"><div className="broken-box panel"><p className="hint">…</p></div></div>
      </main>
    );
  }

  const groups: [string, string][] = [
    ["window", t("iconsGroupWindow")],
    ["win", t("iconsGroupWin")],
    ["android", t("iconsGroupAndroid")],
  ];
  const slotsByGroup = (g: string) => icon.slots.filter((s) => s.group === g);

  return (
    <>
    {busy && (
      <div className="loading-mask">
        <div className="loading-bar" />
      </div>
    )}
    <main className="layout">
      <div className="broken-box panel icons-page">
        <div className="panel-head">
          <h2>{t("icons")}</h2>
          <span className="icons-tools">
            <button className="btn small" onClick={() => pick("generate")} disabled={busy}>
              {t("iconsGenerate")}
            </button>
            <button className="btn small" onClick={onBack}>← {t("back")}</button>
          </span>
        </div>
        <p className="hint">{t("iconsRebuildHint")}</p>
        <p className="hint">{t("iconsScopeHint")}</p>
        <input ref={fileRef} type="file" accept="image/png,image/jpeg"
          style={{ display: "none" }} onChange={onFile} />
        {groups.map(([g, title]) => (
          <div className="icons-group" key={g}>
            <h3>{title}</h3>
            <div className="icon-grid">
              {slotsByGroup(g).map((s) => (
                <div className={"icon-slot" + (s.exists ? "" : " empty")} key={s.key}>
                  <div className="icon-thumb">
                    {s.thumb
                      ? <img src={s.thumb} alt={s.label} />
                      : <span className="icon-empty">{t("iconsEmpty")}</span>}
                  </div>
                  <span className="icon-label">{s.label}</span>
                  {s.actualSize && (
                    <span className={"icon-size" + (s.targetSize && (s.actualSize[0] !== s.targetSize[0] || s.actualSize[1] !== s.targetSize[1]) ? " warn" : "")}>
                      {s.actualSize[0]}×{s.actualSize[1]}
                    </span>
                  )}
                  <span className="icon-actions">
                    <button className="btn small" onClick={() => pick(s.key)} disabled={busy}>
                      {t("iconsPick")}
                    </button>
                    <button className="btn small danger" title={t("iconsClear")}
                      onClick={() => clear(s.key)} disabled={busy || !s.exists}>{t("iconsClear")}</button>
                  </span>
                </div>
              ))}
            </div>
            {g === "android" && <p className="hint icons-group-hint">{t("iconsAndroidHint")}</p>}
          </div>
        ))}
      </div>
    </main>
  </>
  );
}

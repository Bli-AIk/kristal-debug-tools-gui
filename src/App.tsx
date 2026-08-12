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
    customConfig: "自定义配置", configure: "配置", chapterConfig: "章节配置",
    fLang: "语言", fEncounter: "遭遇", fWave: "波次", fWaveForce: "强制波次",
    fTp: "初始 TP", fMercy: "初始 mercy", fExtra: "额外参数",
    love: "love", engine: "引擎", mod: "模组", just: "just", justEmbedded: "内置 (just crate)",
    loveMissing: "未找到 love（请安装 LÖVE 并加入 PATH）",
    noEngine: "未找到 Kristal 引擎", noMod: "未找到 mod", justNone: "不可用",
    empty: "（空）", noTasks: "没有可运行的任务",
    launchOk: "游戏已在新终端窗口中启动", taskStarted: "任务已在新终端窗口中启动",
    taskFail: "启动失败", apply: "应用", overrideDefault: "默认", overrideOn: "开", overrideOff: "关",
    overrideSaved: "已保存", overrideReset: "重置", back: "返回", chapterSaved: "默认章节已保存",
    run: "运行", refresh: "刷新",
  },
  en: {
    tasks: "RUN LIST (ADVANCED)", launch: "LAUNCH GAME", runs: "RUNS",
    project: "PROJECT", system: "system", libraries: "libraries", projectBuilds: "PROJECT BUILDS",
    initProject: "INITIALIZE PROJECT", projectName: "PROJECT NAME", initBtn: "INITIALIZE", initConfirm: "CLICK AGAIN TO CONFIRM",
    initDone: "initialization started in a terminal window — restart the GUI when done", initFail: "init failed",
    customConfig: "CUSTOM CONFIG", configure: "CONFIGURE", chapterConfig: "CHAPTER CONFIG",
    fLang: "LANGUAGE", fEncounter: "ENCOUNTER", fWave: "WAVE", fWaveForce: "WAVE FORCE",
    fTp: "TP", fMercy: "MERCY", fExtra: "EXTRA ARGS",
    love: "love", engine: "engine", mod: "mod", just: "just", justEmbedded: "builtin (just crate)",
    loveMissing: "love not found (install LÖVE and add it to PATH)",
    noEngine: "Kristal engine not found", noMod: "mod not found", justNone: "unavailable",
    empty: "(empty)", noTasks: "no runnable tasks",
    launchOk: "game launched in a new terminal window", taskStarted: "task started in a new terminal window",
    taskFail: "start failed", apply: "APPLY", overrideDefault: "default", overrideOn: "on", overrideOff: "off",
    overrideSaved: "saved", overrideReset: "reset", back: "BACK", chapterSaved: "default chapter saved",
    run: "RUN", refresh: "REFRESH",
  },
};

let lang = (localStorage.getItem("kdt-lang") ||
  (navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en")) as string;
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
}
interface TaskItem {
  name: string; doc?: string; private?: boolean;
  params?: { name: string; kind: string }[];
  aliases?: string[];
}
interface TasksResult { source: string; tasks: TaskItem[]; mod?: { tasks: TaskItem[] } | null }
interface ChapterItem {
  key: string; desc?: string; values: Record<string, unknown>;
  override?: unknown;
}
interface ChapterConfig { chapter: number; items: ChapterItem[] }
interface RunEntry { id: number; label: string; command: string }

/* ---------- helpers ---------- */

function fmtValue(v: unknown): string {
  if (v === true) return "✓";
  if (v === false) return "✗";
  if (v === null || v === undefined) return "—";
  return String(v);
}

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
    invoke<TasksResult>("tasks").then(setTasks).catch(() => setTasks(null));
    invoke<ChapterConfig>("chapter_config").then(setChapterConfig).catch(() => setChapterConfig(null));
  }, [showFlash]);

  useEffect(() => { loadAll(); }, [loadAll]);

  // High-DPI aware zoom: default scales with devicePixelRatio (>= 1.25,
  // capped at 1.6), user-adjustable via A−/A+ and persisted. Mirrors the
  // old GUI's DPR handling.
  const [scaleLabel, setScaleLabel] = useState("");
  const dprScale = () => {
    const dpr = window.devicePixelRatio || 1;
    return Math.min(1.6, Math.max(1.25, 0.6 + dpr * 0.5));
  };
  const storedScale = () => {
    const s = parseFloat(localStorage.getItem("kdt-scale") ?? "");
    return s >= 0.75 && s <= 3 ? s : dprScale();
  };
  useEffect(() => {
    document.documentElement.style.zoom = storedScale().toFixed(3);
    setScaleLabel(Math.round(storedScale() * 100) + "%");
  }, []);
  const setScale = (delta: number) => {
    const s = Math.min(3, Math.max(0.75, storedScale() + delta));
    localStorage.setItem("kdt-scale", String(s));
    document.documentElement.style.zoom = s.toFixed(3);
    setScaleLabel(Math.round(s * 100) + "%");
  };

  const addRun = (label: string, command: string) =>
    setRuns((rs) => [{ id: rs.length + 1, label, command }, ...rs].slice(0, 50));

  const overrideCount =
    chapterConfig?.items.filter((i) => i.override !== null && i.override !== undefined).length ?? 0;

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
            <button className="btn" onClick={() => {
              lang = lang === "zh" ? "en" : "zh";
              localStorage.setItem("kdt-lang", lang);
              refresh();
            }}>
              {lang === "zh" ? "EN" : "中文"}
            </button>
          </span>
        </div>
      </header>

      <div className="statusbar">
        <StatusBar status={status} />
        {flash && <span className={"flash" + (flash.err ? " err" : "")}>{flash.msg}</span>}
      </div>

      {view === "main" ? (
        <main className="layout">
          <section className="col">
            <LaunchPanel status={status} onLaunch={async (opts) => {
              try {
                await invoke("launch_game", opts);
                addRun("launch game", `love ${status?.engineRoot ?? ""} --mod ${status?.modID ?? ""}`);
                showFlash(t("launchOk"));
              } catch (e) { showFlash(t("taskFail") + ": " + e, true); }
            }} />

            {status?.template?.isTemplate && (
              <InitPanel onInit={async (name) => {
                try {
                  await invoke("template_init", { name });
                  showFlash(t("initDone"));
                  addRun("initialize project", `bash start.sh --name ${name}`);
                } catch (e) { showFlash(t("initFail") + ": " + e, true); }
              }} />
            )}

            <TaskList tasks={tasks} onRun={async (task, args, justfile) => {
              try {
                await invoke("run_task", { task, args, justfile });
                showFlash(t("taskStarted"));
                addRun(task, `just ${task} ${args.join(" ")}`);
              } catch (e) { showFlash(t("taskFail") + ": " + e, true); }
            }} onRefresh={loadAll} />
          </section>

          <section className="col">
            <ChapterEntry count={overrideCount} onOpen={() => { loadAll(); setView("chapter"); }} />
            <ProjectPanel status={status} />
            <RunsLog runs={runs} />
          </section>
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
          onSet={async (key, value) => {
            try {
              await invoke("chapter_config_set", { key, value });
              showFlash(t("overrideSaved") + ": " + key);
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
          ★ {armed ? t("initConfirm") : t("initBtn")}
        </button>
      </div>
    </div>
  );
}

/* ---------- chapter entry ---------- */

function ChapterEntry({ count, onOpen }: { count: number; onOpen: () => void }) {
  return (
    <div className="broken-box panel">
      <div className="panel-head">
        <h2>{t("chapterConfig")}</h2>
        <button className="btn small" onClick={onOpen}>{t("configure")}{count > 0 ? ` ✎${count}` : ""}</button>
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
    return (
      <div className="task-row" key={id}>
        <span className="task-name">
          {task.name}
          {task.aliases?.length ? <span className="alias"> [{task.aliases.join(", ")}]</span> : null}
        </span>
        <span className="task-doc">{task.doc ?? ""}</span>
        {task.params?.length ? (
          <span className="task-params">
            {task.params.map((p) => (
              <input key={p.name} type="text"
                placeholder={p.kind === "many" ? "a, b, c" : p.kind === "star" ? "arg1 arg2" : p.name}
                value={val} onChange={(e) => setArgs((a) => ({ ...a, [id]: e.target.value }))} />
            ))}
          </span>
        ) : null}
        <button className="btn small" onClick={() => {
          const parts = val.split(/\s+/).filter(Boolean);
          onRun(task.name, parts, justfile);
        }}>▶ {t("run")}</button>
      </div>
    );
  };

  return (
    <details className="broken-box panel" open={open} onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}>
      <summary>
        <h2>{t("tasks")}</h2>
        <button className="btn small" onClick={onRefresh}>{t("refresh")}</button>
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

function ChapterConfigPage({ config, onBack, onPickChapter, onSet }: {
  config: ChapterConfig | null;
  onBack: () => void;
  onPickChapter: (n: number) => void;
  onSet: (key: string, value: unknown) => void;
}) {
  const [selected, setSelected] = useState(0);
  const [values, setValues] = useState<Record<string, string>>({});
  useEffect(() => { if (config) setSelected(config.chapter); }, [config]);

  if (!config) {
    return <main className="layout"><section className="col"><div className="broken-box panel"><p className="hint">…</p></div></section></main>;
  }

  const pending = selected !== config.chapter && selected !== 0;

  return (
    <main className="layout">
      <section className="col">
        <div className="broken-box panel chapter-page">
          <div className="panel-head">
            <h2>{t("chapterConfig")}</h2>
            <span className="chapter-buttons">
              {[1, 2, 3, 4].map((n) => (
                <button key={n}
                  className={"btn small" + (n === config.chapter ? " applied" : "") + (n === selected && pending ? " pending" : "")}
                  onClick={() => setSelected(n)}>Ch.{n}</button>
              ))}
              <button className="btn small danger" disabled={!pending}
                onClick={() => { onPickChapter(selected); setSelected(config.chapter); }}>
                {t("apply")}
              </button>
            </span>
            <button className="btn small" onClick={onBack}>← {t("back")}</button>
          </div>
          <div className="chapter-config-list wide">
            {config.items.map((item) => (
              <div className="cc-row" key={item.key}>
                <span className="cc-key">{item.key}</span>
                <span className="cc-desc" title={item.desc}>{item.desc ?? ""}</span>
                <span className="cc-chips">
                  {[1, 2, 3, 4].map((n) => (
                    <span key={n} className={"cc-chip" + (n === config.chapter ? " cur" : "")}>
                      {n}:{item.values[n] !== undefined ? fmtValue(item.values[n]) : "—"}
                    </span>
                  ))}
                </span>
                <span className="cc-override">
                  {typeof item.values["1"] === "boolean" ? (
                    <BoolToggle item={item} onSet={onSet} />
                  ) : (
                    <>
                      <input type="text" value={values[item.key] ?? (item.override != null ? String(item.override) : "")}
                        onChange={(e) => setValues((v) => ({ ...v, [item.key]: e.target.value }))}
                        placeholder={t("overrideDefault")} />
                      <button className="btn small" onClick={() => onSet(item.key, values[item.key]?.trim() || null)}>{t("overrideSaved")}</button>
                      <button className="btn small" onClick={() => { setValues((v) => ({ ...v, [item.key]: "" })); onSet(item.key, null); }}>{t("overrideReset")}</button>
                    </>
                  )}
                </span>
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}

function BoolToggle({ item, onSet }: { item: ChapterItem; onSet: (key: string, value: unknown) => void }) {
  const [state, setState] = useState(
    item.override === true ? 1 : item.override === false ? 2 : 0,
  );
  const states: (boolean | null)[] = [null, true, false];
  const label = state === 0 ? t("overrideDefault") : state === 1 ? `${t("overrideOn")} ✓` : `${t("overrideOff")} ✗`;
  return (
    <button className={"btn small" + (state !== 0 ? " pending" : "")}
      onClick={() => {
        const next = (state + 1) % 3;
        setState(next);
        onSet(item.key, states[next]);
      }}>
      {label}
    </button>
  );
}

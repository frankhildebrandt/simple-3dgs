import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getPipelineLogPage, getPipelineLogs, listPipelineRuns } from "../api";
import { logProjectFromSearch, logText } from "../logWindow";
import type { PipelineRunSummary } from "../types";

/** Dedicated window: full pipeline log, selectable and copyable. */
export function LogWindow() {
  const [lines, setLines] = useState<string[]>([]);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [runs, setRuns] = useState<PipelineRunSummary[]>([]);
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const [nextBefore, setNextBefore] = useState<number | null>(null);
  const preRef = useRef<HTMLPreElement>(null);
  const pendingRef = useRef<string[]>([]);
  const flushRef = useRef<number | null>(null);
  const followRef = useRef(true);
  const followingCurrentRef = useRef(true);
  const projectDir = logProjectFromSearch(window.location.search);

  useEffect(() => {
    let cancelled = false;
    let stopLog: (() => void) | undefined;
    let stopReset: (() => void) | undefined;
    void (async () => {
      try {
        let snapshot: string[];
        if (projectDir) {
          const available = await listPipelineRuns(projectDir).catch(() => []);
          setRuns(available);
          const latest = available[0]?.runId ?? null;
          setSelectedRun(latest);
          if (latest) {
            const page = await getPipelineLogPage(projectDir, latest);
            snapshot = page.lines;
            setNextBefore(page.nextBefore ?? null);
          } else {
            snapshot = await getPipelineLogs();
          }
        } else {
          snapshot = await getPipelineLogs();
        }
        if (!cancelled) {
          setLines(snapshot);
          setError(null);
        }
      } catch (err: unknown) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
      const offLog = await listen<string>("pipeline-log", (event) => {
        if (!followingCurrentRef.current) return;
        pendingRef.current.push(event.payload);
        if (flushRef.current === null) {
          flushRef.current = window.setTimeout(() => {
            const batch = pendingRef.current.splice(0);
            flushRef.current = null;
            setLines((prev) => [...prev, ...batch].slice(-4000));
          }, 250);
        }
      });
      const offReset = await listen("pipeline-log-reset", () => {
        followingCurrentRef.current = true;
        setSelectedRun(null);
        setNextBefore(null);
        pendingRef.current = [];
        setLines([]);
      });
      if (cancelled) {
        offLog();
        offReset();
        return;
      }
      stopLog = offLog;
      stopReset = offReset;
    })();
    return () => {
      cancelled = true;
      stopLog?.();
      stopReset?.();
      if (flushRef.current !== null) window.clearTimeout(flushRef.current);
    };
  }, []);

  async function selectRun(runId: string) {
    if (!projectDir) return;
    try {
      const page = await getPipelineLogPage(projectDir, runId);
      followingCurrentRef.current = runId === runs[0]?.runId;
      setSelectedRun(runId);
      setLines(page.lines);
      setNextBefore(page.nextBefore ?? null);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function loadEarlier() {
    if (!projectDir || !selectedRun || nextBefore == null) return;
    try {
      const page = await getPipelineLogPage(projectDir, selectedRun, nextBefore);
      setLines((current) => [...page.lines, ...current]);
      setNextBefore(page.nextBefore ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  useEffect(() => {
    const el = preRef.current;
    if (el && followRef.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [lines]);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = window.setTimeout(() => setCopied(false), 1500);
    return () => window.clearTimeout(timer);
  }, [copied]);

  async function copyAll() {
    const text = logText(lines);
    if (!text) {
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setError(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="shell log-window">
      <div className="log-toolbar">
        <span className="log-toolbar-count">{lines.length} lines</span>
        {runs.length ? (
          <select value={selectedRun ?? ""} aria-label="Pipeline run" onChange={(event) => void selectRun(event.currentTarget.value)}>
            {runs.map((run, index) => (
              <option key={run.runId} value={run.runId}>
                {index === 0 ? "Latest" : new Date(run.modifiedMs).toLocaleString()}
              </option>
            ))}
          </select>
        ) : null}
        <button type="button" disabled={nextBefore == null} onClick={() => void loadEarlier()}>
          Earlier
        </button>
        <button type="button" disabled={lines.length === 0} onClick={() => void copyAll()}>
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      {error ? <p className="archive-error">{error}</p> : null}
      <pre
        ref={preRef}
        className="logs"
        tabIndex={0}
        aria-label="Pipeline log"
        onScroll={(event) => {
          const el = event.currentTarget;
          followRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {logText(lines) || "No log lines yet."}
      </pre>
    </div>
  );
}

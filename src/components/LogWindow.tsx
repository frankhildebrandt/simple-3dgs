import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getPipelineLogs } from "../api";
import { logText } from "../logWindow";

/** Dedicated window: full pipeline log, selectable and copyable. */
export function LogWindow() {
  const [lines, setLines] = useState<string[]>([]);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const preRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    let cancelled = false;
    let stopLog: (() => void) | undefined;
    let stopReset: (() => void) | undefined;
    void (async () => {
      try {
        const snapshot = await getPipelineLogs();
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
        setLines((prev) => [...prev, event.payload]);
      });
      const offReset = await listen("pipeline-log-reset", () => {
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
    };
  }, []);

  useEffect(() => {
    const el = preRef.current;
    if (el) {
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
        <button type="button" disabled={lines.length === 0} onClick={() => void copyAll()}>
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      {error ? <p className="archive-error">{error}</p> : null}
      <pre ref={preRef} className="logs" tabIndex={0} aria-label="Pipeline log">
        {logText(lines) || "No log lines yet."}
      </pre>
    </div>
  );
}

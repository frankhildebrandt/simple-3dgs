import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getArchive, setArchivePoster } from "../api";
import { toggleNativeFullscreen, watchNativeFullscreen } from "../fullscreen";
import type { ArchiveEntry } from "../types";
import { SplatViewer } from "./SplatViewer";

type Props = {
  splatId: string;
};

/** Dedicated window: splat viewer plus preview capture, no archive chrome. */
export function SplatWindow({ splatId }: Props) {
  const [entry, setEntry] = useState<ArchiveEntry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getArchive(splatId)
      .then((next) => {
        if (!cancelled) {
          setEntry(next);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [splatId]);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void listen<string>("archive-changed", (event) => {
      if (event.payload !== splatId) {
        return;
      }
      void getArchive(splatId)
        .then((next) => {
          if (!cancelled) {
            setEntry(next);
            setError(null);
          }
        })
        .catch((err: unknown) => {
          if (!cancelled) {
            setError(err instanceof Error ? err.message : String(err));
          }
        });
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [splatId]);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void watchNativeFullscreen(setFullscreen).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  if (error) {
    return (
      <div className="shell splat-window">
        <p className="archive-empty">{error}</p>
      </div>
    );
  }
  if (!entry) {
    return (
      <div className="shell splat-window">
        <p className="archive-empty">Loading splat…</p>
      </div>
    );
  }

  return (
    <div className={fullscreen ? "shell viewer-fullscreen splat-window" : "shell splat-window"}>
      <SplatViewer
        key={`${entry.id}:${entry.plyPath}`}
        plyPath={entry.plyPath}
        captureMode={entry.settings?.captureMode ?? "object"}
        fullscreen={fullscreen}
        onToggleFullscreen={() => {
          void toggleNativeFullscreen().then(setFullscreen);
        }}
        onSetPreview={async (jpegBase64) => {
          await setArchivePoster(entry.id, jpegBase64);
        }}
      />
    </div>
  );
}

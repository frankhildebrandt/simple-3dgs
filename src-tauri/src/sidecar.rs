//! Sidecar command specs and process runner. Tests use [`FakeRunner`].

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::error::PipelineError;
use crate::settings::CaptureMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub sidecar: &'static str,
    pub args: Vec<String>,
    pub watch_dir: Option<PathBuf>,
    pub capture_mode: CaptureMode,
}

impl CommandSpec {
    pub fn new(sidecar: &'static str, args: Vec<String>) -> Self {
        Self {
            sidecar,
            args,
            watch_dir: None,
            capture_mode: CaptureMode::Object,
        }
    }

    pub fn watching(mut self, dir: impl Into<PathBuf>) -> Self {
        self.watch_dir = Some(dir.into());
        self
    }

    /// Attaches the capture mode so COLMAP failures can hint the right reshoot advice.
    pub fn capture(mut self, mode: CaptureMode) -> Self {
        self.capture_mode = mode;
        self
    }
}

pub fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub trait SidecarRunner {
    fn run(
        &mut self,
        spec: &CommandSpec,
        log: &mut dyn FnMut(&str),
        preview: &mut dyn FnMut(&Path),
    ) -> Result<(), PipelineError>;
}

#[derive(Clone, Default)]
pub struct CancelFlag {
    inner: Arc<AtomicBool>,
}

impl CancelFlag {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.inner.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), PipelineError> {
        if self.is_cancelled() {
            Err(PipelineError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Resolves bundled sidecars, then PATH, then common Brush binary names.
pub fn resolve_binary(name: &str) -> Result<PathBuf, PipelineError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(name);
            if is_executable(&candidate) {
                return Ok(candidate);
            }
        }
    }

    let triple = "aarch64-apple-darwin";
    let manifest_candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!("{name}-{triple}")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(name),
    ];
    for candidate in manifest_candidates {
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }

    if let Some(path) = which(name) {
        return Ok(path);
    }
    if name == "brush" {
        if let Some(path) = which("brush-cli") {
            return Ok(path);
        }
    }

    Err(PipelineError::message(format!(
        "Could not find `{name}`. Run scripts/fetch-sidecars.sh to bundle it."
    )))
}

pub struct ProcessRunner {
    cancel: CancelFlag,
}

impl ProcessRunner {
    pub fn new(cancel: CancelFlag) -> Self {
        Self { cancel }
    }
}

impl SidecarRunner for ProcessRunner {
    fn run(
        &mut self,
        spec: &CommandSpec,
        log: &mut dyn FnMut(&str),
        preview: &mut dyn FnMut(&Path),
    ) -> Result<(), PipelineError> {
        self.cancel.check()?;
        let bin = resolve_binary(spec.sidecar)?;
        log(&format!("$ {} {}", spec.sidecar, spec.args.join(" ")));

        let mut command = Command::new(&bin);
        command.args(&spec.args);
        if spec.sidecar == "brush" {
            // Brush's env_logger defaults to error; info is required to see train metrics.
            command.env("RUST_LOG", "info");
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                PipelineError::message(format!("failed to start {}: {err}", spec.sidecar))
            })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        if let Some(out) = stdout {
            let tx = tx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(out);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(line);
                }
            });
        }
        if let Some(err) = stderr {
            let tx = tx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(err);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(line);
                }
            });
        }
        drop(tx);

        let mut tail = Vec::new();
        let mut last_preview: Option<PathBuf> = None;
        let mut ticks: u32 = 0;
        let status = loop {
            self.cancel.check().map_err(|err| {
                let _ = child.kill();
                let _ = child.wait();
                err
            })?;
            match child.try_wait()? {
                Some(status) => break status,
                None => {
                    while let Ok(line) = rx.try_recv() {
                        remember(&mut tail, log, line);
                    }
                    ticks = ticks.saturating_add(1);
                    if ticks % 10 == 0 {
                        emit_preview(spec, &mut last_preview, preview);
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        };
        while let Ok(line) = rx.try_recv() {
            remember(&mut tail, log, line);
        }
        emit_preview(spec, &mut last_preview, preview);

        if status.success() {
            Ok(())
        } else {
            let code = status.code().unwrap_or(-1);
            let detail = tail.join("\n");
            Err(match spec.sidecar {
                "ffmpeg" => PipelineError::ffmpeg_failed(code),
                "colmap" => PipelineError::colmap_failed_with(code, &detail, spec.capture_mode),
                "brush" => PipelineError::brush_failed(code),
                other => PipelineError::Sidecar {
                    tool: other.into(),
                    code,
                    hint: "The sidecar exited with an error.".into(),
                },
            })
        }
    }
}

fn remember(tail: &mut Vec<String>, log: &mut dyn FnMut(&str), line: String) {
    log(&line);
    tail.push(line);
    if tail.len() > 40 {
        tail.remove(0);
    }
}

fn emit_preview(spec: &CommandSpec, last: &mut Option<PathBuf>, preview: &mut dyn FnMut(&Path)) {
    let Some(dir) = spec.watch_dir.as_ref() else {
        return;
    };
    let Some(path) = crate::project::newest_ready_ply(dir) else {
        return;
    };
    if last.as_ref() == Some(&path) {
        return;
    }
    *last = Some(path.clone());
    preview(&path);
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Test double that records specs and materializes fake COLMAP/PLY artifacts.
#[cfg(test)]
pub struct FakeRunner {
    pub calls: Vec<CommandSpec>,
    pub fail_on: Option<&'static str>,
}

#[cfg(test)]
impl FakeRunner {
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            fail_on: None,
        }
    }
}

#[cfg(test)]
impl Default for FakeRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl SidecarRunner for FakeRunner {
    fn run(
        &mut self,
        spec: &CommandSpec,
        log: &mut dyn FnMut(&str),
        preview: &mut dyn FnMut(&Path),
    ) -> Result<(), PipelineError> {
        log(&format!("fake {} {}", spec.sidecar, spec.args.join(" ")));
        self.calls.push(spec.clone());
        if self.fail_on == Some(spec.sidecar) {
            return Err(match spec.sidecar {
                "ffmpeg" => PipelineError::ffmpeg_failed(1),
                "colmap" => PipelineError::colmap_failed_with(1, "", spec.capture_mode),
                "brush" => PipelineError::brush_failed(1),
                other => PipelineError::message(format!("{other} failed")),
            });
        }
        match spec.sidecar {
            "ffmpeg" => {
                if spec.args.iter().any(|a| a == "ffmetadata") {
                    log("com.apple.quicktime.location.ISO6709=+52.520008+013.404954/");
                } else {
                    write_fake_frames(spec)?;
                }
            }
            "colmap" => write_fake_colmap(spec)?,
            "brush" => {
                write_fake_ply(spec)?;
                log("Refine iter 1, 8 splats.");
            }
            _ => {}
        }
        if spec.sidecar == "brush" {
            if let Some(path) = spec.watch_dir.as_ref().and_then(|dir| {
                crate::project::newest_ready_ply(dir).or_else(|| crate::project::newest_ply(dir))
            }) {
                preview(&path);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
fn write_fake_frames(spec: &CommandSpec) -> Result<(), PipelineError> {
    let Some(pattern) = spec.args.last() else {
        return Ok(());
    };
    let path = Path::new(pattern);
    let dir = path.parent().unwrap_or(Path::new("."));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    std::fs::create_dir_all(dir)?;
    for i in 1..=8 {
        std::fs::write(dir.join(format!("frame_{i:05}.{ext}")), b"fake-frame")?;
    }
    Ok(())
}

#[cfg(test)]
fn write_fake_colmap(spec: &CommandSpec) -> Result<(), PipelineError> {
    match spec.args.first().map(String::as_str) {
        Some("feature_extractor") | Some("sequential_matcher") | Some("exhaustive_matcher") => {
            if let Some(db) = arg_value(&spec.args, "--database_path") {
                let path = Path::new(db);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if !path.is_file() {
                    std::fs::write(path, b"fake-colmap-db")?;
                }
            }
            return Ok(());
        }
        Some("mapper") | Some("global_mapper") => {}
        _ => return Ok(()),
    }
    let output = arg_value(&spec.args, "--output_path")
        .ok_or_else(|| PipelineError::message("mapper missing --output_path"))?;
    let model = Path::new(output).join("0");
    std::fs::create_dir_all(&model)?;
    for name in ["cameras.bin", "images.bin", "points3D.bin"] {
        std::fs::write(model.join(name), name.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
fn write_fake_ply(spec: &CommandSpec) -> Result<(), PipelineError> {
    let export = arg_value(&spec.args, "--export-path").unwrap_or(".");
    let name = arg_value(&spec.args, "--export-name").unwrap_or(crate::project::OUTPUT_PLY);
    let name = name.replace("{iter}", "1");
    let dir = Path::new(export);
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(name), b"ply\nfake\n")?;
    Ok(())
}

#[cfg(test)]
fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_runner_records_calls() {
        let mut runner = FakeRunner::new();
        let dir = tempfile::tempdir().unwrap();
        let pattern = dir.path().join("frame_%05d.jpg");
        let spec = CommandSpec::new("ffmpeg", vec![path_arg(&pattern)]);
        let mut logs = Vec::new();
        runner
            .run(&spec, &mut |line| logs.push(line.to_string()), &mut |_| {})
            .unwrap();
        assert_eq!(runner.calls.len(), 1);
        assert_eq!(crate::project::count_frames(dir.path()).unwrap(), 8);
        assert!(!logs.is_empty());
    }
}

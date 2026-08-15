//! Drive otool / install_name_tool / codesign and copy Qt frameworks into `colmap-libs`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::copy::copy_dir;
use crate::framework::{
    brew_framework_binary, parse_framework_ref, source_framework_dir, FrameworkRef,
};
use crate::otool::{parse_install_names, parse_rpaths};
use crate::rpath::{rpaths_to_add, rpaths_to_delete};

pub fn bundle(binary: &Path, libdir: &Path) -> io::Result<()> {
    fs::create_dir_all(libdir)?;
    let mut pending: Vec<PathBuf> = vec![binary.to_path_buf()];
    pending.extend(dylibs_in(libdir)?);
    let mut seen = Vec::new();

    while let Some(path) = pending.pop() {
        if seen.iter().any(|p| p == &path) || !path.exists() {
            continue;
        }
        seen.push(path.clone());
        rewrite_file(&path, libdir)?;
        for lib in list_install_names(&path)? {
            if let Some(framework) = parse_framework_ref(&lib) {
                copy_framework(&lib, &framework, libdir)?;
                let bundled = framework.bundled_binary(libdir);
                if bundled.exists() {
                    pending.push(bundled);
                }
            }
        }
    }

    add_missing_rpaths(
        binary,
        &[
            "@executable_path/colmap-libs",
            "@executable_path/../Resources/colmap-libs",
        ],
    )?;
    codesign(binary)?;
    Ok(())
}

fn rewrite_file(path: &Path, libdir: &Path) -> io::Result<()> {
    for rpath in rpaths_to_delete(&list_rpaths(path)?) {
        let _ = run(&["install_name_tool", "-delete_rpath", &rpath, &path_str(path)]);
    }
    for lib in list_install_names(path)? {
        if let Some(framework) = parse_framework_ref(&lib) {
            if let Some(new) = copy_framework(&lib, &framework, libdir)? {
                let _ = run(&[
                    "install_name_tool",
                    "-change",
                    &lib,
                    &new,
                    &path_str(path),
                ]);
            }
        }
    }
    if path.extension().and_then(|e| e.to_str()) == Some("dylib") {
        add_missing_rpaths(path, &["@loader_path"])?;
    }
    codesign(path)?;
    Ok(())
}

fn add_missing_rpaths(path: &Path, wanted: &[&str]) -> io::Result<()> {
    for rpath in rpaths_to_add(&list_rpaths(path)?, wanted) {
        run(&[
            "install_name_tool",
            "-add_rpath",
            &rpath,
            &path_str(path),
        ])?;
    }
    Ok(())
}

fn copy_framework(
    install_name: &str,
    framework: &FrameworkRef,
    libdir: &Path,
) -> io::Result<Option<String>> {
    let dest = libdir.join(&framework.name);
    if !dest.exists() {
        let src = source_framework_dir(install_name)
            .or_else(|| {
                brew_framework_binary(framework).and_then(|bin| {
                    bin.ancestors()
                        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("framework"))
                        .map(Path::to_path_buf)
                })
            });
        let Some(src) = src else {
            return Ok(None);
        };
        if !src.is_dir() {
            return Ok(None);
        }
        copy_dir(&src, &dest)?;
    }
    Ok(Some(framework.rpath_install_name()))
}

fn dylibs_in(libdir: &Path) -> io::Result<Vec<PathBuf>> {
    if !libdir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(libdir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("dylib") {
            files.push(path);
        }
    }
    Ok(files)
}

fn list_install_names(path: &Path) -> io::Result<Vec<String>> {
    let stdout = tool_stdout(&["otool", "-L", &path_str(path)])?;
    Ok(parse_install_names(&stdout))
}

fn list_rpaths(path: &Path) -> io::Result<Vec<String>> {
    let stdout = tool_stdout(&["otool", "-l", &path_str(path)])?;
    Ok(parse_rpaths(&stdout))
}

fn codesign(path: &Path) -> io::Result<()> {
    run(&["codesign", "--force", "--sign", "-", &path_str(path)])
}

fn tool_stdout(args: &[&str]) -> io::Result<String> {
    let output = Command::new(args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{} failed: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run(args: &[&str]) -> io::Result<()> {
    let status = Command::new(args[0]).args(&args[1..]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{} exited {}", args[0], status)))
    }
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

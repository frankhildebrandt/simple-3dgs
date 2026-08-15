mod copy;
mod framework;
mod otool;
mod rewrite;
mod rpath;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(binary) = args.next() else {
        eprintln!("usage: colmap-bundle COLMAP_BINARY COLMAP_LIBS_DIR");
        return ExitCode::from(2);
    };
    let Some(libdir) = args.next() else {
        eprintln!("usage: colmap-bundle COLMAP_BINARY COLMAP_LIBS_DIR");
        return ExitCode::from(2);
    };

    match rewrite::bundle(&PathBuf::from(binary), &PathBuf::from(libdir)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("colmap-bundle: {err}");
            ExitCode::FAILURE
        }
    }
}

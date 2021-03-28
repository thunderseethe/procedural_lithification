use std::{ffi::OsString, fs::*, process::Child};
use std::io;
use std::io::Write;
use which::which;

fn main() -> io::Result<()> {
    use std::io::{Error, ErrorKind};
    let mut childs: Vec<(OsString, Child)> = Vec::new();

    let npm = which("npm")
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut base_cmd = std::process::Command::new(npm);
    base_cmd.args(&["run", "asbuild"])
            .envs(std::env::vars());
    for dir_res in read_dir("mods")?.into_iter() {
        let entry = dir_res?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let assembly = dir.join("assembly").canonicalize()?;
        println!("cargo:rerun-if-changed={}", assembly.display());

        let child = base_cmd.current_dir(dir).spawn()?;
    
        childs.push((entry.file_name(), child));
    }

    for (dir, child) in childs {
        let output = child.wait_with_output()?;
        let dir_str = dir.into_string().unwrap();
        if !output.stdout.is_empty() {
            io::stdout().write_fmt(format_args!("{}\n=========================\n", dir_str))?;
            io::stdout().write_all(&output.stdout)?;
        }
        if !output.status.success() {
            let s = String::from_utf8(output.stderr).unwrap();
            return Err(io::Error::new(io::ErrorKind::Other,
                 format_args!("{} failed to compile.\n{}", dir_str, s).to_string()));
        }
    }

    Ok(())
}
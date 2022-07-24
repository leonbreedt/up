use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, fs, io};

const BUN: &str = "bun";
const NPM: &str = "npm";
const UI_PROJECT_DIR: &str = "../up-ui";

#[derive(Clone)]
enum BuildType {
    Development,
    Production,
}

impl FromStr for BuildType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "debug" => Self::Development,
            "release" => Self::Production,
            _ => Self::Development,
        })
    }
}

impl ToString for BuildType {
    fn to_string(&self) -> String {
        match self {
            BuildType::Development => "development".to_string(),
            BuildType::Production => "production".to_string(),
        }
    }
}

trait UIBuilder {
    fn build(&self, build_type: BuildType) -> Result<(), io::Error>;
}

fn main() {
    print_rerun_if_changed(UI_PROJECT_DIR, &["node_modules", "out"])
        .expect("failed to produce cargo:rerun-if-changed file list");

    let cargo_profile = &env::var("PROFILE").unwrap()[..];
    let builder = new_ui_builder();

    builder
        .build(cargo_profile.parse().expect("unsupported Cargo profile"))
        .expect("failed to build UI project");
}

fn new_ui_builder() -> Box<dyn UIBuilder> {
    let exe_path = find_first_matching_executable(&[BUN, NPM])
        .unwrap_or_else(|| panic!("at least '{}' or '{}' must be installed", BUN, NPM));
    if exe_path.ends_with(BUN) {
        Box::new(BunBuilder)
    } else {
        Box::new(NpmBuilder)
    }
}

struct NpmBuilder;

impl UIBuilder for NpmBuilder {
    fn build(&self, build_type: BuildType) -> Result<(), io::Error> {
        if !dir_exists(Path::new(UI_PROJECT_DIR).join("node_modules")) {
            run_builder_exe(NPM, build_type.clone(), &["install"])?;
        }
        run_builder_exe(NPM, build_type, &["run", "build"])
    }
}

struct BunBuilder;

impl UIBuilder for BunBuilder {
    fn build(&self, build_type: BuildType) -> Result<(), io::Error> {
        if !dir_exists(Path::new(UI_PROJECT_DIR).join("node_modules")) {
            run_builder_exe(BUN, build_type.clone(), &["install"])?;
        }
        run_builder_exe(BUN, build_type, &["run", "build"])
    }
}

fn run_builder_exe<P, A>(exe: P, build_type: BuildType, args: &[A]) -> Result<(), io::Error>
where
    P: AsRef<OsStr>,
    A: AsRef<OsStr>,
{
    let mut cmd = Command::new(exe.as_ref());
    cmd.env("NODE_ENV", build_type.to_string());
    cmd.args(args);
    cmd.current_dir(UI_PROJECT_DIR);
    let status = cmd.status()?;
    if !status.success() {
        Err(io::Error::from_raw_os_error(status.code().unwrap()))
    } else {
        Ok(())
    }
}

fn which<P>(name: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    if let Some(paths) = env::var_os("PATH") {
        env::split_paths(&paths)
            .filter_map(|path_dir| {
                let test_file = path_dir.join(&name);
                if test_file.is_file() {
                    Some(test_file)
                } else {
                    None
                }
            })
            .next()
    } else {
        None
    }
}

fn find_first_matching_executable<E>(names: &[E]) -> Option<PathBuf>
where
    E: AsRef<OsStr>,
{
    for name in names {
        if let Some(path) = which(name.as_ref()) {
            return Some(path);
        }
    }
    None
}

fn dir_exists<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    path.as_ref().is_dir()
}

fn print_rerun_if_changed<P>(base_dir: P, excludes: &[P]) -> Result<(), io::Error>
where
    P: AsRef<Path>,
{
    let base_dir = base_dir.as_ref().canonicalize().unwrap();
    let exclude_paths: Vec<PathBuf> = excludes.iter().map(|p| base_dir.join(p.as_ref())).collect();

    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Ignore hidden files
        if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
            if file_name.starts_with('.') {
                continue;
            }
        }

        // Ignore excluded paths
        if exclude_paths.iter().any(|i| path.starts_with(i)) {
            continue;
        }

        if path.is_dir() {
            print_rerun_if_changed(path, &exclude_paths)?;
        } else if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }

    Ok(())
}

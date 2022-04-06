use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use duct::cmd;

use crate::config::{ProjectType, WorkspaceConfig};
use crate::opts::*;
use crate::util::display_cmd_hook;
use crate::{driver, Project};

const DRIVERS_ROOT: &'static str = "drivers";

fn cargo_build(project: &str, cargo_args: Vec<String>, rustc_args: Vec<String>) {
    println!("==> Building {}", project);

    let mut args = vec!["rustc".into(), "-p".into(), project.to_string()];
    args.extend(cargo_args);
    args.push("--".into());
    args.extend(rustc_args);

    cmd("cargo", args)
        .before_spawn(display_cmd_hook)
        .run()
        .unwrap();
}

fn find_all_drivers() -> impl Iterator<Item = PathBuf> {
    fs::read_dir(DRIVERS_ROOT)
        .expect("Could not open drivers dir")
        .filter_map(|dir| {
            let dir = dir.ok()?;
            dir.file_type().ok()?.is_dir().then(move || {
                fs::read_dir(dir.path())
                    .expect(&format!("Could not open {:?} dir", dir.path()))
                    .map(|dir| dir.unwrap().path())
            })
        })
        .flatten()
}

fn get_args_for_project(
    opts: &BuildOpts,
    workspace: &WorkspaceConfig,
    proj: &Project,
) -> (Vec<String>, Vec<String>) {
    let mut cargo_args = Vec::new();
    if opts.release {
        cargo_args.push("--release".into());
    }
    cargo_args.push("--target".into());
    cargo_args.push(proj.target.value().into());
    cargo_args.extend(proj.flags.flags.clone());
    cargo_args.extend(opts.cargo_args.iter().cloned());

    let mut rustc_args = Vec::new();
    if let Some(linker_script) = proj.flags.linker_script.as_deref() {
        rustc_args.push(format!(
            "-Clink-args=-T{}",
            proj.path.join(linker_script).to_string_lossy()
        ));
    }
    rustc_args.extend(proj.flags.rustc_flags.clone());
    rustc_args.extend(opts.rustc_args.iter().cloned());

    (cargo_args, rustc_args)
}

fn build_drivers(opts: &BuildOpts, workspace: &WorkspaceConfig, kernel: &Project) ->  Vec<PathBuf> {
    let mut static_drivers = HashSet::new();
    static_drivers.insert(workspace.static_drivers.initrd.clone());
    static_drivers.extend(workspace.static_drivers.others.iter().cloned());
    let mut initrd_drivers: HashSet<_> = workspace.initrd_drivers.iter().cloned().collect();

    let mut initrd_paths = Vec::new();

    let (kernel_cargo_args, kernel_rustc_args) = get_args_for_project(opts, workspace, kernel);

    for driver_path in find_all_drivers() {
        let config =
            cargo_toml::Manifest::from_path(driver_path.join("Cargo.toml")).expect(&format!(
                "Could not open {}/Cargo.toml",
                driver_path.to_string_lossy()
            ));
        let name = &config
            .package
            .as_ref()
            .expect("Should be a normal project")
            .name;

        let lib_type;
        if static_drivers.remove(name) {
            lib_type = "rlib";
        } else if initrd_drivers.remove(name) {
            lib_type = "dylib";
            initrd_paths.push(
                Path::new(&format!("target/{}/{}.elf", kernel.target.name(), name)).to_path_buf(),
            );
        } else {
            lib_type = "dylib";
        }

        let mut cargo_args = kernel_cargo_args.clone();

        let mut rustc_args = kernel_rustc_args.clone();
        rustc_args.push("--crate-type".into());
        rustc_args.push(lib_type.into());

        cargo_build(name, cargo_args, rustc_args);
    }

    if !initrd_drivers.is_empty() {
        panic!("Could not find initrd drivers {:?}", initrd_drivers);
    }

    if !static_drivers.is_empty() {
        panic!("Could not find static drivers {:?}", static_drivers);
    }

    initrd_paths
}

const KERNEL_MAIN_RS: &'static str = include_str!("../static/kernel-main.rs");
const STATIC_CRATES_KEY: &'static str = "%STATIC_CRATES%";
const KERNEL_MAIN_PATH: &'static str = "kernel/src/main.rs";

pub fn build_main(
    opts: &BuildOpts,
    workspace: &WorkspaceConfig,
    projects: &[Project],
) -> Vec<PathBuf> {
    let mut driver_paths = None;
    for proj in projects {
        let (mut cargo_args, mut rustc_args) = get_args_for_project(opts, workspace, proj);
        if proj.typ == ProjectType::Kernel {
            let drv_driver_paths = build_drivers(opts, workspace, proj);
            driver_paths = Some(drv_driver_paths);
        }

        cargo_build(&proj.name, cargo_args, rustc_args)
    }
    driver_paths.expect("Kernel not built")
}

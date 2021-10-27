#![feature(backtrace)]

#[cfg(not(target_os = "linux"))]
compile_error!("Only works on Linux");

mod build;
use build::*;

mod clean;
use clean::*;

mod consts;
use consts::*;

mod deploy;
use deploy::*;

mod config;
use config::*;

mod lint;
use lint::*;

mod project;
use project::*;

mod opts;
use opts::*;

mod run;
use run::*;

mod testing;
use testing::*;

mod util;
use util::*;

use std::error::Error;
use std::path::{Path, PathBuf};

use cargo_toml::Manifest;

use structopt::StructOpt;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn load_project(
    opts: &BuildOpts,
    builder: &mut ProjectBuilder,
    path: impl AsRef<Path>,
) -> Result<()> {
    let dir = path.as_ref().parent().unwrap_or(Path::new("/"));
    let config = read_project_config(path.as_ref())?;
    builder.merge_flags(dir, &config.project.common);
    if opts.release {
        builder.merge_flags(dir, &config.project.release);
    } else {
        builder.merge_flags(dir, &config.project.debug);
    }
    for include in &config.meta.include {
        let include = include
            .to_string_lossy()
            .as_ref()
            .replace("${ARCH}", &opts.arch);
        load_project(
            opts,
            builder,
            [dir, Path::new(&include)].iter().collect::<PathBuf>(),
        )?;
    }
    Ok(())
}

fn get_projects(opts: &BuildOpts) -> Result<Vec<Project>> {
    let config = read_root_config(ROOT_CONFIG_PATH).unwrap();
    config
        .projects
        .iter()
        .map(|(name, proj)| {
            let cargo_toml = Manifest::from_path(
                [&proj.path, Path::new("Cargo.toml")]
                    .iter()
                    .collect::<PathBuf>(),
            )?;
            let cargo_name = &cargo_toml
                .package
                .as_ref()
                .ok_or("Package should contain a name")?
                .name;
            let mut proj_builder = ProjectBuilder::new(name.clone(), cargo_name, proj.path.clone());
            proj_builder.merge_flags(".", &config.flags.common);
            if opts.release {
                proj_builder.merge_flags(".", &config.flags.release);
            } else {
                proj_builder.merge_flags(".", &config.flags.debug);
            }
            load_project(
                &opts,
                &mut proj_builder,
                [&proj.path, Path::new(PROJECT_CONFIG_NAME)]
                    .iter()
                    .collect::<PathBuf>(),
            )?;
            Ok(proj_builder.finish()?)
        })
        .collect()
}

fn main() {
    let opts = Opts::from_args();
    match opts {
        Opts::Build(opts) => {
            let config = get_projects(&opts).unwrap();
            build_main(&opts, &config);
        }
        Opts::Deploy(opts) => {
            let config = get_projects(&opts.build).unwrap();
            deploy_main(&opts, &config);
        }
        Opts::Run(opts) => {
            let config = get_projects(&opts.build).unwrap();
            run_main(&opts, &config);
        },
        Opts::Test(opts) => {
            let config = get_projects(&opts.build).unwrap();
            test_main(&opts, &config)
        },
        Opts::Lint(opts) => {
            let config = get_projects(&opts.build).unwrap();
            lint_main(&opts, &config);
        },
        Opts::Clean => clean_main(),
    }
}

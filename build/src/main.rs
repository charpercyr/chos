#![feature(once_cell)]
#![feature(path_file_prefix)]

mod build;
mod clean;
mod config;
mod consts;
mod deploy;
mod opts;
mod run;
mod util;

use std::lazy::SyncLazy;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use build::build_main;
use cargo_toml::Manifest;
use clean::clean_main;
use config::TargetMatch;
use deploy::deploy_main;
use run::run_main;
use structopt::StructOpt;
use util::Target;

static ROOT_CONFIG: SyncLazy<PathBuf> =
    SyncLazy::new(|| PathBuf::from_str("./Cargo.toml").unwrap());

const DEBUG_STR: &'static str = "debug";
const RELEASE_STR: &'static str = "release";

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn expand_glob(path: &Path) -> impl Iterator<Item = PathBuf> {
    // TODO support globs
    vec![path.to_path_buf()].into_iter()
}

fn find_all_projects(workspace: &config::WorkspaceRoot, target_match: TargetMatch) -> Vec<Project> {
    workspace
        .chos
        .projects
        .iter()
        .map(|p| expand_glob(Path::new(p)))
        .flatten()
        .filter_map(move |proj| {
            let mut path = PathBuf::new();
            path.push(proj.clone());
            path.push("Cargo.toml");
            let manifest = Manifest::<config::ProjectRoot>::from_path_with_metadata(&path)
                .expect("Invalid configuration");
            let package = manifest.package?;
            let name = package.name.clone();
            let config = package.metadata?.chos?;
            let flags = workspace
                .chos
                .flags
                .get_flags(target_match)
                .merge(&config.flags.get_flags(target_match));
            let target =
                Target::from_base_str(&proj, flags.target.as_ref().expect("Target must be set"));
            Some(Project {
                name,
                path: proj,
                flags,
                target,
            })
        })
        .collect()
}

fn target_from_opts(opts: &opts::BuildOpts) -> config::TargetMatch<'_> {
    config::TargetMatch {
        arch: &opts.arch,
        profile: if opts.release { RELEASE_STR } else { DEBUG_STR },
    }
}

#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub flags: config::Flags,
    pub target: Target,
}

fn main() {
    let manifest: Manifest<config::WorkspaceRoot> =
        Manifest::from_path_with_metadata(&*ROOT_CONFIG).unwrap();
    let workspace = manifest
        .workspace
        .expect("Root project should be a workspace")
        .metadata
        .expect("Root project should have chos config");
    let opts = opts::Opts::from_args();
    match opts {
        opts::Opts::Build(opts) => build_main(
            &opts,
            &find_all_projects(&workspace, target_from_opts(&opts)),
        ),
        opts::Opts::Deploy(opts) => deploy_main(
            &opts,
            &find_all_projects(&workspace, target_from_opts(&opts.build)),
        ),
        opts::Opts::Run(opts) => run_main(
            &opts,
            &find_all_projects(&workspace, target_from_opts(&opts.build)),
        ),
        opts::Opts::Test(opts) => todo!("Test for {:?}", opts),
        opts::Opts::Clean => clean_main(),
    }
}

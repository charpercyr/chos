
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct BuildOpts {
    #[structopt(default_value = "x86_64", long)]
    pub arch: String,
    /// Release build
    #[structopt(long)]
    pub release: bool,
    #[structopt(long)]
    pub cargo_args: Vec<String>,
    #[structopt(long)]
    pub rustc_args: Vec<String>,
}

#[derive(StructOpt, Debug)]
pub struct DeployOpts {
    #[structopt(flatten)]
    pub build: BuildOpts,
    /// Image size
    #[structopt(long)]
    pub image_size: Option<usize>,
    /// Output path
    pub output: String,
}

#[derive(StructOpt, Debug)]
pub struct RunOpts {
    #[structopt(flatten)]
    pub build: BuildOpts,
    /// Start qemu in debug mode
    #[structopt(long, short = "d")]
    pub debug: bool,
    #[structopt(long, short, default_value = "4G")]
    pub mem: String,
    #[structopt(long, short, default_value = "2")]
    pub smp: usize,
}

#[derive(StructOpt, Debug)]
pub enum Opts {
    /// Build project
    #[structopt(visible_alias = "b")]
    Build(BuildOpts),
    /// Build and deploy project
    #[structopt(visible_alias = "d")]
    Deploy(DeployOpts),
    /// Build and run project
    #[structopt(visible_alias = "r")]
    Run(RunOpts),
    /// Clean project
    #[structopt(visible_alias = "c")]
    Clean,
}

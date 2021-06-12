
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct BuildOpts {
    #[structopt(default_value = "x86_64", long)]
    pub arch: String,
    /// Release build
    #[structopt(long)]
    pub release: bool,
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

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
    /// Don't rebuild before deploying
    #[structopt(long)]
    pub no_build: bool,
    /// Don't use KVM acceleration
    #[structopt(long)]
    pub no_kvm: bool,
    /// Start QEMU with monitor set to 'curses' and serial to 'none'
    #[structopt(long)]
    pub curses: bool,
}

#[derive(StructOpt, Debug)]
pub struct TestOpts {
    #[structopt(flatten)]
    pub build: BuildOpts,
    #[structopt(long, short = "p")]
    pub packages: Option<Vec<String>>,
    pub filters: Vec<String>,
}

#[derive(StructOpt, Debug)]
pub struct LintOpts {
    #[structopt(flatten)]
    pub build: BuildOpts,
    #[structopt(long)]
    pub clippy_args: Option<Vec<String>>,
}

#[derive(StructOpt, Debug)]
pub struct NewDriverOpts {
    /// Driver category (e.g. fs, net, misc, ...)
    pub category: String,
    /// Driver name
    pub name: String,
    /// Compile statically into the kernel (excludes '--initrd')
    #[structopt(long = "static")]
    pub static_: bool,
    #[structopt(long)]
    /// Add to initrd (excludes '--static')
    pub initrd: bool,
}

#[derive(StructOpt, Debug)]
pub enum DriverOpts {
    New(NewDriverOpts),
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
    /// Driver projects management
    Driver(DriverOpts),
    /// Clean project
    #[structopt(visible_alias = "c")]
    Clean,
}

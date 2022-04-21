use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspaceRoot {
    pub chos: WorkspaceConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct StaticDriver {
    pub ramfs: String,
    #[serde(default)]
    pub others: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceConfig {
    pub projects: Vec<String>,
    #[serde(default, flatten)]
    pub flags: ProjectFlags,
    pub static_drivers: StaticDriver,
    #[serde(default)]
    pub initrd_drivers: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProjectRoot {
    pub chos: Option<ProjectConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Boot,
    Kernel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProjectConfig {
    #[serde(rename = "type")]
    pub typ: ProjectType,
    #[serde(default, flatten)]
    pub flags: ProjectFlags,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Flags {
    pub target: Option<String>,
    pub linker_script: Option<PathBuf>,
    pub deploy: Option<(PathBuf, String)>,
    pub initrd: Option<PathBuf>,
    #[serde(default)]
    pub copy: Vec<(PathBuf, String)>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub rustc_flags: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProjectFlags {
    #[serde(default, flatten)]
    pub flags: Flags,
    #[serde(default, flatten)]
    pub configs: HashMap<String, ProjectFlags>,
}

#[derive(Clone, Copy, Debug)]
pub struct TargetMatch<'a> {
    pub arch: &'a str,
    pub profile: &'a str,
}

impl Flags {
    pub fn merge(mut self, rhs: &Self) -> Self {
        self.merge_with(rhs);
        self
    }

    pub fn merge_with(&mut self, rhs: &Self) {
        fn defined_once<T: Clone>(target: &mut Option<T>, src: &Option<T>, name: &str) {
            if let Some(src) = src {
                if target.is_some() {
                    panic!("{} defined twice", name);
                }
                *target = Some(src.clone());
            }
        }
        defined_once(&mut self.target, &rhs.target, "target");
        defined_once(&mut self.linker_script, &rhs.linker_script, "linker-script");
        defined_once(&mut self.deploy, &rhs.deploy, "deploy");
        defined_once(&mut self.initrd, &rhs.initrd, "initrd");
        self.copy.extend(rhs.copy.iter().cloned());
        self.flags.extend(rhs.flags.iter().cloned());
        self.rustc_flags.extend(rhs.rustc_flags.iter().cloned());
    }
}

impl ProjectFlags {
    pub fn get_flags(&self, target: TargetMatch) -> Flags {
        let mut flags = Flags::default();
        self.populate_flags(target, &mut flags);
        flags
    }

    fn populate_flags(&self, target: TargetMatch, flags: &mut Flags) {
        flags.merge_with(&self.flags);
        for (name, config) in &self.configs {
            if name == target.arch || name == target.profile {
                config.populate_flags(target, flags);
            }
        }
    }
}
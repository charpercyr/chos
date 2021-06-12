
use serde::Deserialize;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct RootConfig {
    #[serde(default)]
    pub flags: ProjectOptions,
    #[serde(flatten)]
    pub projects: HashMap<String, RootProjectConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RootProjectConfig {
    pub path: PathBuf,
}

pub fn read_root_config(p: impl AsRef<Path>) -> crate::Result<RootConfig> {
    Ok(toml::from_str(&fs::read_to_string(p)?)?)
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProjectMeta {
    #[serde(default)]
    pub include: Vec<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectFlags {
    pub target: Option<PathBuf>,
    pub linker_script: Option<PathBuf>,
    pub deploy: Option<PathBuf>,
    #[serde(default)]
    pub copy: Vec<(PathBuf, PathBuf)>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub rustc_flags: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProjectOptions {
    #[serde(flatten, default)]
    pub common: ProjectFlags,
    #[serde(default)]
    pub debug: ProjectFlags,
    #[serde(default)]
    pub release: ProjectFlags,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub meta: ProjectMeta,
    #[serde(default)]
    pub project: ProjectOptions,
}

pub fn read_project_config(p: impl AsRef<Path>) -> crate::Result<ProjectConfig> {
    Ok(toml::from_str(&fs::read_to_string(p)?)?)
}
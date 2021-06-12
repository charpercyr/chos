
use std::path::{Path, PathBuf};

use crate::ProjectFlags;

fn canonicalize_pathbuf(path: &mut PathBuf) -> crate::Result<()> {
    *path = path.canonicalize()?;
    Ok(())
}

pub struct ProjectBuilder {
    project: Project,
}

impl ProjectBuilder {
    pub fn new(name: impl Into<String>, cargo_name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            project: Project {
                name: name.into(),
                cargo_name: cargo_name.into(),
                path: path.into(),
                flags: ProjectFlags::default(),
            }
        }
    }
    pub fn merge_flags(&mut self, dir: impl AsRef<Path>, flags: &ProjectFlags) {
        if let Some(target) = &flags.target {
            self.project.flags.target = Some([dir.as_ref(), target].iter().collect());
        }
        if let Some(linker_script) = &flags.linker_script {
            self.project.flags.linker_script = Some([dir.as_ref(), linker_script].iter().collect());
        }
        if let Some(deploy) = &flags.deploy {
            self.project.flags.deploy = Some(deploy.clone());
        }
        for (from, to) in &flags.copy {
            self.project.flags.copy.push((
                [dir.as_ref(), from].iter().collect(),
                to.clone(),
            ));
        }
        self.project.flags.flags.extend(flags.flags.iter().cloned());
        self.project.flags.rustc_flags.extend(flags.rustc_flags.iter().cloned());
    }

    pub fn finish(mut self) -> crate::Result<Project> {
        if let Some(target) = &mut self.project.flags.target {
            canonicalize_pathbuf(target)?;
        }
        if let Some(linker_script) = &mut self.project.flags.linker_script {
            canonicalize_pathbuf(linker_script)?;
        }
        for (from, _) in &mut self.project.flags.copy {
            canonicalize_pathbuf(from)?;
        }
        Ok(self.project)
    }
}

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub cargo_name: String,
    pub path: PathBuf,
    pub flags: ProjectFlags,
}
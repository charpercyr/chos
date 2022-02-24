use std::path::{Path, PathBuf};
use std::str::FromStr;

use duct::cmd;
use tempfile::Builder;

use crate::build::build_main;
use crate::consts::{DEPLOY_BLOCK_SIZE, DEPLOY_BOOT_DEFAULT_SIZE, DEPLOY_DEFAULT_SIZE};
use crate::opts::DeployOpts;
use crate::util::{display_cmd_hook, ErrorGuard};
use crate::Project;

fn check_projects(projects: &[Project]) {
    for proj in projects {
        assert!(
            proj.flags.deploy.is_some(),
            "Deploy path must be set for {}",
            proj.name,
        )
    }
}

struct Loopdev {
    loopdev: PathBuf,
    bootmount: PathBuf,
    fsmount: PathBuf,
}

fn remove_loop(loopdev: impl AsRef<Path>) {
    cmd!("sudo", "losetup", "-d", loopdev.as_ref())
        .before_spawn(display_cmd_hook)
        .run()
        .unwrap();
}

impl Loopdev {
    fn new(file: &Path, bootmount: &Path, fsmount: &Path) -> crate::Result<Self> {
        let loopdev = cmd!("losetup", "-f")
            .before_spawn(display_cmd_hook)
            .read()?;
        let bootpart = format!("{}p1", loopdev);
        let fspart = format!("{}p2", loopdev);

        let err_guard = ErrorGuard::new(|| remove_loop(&loopdev));

        cmd!("sudo", "losetup", "-P", &loopdev, file)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mkdosfs", "-F32", &bootpart)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mkfs.ext2", &fspart)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mkdir", "-p", bootmount)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mkdir", "-p", fsmount)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mount", &bootpart, bootmount)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mount", &fspart, fsmount)
            .before_spawn(display_cmd_hook)
            .run()?;

        err_guard.defuse();

        Ok(Self {
            loopdev: PathBuf::from_str(&loopdev)?,
            bootmount: bootmount.to_path_buf(),
            fsmount: fsmount.to_path_buf(),
        })
    }

    fn loopdev(&self) -> &Path {
        &self.loopdev
    }
}

impl Drop for Loopdev {
    fn drop(&mut self) {
        cmd!("sudo", "umount", &self.bootmount)
            .before_spawn(display_cmd_hook)
            .run()
            .unwrap();
        cmd!("sudo", "umount", &self.fsmount)
            .before_spawn(display_cmd_hook)
            .run()
            .unwrap();
        remove_loop(&self.loopdev);
    }
}

fn copy_file(mount: &Path, from: &Path, to: &Path) -> crate::Result<()> {
    let mut to_path = mount.to_owned().to_string_lossy().into_owned();
    to_path += &*to.to_string_lossy();
    let to_path = Path::new(&to_path);

    let to_dir = to_path.parent().unwrap();

    cmd!("sudo", "mkdir", "-p", to_dir)
        .before_spawn(display_cmd_hook)
        .run()?;
    cmd!("sudo", "cp", from, to_path)
        .before_spawn(display_cmd_hook)
        .run()?;

    Ok(())
}

pub fn deploy(
    file: &Path,
    projects: &[Project],
    release: bool,
    image_size: usize,
) -> crate::Result<()> {
    check_projects(projects);
    cmd!(
        "dd",
        "if=/dev/zero",
        format!("of={}", file.to_string_lossy()),
        format!("bs={}", DEPLOY_BLOCK_SIZE),
        format!("count={}", image_size / DEPLOY_BLOCK_SIZE),
    )
    .before_spawn(display_cmd_hook)
    .run()?;

    cmd!("fdisk", file)
        .stdin_bytes(format!(
            concat!(
                "g\n",          // GPT Partitions
                "n\n\n\n+{}\n", // Boot partition
                "t\n\n1\n",    // Change boot partition to EFI
                "n\n\n\n\n",    // Root partition
                "p\nw\n",       // Write changes
            ),
            DEPLOY_BOOT_DEFAULT_SIZE / DEPLOY_BLOCK_SIZE
        ))
        .before_spawn(display_cmd_hook)
        .run()?;

    let mount = Builder::new().prefix("chos").tempdir()?;

    let loopdev = Loopdev::new(
        file,
        &mount.path().join(Path::new("boot")),
        &mount.path().join("root"),
    )?;

    for proj in projects {
        let target_name = Path::new(proj.flags.target.as_ref().unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy();

        let deploy = proj.flags.deploy.as_ref().unwrap().clone();
        let binary_path: PathBuf = [
            "./target",
            &target_name,
            if release { "release" } else { "debug" },
            &*deploy.0.to_string_lossy(),
        ]
        .iter()
        .collect();

        copy_file(mount.path(), &binary_path, &deploy.1)?;

        for (from, to) in &proj.flags.copy {
            copy_file(mount.path(), from, to)?;
        }
    }

    drop(loopdev);

    cmd!("sync").before_spawn(display_cmd_hook).run()?;

    Ok(())
}

pub fn deploy_main(opts: &DeployOpts, projects: &[Project]) {
    build_main(&opts.build, projects);
    deploy(
        Path::new(&opts.output),
        projects,
        opts.build.release,
        opts.image_size.unwrap_or(DEPLOY_DEFAULT_SIZE),
    )
    .unwrap();
}

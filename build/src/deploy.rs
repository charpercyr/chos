use std::path::{Path, PathBuf};
use std::str::FromStr;

use duct::cmd;
use tempfile::Builder;

use crate::config::{ProjectType, WorkspaceConfig};
use crate::util::display_cmd_hook;
use crate::{build_main, DeployOpts, ErrorMessage, Project};

const DISK_PREFIX: &'static str = "disk:";
const INITRD_PREFIX: &'static str = "initrd:";

fn check_config(config: &[Project]) {
    for proj in config {
        assert!(
            proj.flags.deploy.is_some(),
            "Deploy path must be set for {}",
            proj.name
        );
    }
}

struct Loopdev {
    loopdev: PathBuf,
    mount: PathBuf,
}

fn remove_loop(loopdev: impl AsRef<Path>) {
    cmd!("sudo", "losetup", "-d", loopdev.as_ref())
        .before_spawn(display_cmd_hook)
        .run()
        .unwrap();
}

impl Loopdev {
    fn new(file: impl AsRef<Path>, mount: impl AsRef<Path>) -> crate::Result<Self> {
        let file = file.as_ref();
        let mount = mount.as_ref();

        let loopdev = cmd!("losetup", "-f")
            .before_spawn(display_cmd_hook)
            .read()?;
        let looppart = format!("{}p1", loopdev);

        let err_guard = crate::ErrorGuard::new(|| remove_loop(&loopdev));

        cmd!("sudo", "losetup", "-P", &loopdev, file)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mkfs.ext2", &looppart)
            .before_spawn(display_cmd_hook)
            .run()?;
        cmd!("sudo", "mount", &looppart, mount)
            .before_spawn(display_cmd_hook)
            .run()?;

        err_guard.defuse();

        Ok(Self {
            loopdev: PathBuf::from_str(&loopdev)?,
            mount: mount.to_owned(),
        })
    }

    fn loopdev(&self) -> &Path {
        &self.loopdev
    }
}

impl Drop for Loopdev {
    fn drop(&mut self) {
        cmd!("sudo", "umount", &self.mount)
            .before_spawn(display_cmd_hook)
            .run()
            .unwrap();
        remove_loop(&self.loopdev);
    }
}

fn copy_file(
    mount: impl AsRef<Path>,
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) -> crate::Result<()> {
    let mount = mount.as_ref();
    let from = from.as_ref();
    let mut to = to.as_ref();

    if to.starts_with("/") {
        to = to.strip_prefix("/")?;
    }

    let to_path = mount.join(to);

    let to_dir = to_path.parent().unwrap();

    cmd!("sudo", "mkdir", "-p", to_dir)
        .before_spawn(display_cmd_hook)
        .run()?;
    cmd!("sudo", "cp", from, to_path)
        .before_spawn(display_cmd_hook)
        .run()?;

    Ok(())
}

fn deploy_initrd(
    disk_dir: &Path,
    initrd_dir: &Path,
    initrd_drivers: &[PathBuf],
    initrd_tar: &Path,
) -> crate::Result<()> {
    for driver in initrd_drivers {
        let filename = driver.file_name().expect("Should have a file name");
        copy_file(initrd_dir, driver, filename)?;
    }
    cmd!(
        "sudo",
        "tar",
        "--strip-components=1",
        "-C",
        initrd_dir,
        "-cvf",
        disk_dir.join(initrd_tar),
        ".",
    )
    .before_spawn(display_cmd_hook)
    .run()?;
    Ok(())
}

pub fn deploy(
    file: impl AsRef<Path>,
    config: &[Project],
    release: bool,
    image_size: usize,
    initrd_drivers: Vec<PathBuf>,
) -> crate::Result<()> {
    check_config(config);
    let file = file.as_ref();

    cmd!(
        "dd",
        "if=/dev/zero",
        format!("of={}", file.to_string_lossy()),
        format!("bs={}", crate::DEPLOY_BLOCK_SIZE),
        format!("count={}", image_size / crate::DEPLOY_BLOCK_SIZE),
    )
    .before_spawn(display_cmd_hook)
    .run()?;

    cmd!("fdisk", file)
        .stdin_bytes(&b"n\n\n\n\n\nw\n"[..])
        .before_spawn(display_cmd_hook)
        .run()?;

    let disk_mount = Builder::new().prefix("chos-disk").tempdir()?;
    let initrd_dir = Builder::new().prefix("chos-initrd").tempdir()?;
    let loopdev = Loopdev::new(file, disk_mount.path())?;
    let mount_path = disk_mount.path().to_string_lossy();

    cmd!(
        "sudo",
        "grub-install",
        format!("--root-directory={}", mount_path),
        format!("--boot-directory={}/boot", mount_path),
        loopdev.loopdev(),
    )
    .before_spawn(display_cmd_hook)
    .run()?;

    for proj in config {
        let target_name = proj.target.name();

        let &(ref deploy_from, ref deploy_to) = proj.flags.deploy.as_ref().unwrap();
        let binary_path: PathBuf = [
            "./target",
            &target_name,
            if release { "release" } else { "debug" },
            &*deploy_from.to_string_lossy(),
        ]
        .iter()
        .collect();

        if deploy_to.starts_with(DISK_PREFIX) {
            copy_file(
                disk_mount.path(),
                binary_path,
                &deploy_to[DISK_PREFIX.len()..],
            )?;
        } else if deploy_to.starts_with(INITRD_PREFIX) {
            copy_file(
                initrd_dir.path(),
                binary_path,
                &deploy_to[INITRD_PREFIX.len()..],
            )?;
        } else {
            return Err(ErrorMessage::from(format!("Invalid path '{}'", deploy_to)).into());
        }

        for (from, to) in &proj.flags.copy {
            if to.starts_with(DISK_PREFIX) {
                copy_file(
                    disk_mount.path(),
                    proj.path.join(from),
                    &to[DISK_PREFIX.len()..],
                )?;
            } else if to.starts_with(INITRD_PREFIX) {
                copy_file(
                    initrd_dir.path(),
                    proj.path.join(from),
                    &to[INITRD_PREFIX.len()..],
                )?;
            } else {
                return Err(ErrorMessage::from(format!("Invalid path '{}'", to)).into());
            }
        }
        if proj.typ == ProjectType::Kernel {
            let mut initrd_tar = proj
                .flags
                .initrd
                .as_deref()
                .ok_or(ErrorMessage::from("Initrd must be set for kernel"))?;
            if initrd_tar.starts_with("/") {
                initrd_tar = initrd_tar.strip_prefix("/")?;
            }
            deploy_initrd(
                disk_mount.path(),
                initrd_dir.path(),
                &initrd_drivers,
                initrd_tar,
            )?;
        }
    }

    cmd!("sync").before_spawn(display_cmd_hook).run()?;

    Ok(())
}

pub fn deploy_main(opts: &DeployOpts, workspace: &WorkspaceConfig, config: &[Project]) {
    let initrd_drivers = build_main(&opts.build, workspace, config);
    deploy(
        &opts.output,
        config,
        opts.build.release,
        opts.image_size.unwrap_or(crate::DEPLOY_DEFAULT_SIZE),
        initrd_drivers,
    )
    .unwrap();
}

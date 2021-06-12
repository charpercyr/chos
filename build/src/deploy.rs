use crate::{build_main, DeployOpts, Project};

use std::path::{Path, PathBuf};
use std::process::{Command};
use std::str::{FromStr};

use duct::cmd;

use tempfile::Builder;

fn display_cmd(cmd: &mut Command) -> std::io::Result<()> {
    Ok(crate::display_cmd(cmd))
}

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
        .before_spawn(display_cmd)
        .run()
        .unwrap();
}

impl Loopdev {
    fn new(file: impl AsRef<Path>, mount: impl AsRef<Path>) -> crate::Result<Self> {
        let file = file.as_ref();
        let mount = mount.as_ref();

        let loopdev = cmd!("losetup", "-f").before_spawn(display_cmd).read()?;
        let looppart = format!("{}p1", loopdev);

        let err_guard = crate::ErrorGuard::new(|| remove_loop(&loopdev));

        cmd!("sudo", "losetup", "-P", &loopdev, file).before_spawn(display_cmd).run()?;
        cmd!("sudo", "mkfs.ext2", &looppart).before_spawn(display_cmd).run()?;
        cmd!("sudo", "mount", &looppart, mount).before_spawn(display_cmd).run()?;

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
        cmd!("sudo", "umount", &self.mount).before_spawn(display_cmd).run().unwrap();
        remove_loop(&self.loopdev);
    }
}

fn copy_file(mount: impl AsRef<Path>, from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::Result<()> {
    let mount = mount.as_ref();
    let from = from.as_ref();
    let to = to.as_ref();

    let mut to_path = mount.to_owned().to_string_lossy().into_owned();
    to_path += &*to.to_string_lossy();
    let to_path = Path::new(&to_path);

    let to_dir = to_path.parent().unwrap();

    cmd!("sudo", "mkdir", "-p", to_dir).run()?;
    cmd!("sudo", "cp", from, to_path).run()?;

    Ok(())
}

pub fn deploy(
    file: impl AsRef<Path>,
    config: &[Project],
    release: bool,
    image_size: usize,
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
    .before_spawn(display_cmd)
    .run()?;

    cmd!("fdisk", file)
        .stdin_bytes(&b"n\n\n\n\n\nw\n"[..])
        .before_spawn(display_cmd)
        .run()?;

    let mount = Builder::new().prefix("chos").tempdir()?;
    let loopdev = Loopdev::new(file, mount.path())?;
    let mount_path = mount.path().to_string_lossy();

    cmd!(
        "sudo",
        "grub-install",
        format!("--root-directory={}", mount_path),
        format!("--boot-directory={}/boot", mount_path),
        loopdev.loopdev(),
    )
    .before_spawn(display_cmd)
    .run()?;

    for proj in config {
        let target_name = proj
            .flags
            .target
            .as_ref()
            .unwrap()
            .file_stem()
            .unwrap()
            .to_string_lossy();
        let mut bin_name = proj.cargo_name.clone();
        bin_name += ".elf";
        let binary_path: PathBuf = [
            "./target",
            &target_name,
            if release { "release" } else { "debug" },
            &bin_name,
        ]
        .iter()
        .collect();

        copy_file(
            mount.path(),
            binary_path,
            proj.flags.deploy.as_ref().unwrap(),
        )?;

        for (from, to) in &proj.flags.copy {
            copy_file(mount.path(), from, to)?;
        }
    }

    cmd!("sync").run()?;

    Ok(())
}

pub fn deploy_main(opts: &DeployOpts, config: &[Project]) {
    build_main(&opts.build, config);
    deploy(
        &opts.output,
        config,
        opts.build.release,
        opts.image_size.unwrap_or(crate::DEPLOY_DEFAULT_SIZE),
    )
    .unwrap();
}

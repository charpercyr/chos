use crate::{build_main, DeployOpts, Project};

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::{from_utf8, FromStr};

use tempfile::TempDir;

fn check_config(config: &Vec<Project>) {
    for proj in config {
        assert!(proj.flags.deploy.is_some(), "Deploy path must be set for {}", proj.name);
    }
}

struct Loopdev {
    loopdev: PathBuf,
    mount: PathBuf,
}

fn remove_loop(loopdev: impl AsRef<Path>) {
    let mut cmd = Command::new("sudo");
    cmd.arg("losetup").arg("-d").arg(loopdev.as_ref());
    crate::cmd::status(&mut cmd);
}

impl Loopdev {
    fn new(file: impl AsRef<Path>, mount: impl AsRef<Path>) -> crate::Result<Self> {
        let file = file.as_ref();
        let mount = mount.as_ref();

        let mut cmd = Command::new("losetup");
        cmd.arg("-f");
        crate::cmd::display_cmd(&cmd);
        let output = cmd.output()?;

        crate::cmd::check_status(output.status);

        let loopdev = from_utf8(&output.stdout)?.trim();
        let looppart = format!("{}p1", loopdev);

        let mut cmd = Command::new("sudo");
        cmd.arg("losetup").arg("-P").arg(loopdev).arg(file);
        crate::cmd::status(&mut cmd);

        let mut cmd = Command::new("sudo");
        cmd.arg("mkfs.ext2").arg(&looppart);
        crate::cmd::display_cmd(&cmd);
        let status = cmd.status()?;
        if !status.success() {
            remove_loop(loopdev);
            crate::cmd::check_status(status);
        }

        let mut cmd = Command::new("sudo");
        cmd.arg("mount").arg(looppart).arg(mount);
        crate::cmd::display_cmd(&cmd);
        let status = cmd.status()?;
        if !status.success() {
            remove_loop(loopdev);
            crate::cmd::check_status(status);
        }

        Ok(Self {
            loopdev: PathBuf::from_str(loopdev)?,
            mount: mount.to_owned(),
        })
    }

    fn loopdev(&self) -> &Path {
        &self.loopdev
    }
}

impl Drop for Loopdev {
    fn drop(&mut self) {
        let mut cmd = Command::new("sudo");
        cmd.arg("umount").arg(&self.mount);
        crate::cmd::status(&mut cmd);

        remove_loop(&self.loopdev);
    }
}

fn copy_file(mount: impl AsRef<Path>, from: impl AsRef<Path>, to: impl AsRef<Path>) {
    let mount = mount.as_ref();
    let from = from.as_ref();
    let to = to.as_ref();

    let mut to_path = mount.to_owned().to_string_lossy().into_owned();
    to_path += &*to.to_string_lossy();
    let to_path = Path::new(&to_path);

    let to_dir = to_path.parent().unwrap();

    let mut cmd = Command::new("sudo");
    cmd.arg("mkdir").arg("-p").arg(to_dir);
    crate::cmd::status(&mut cmd);

    let mut cmd = Command::new("sudo");
    cmd.arg("cp").arg(from).arg(to_path);
    crate::cmd::status(&mut cmd);
}

pub fn deploy(
    file: impl AsRef<Path>,
    config: &Vec<Project>,
    release: bool,
    image_size: usize,
) -> crate::Result<()> {
    check_config(config);
    let file = file.as_ref();
    let mut cmd = Command::new("dd");
    let image_size = image_size / crate::DEPLOY_BLOCK_SIZE;
    cmd.arg("if=/dev/zero");
    cmd.arg(format!("of={}", file.to_string_lossy()));
    cmd.arg(format!("bs={}", crate::DEPLOY_BLOCK_SIZE));
    cmd.arg(format!("count={}", image_size));
    crate::cmd::status(&mut cmd);

    let mut cmd = Command::new("fdisk");
    cmd.arg(file);
    cmd.stdin(Stdio::piped());
    crate::cmd::display_cmd(&cmd);
    let mut fdisk = cmd.spawn()?;
    BufWriter::new(fdisk.stdin.as_mut().unwrap()).write_all(b"n\n\n\n\n\nw\n")?;
    crate::cmd::check_status(fdisk.wait()?);

    let mount = TempDir::new()?;
    let loopdev = Loopdev::new(file, mount.path())?;
    let mount_path = mount.path().to_string_lossy();

    let mut cmd = Command::new("sudo");
    cmd.arg("grub-install");
    cmd.arg(format!("--root-directory={}", mount_path));
    cmd.arg(format!("--boot-directory={}/boot", mount_path));
    cmd.arg(loopdev.loopdev());
    crate::cmd::status(&mut cmd);

    for proj in config {
        let target_name = proj
            .flags
            .target
            .as_ref()
            .unwrap()
            .file_stem()
            .unwrap()
            .to_string_lossy()
        ;
        let mut bin_name = proj.cargo_name.to_string();
        bin_name += ".elf";
        let binary_path: PathBuf = [
            "./target",
            &target_name,
            if release { "release" } else { "debug" },
            &bin_name].iter().collect();
            
        copy_file(mount.path(), binary_path, proj.flags.deploy.as_ref().unwrap());

        for (from ,to) in &proj.flags.copy {
            copy_file(mount.path(), from, to);
        }
    }

    Ok(())
}

pub fn deploy_main(opts: &DeployOpts, config: &Vec<Project>) {
    build_main(&opts.build, &config);
    deploy(
        &opts.output,
        config,
        opts.build.release,
        opts.image_size.unwrap_or(crate::DEPLOY_DEFAULT_SIZE),
    )
    .unwrap();
}

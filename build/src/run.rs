use duct::cmd;
use tempfile::Builder;

use crate::build::build_main;
use crate::consts::DEPLOY_DEFAULT_SIZE;
use crate::deploy::deploy;
use crate::opts::RunOpts;
use crate::util::display_cmd_hook;
use crate::Project;
const KERNEL_EXIT_SUCCESS: i32 = 33;

pub fn run_main(opts: &RunOpts, projects: &[Project]) {
    if opts.build.arch != "x86_64" {
        panic!("Run not supported for {}", opts.build.arch);
    }
    if !opts.no_build {
        build_main(&opts.build, projects);
    }
    let imgdir = Builder::new().prefix("chos.run").tempdir().unwrap();
    let imgfile = imgdir.path().join("chos.img");
    let varsfile = imgdir.path().join("OVMF_VARS.fd");

    cmd!(
        "cp",
        "/usr/share/ovmf/x64/OVMF_VARS.fd",
        &varsfile,
    )
    .before_spawn(display_cmd_hook)
    .run()
    .unwrap();

    deploy(&imgfile, projects, opts.build.release, DEPLOY_DEFAULT_SIZE).unwrap();

    let smp = format!("cpus={}", opts.smp);
    let imgfile_str = imgfile.to_string_lossy();
    // let imgdrive_args = format!("if=ide,file={}", );
    let qemu = format!("qemu-system-{}", opts.build.arch);
    let vars_drive = format!(
        "if=pflash,format=raw,unit=1,file={}",
        varsfile.to_string_lossy()
    );
    let mut args: Vec<&str> = vec![
        &qemu,
        "-global", "driver=cfi.pflash01,property=secure,value=on",
        "-drive", "if=pflash,format=raw,unit=0,file=/usr/share/ovmf/x64/OVMF_CODE.fd,readonly=on",
        "-drive", &vars_drive,
        "-hda", &imgfile_str,
        "-m", &opts.mem,
        "-smp", &smp,
        "-machine", "q35,smm=on",
        // "-cpu", "Skylake-Client-v3",
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4",
        "-display", "none",
        "-net", "none",
        "-serial", "stdio",
    ];

    if opts.debug {
        args.push("-s");
        args.push("-S");
    }

    let qemu = cmd("sudo", args)
        .before_spawn(display_cmd_hook)
        .unchecked()
        // .stderr_null()
        .run()
        .unwrap();
    if qemu.status.code().unwrap() != KERNEL_EXIT_SUCCESS {
        panic!("Qemu failed with exit code {}", qemu.status.code().unwrap());
    }
}


use crate::{Project, RunOpts, build::build_main};

use duct::cmd;

use tempfile::Builder;

const KERNEL_EXIT_SUCCESS: i32 = 33;

pub fn run_main(opts: &RunOpts, config: &[Project]) {
    if opts.build.arch != "x86_64" {
        panic!("Run not supported for {}", opts.build.arch);
    }
    build_main(&opts.build, config);
    let imgfile = Builder::new().prefix("chos").suffix(".img").tempfile().unwrap();
    crate::deploy(imgfile.path(), config, opts.build.release, crate::DEPLOY_DEFAULT_SIZE).unwrap();

    let smp = format!("{}", opts.smp);
    let imgfile_str = imgfile.path().to_string_lossy();
    let mut args = vec![
        "-m", &opts.mem,
        "-smp", &smp,
        "-machine", "q35",
        "-cpu", "Skylake-Client-v3",
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4",
        "-display", "none",
        "-serial", "stdio",
        &*imgfile_str,
    ];

    if opts.debug {
        args.push("-s");
        args.push("-S");
    }

    let qemu = cmd(format!("qemu-system-{}", opts.build.arch), args)
        .before_spawn(crate::display_cmd_hook)
        .unchecked()
        .stderr_null()
        .run()
        .unwrap();
    if qemu.status.code().unwrap() != KERNEL_EXIT_SUCCESS {
        panic!("Qemu failed with exit code {}", qemu.status.code().unwrap());
    }
}

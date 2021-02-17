
from util import run

from subprocess import CompletedProcess, DEVNULL

KERNEL_SUCCESS = 33
KERNEL_FAILURE = 35

def qemu(binary: str, serial: bool = True, display: bool = False) -> bool:
    args = [
        'qemu-system-x86_64', binary,
        '-m', '4G',
        '-machine', 'q35',
        '-cpu', 'Skylake-Client-v1',
        '-device', 'isa-debug-exit,iobase=0xf4,iosize=0x4',
        '-display',
    ]
    if display:
        args.append('curses')
    else:
        args.append('none')

    if serial:
        args.extend(['-serial', 'stdio'])

    ret = run(*args, checkReturnCode=False, stderr=DEVNULL)
    if ret.returncode == KERNEL_SUCCESS:
        return True
    else:
        return False
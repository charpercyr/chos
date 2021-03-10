
import tempfile
import sys

from deploy import deploy
from qemu import qemu
from util import run

KERNEL = 'target/x86_64-chos-kernel/debug/chos-kernel.elf'

def main():
    with tempfile.TemporaryDirectory('chos') as wd:
        binary = sys.argv[1]
        img = deploy(binary, KERNEL, wd)
        if not qemu(img, serial=True, display=False):
            exit(1)

if __name__ == '__main__':
    main()

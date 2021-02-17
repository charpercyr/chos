#!/usr/bin/python3

import argparse
import io
import subprocess as sp
import os

from util import run, write_file

BS = 512
SIZE = 128 * 1024 * 1024
COUNT = SIZE//BS

LOOPDEV = '/dev/loop0'
FSDEV = '/dev/loop0p1'

CHOS_NAME='chos-boot.elf'

GRUB_CFG=f"""\
set timeout=0
set default=0

menuentry "chos" {{
    multiboot2 /boot/{CHOS_NAME} arg1 arg2
    boot
}}
"""

IMG = 'chos.img'
FS = 'root'

def deploy(binary: str, wd: str):
    imgpath = f'{wd}/{IMG}'
    fspath = f'{wd}/{FS}'

    run('dd', 'if=/dev/zero', f'of={imgpath}', f'bs={BS}', f'count={COUNT}')
    run('fdisk', imgpath, input=b'n\n\n\n\n\nw\n')
    run('sudo', 'losetup', '-P', LOOPDEV, imgpath)
    run('sudo', 'mkfs.ext2', FSDEV)
    run('mkdir', '-p', fspath)
    run('sudo', 'mount', FSDEV, fspath)
    run('sudo', 'grub-install', f'--root-directory={fspath}', f'--boot-directory={fspath}/boot', LOOPDEV)

    run('sudo', 'cp', binary, f'{fspath}/boot/chos-boot.elf')

    write_file(f'{wd}/grub.cfg', GRUB_CFG)
    run('sudo', 'cp', f'{wd}/grub.cfg', f'{fspath}/boot/grub/grub.cfg')

    run('sudo', 'umount', fspath)
    run('sudo', 'losetup', '-d', LOOPDEV)

    run('sync')

    return imgpath

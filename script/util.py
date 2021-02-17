
import subprocess as sp

def run(*args: [str], checkReturnCode: bool = True, expectedReturns: [int] = [0], **kwargs) -> sp.CompletedProcess:
    print(' '.join(args))
    ret = sp.run(args, **kwargs)
    if checkReturnCode and ret.returncode not in expectedReturns:
        raise Exception(f'Could not run the command, exit={ret.returncode}')
    return ret

def write_file(path: str, content: bytes):
    print(f'write {path} ({len(content)} bytes)')
    file = open(path, 'w')
    file.write(content)
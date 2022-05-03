import pytest
from xprocess import ProcessStarter

import socket
from contextlib import closing

def find_free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(('', 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]


@pytest.fixture
def socket_example(xprocess, request):
    port = find_free_port()

    class Starter(ProcessStarter):
        # startup pattern
        pattern = "Running server"

        # command to start process
        args = ['cargo', 'run', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'server', '--', '--port', str(port)]

    # ensure process is running and return its logfile
    logfile = xprocess.ensure("socket_example", Starter)

    yield port

    # clean up whole process tree afterwards
    xprocess.getinfo("socket_example").terminate()

import os
import pytest
from pyvisa import ResourceManager
from xprocess import ProcessStarter

import socket
from contextlib import closing


def find_free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(('', 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]


@pytest.fixture(scope='session', autouse=True)
def resource_manager(request):
    return ResourceManager()


@pytest.fixture
def hislip_example(xprocess, request):
    debug = os.environ.get('HISLIP_DEBUG')
    if debug is not None:
        addr, port = debug.split(':')

        yield f'TCPIP::{addr}::hislip0,{port}::INSTR'
    else:
        addr = "127.0.0.1"
        port = find_free_port()

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Cargo might print a lot before starting the program
            max_read_lines = 500

            # command to start process
            args = ['cargo', 'run', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'server', '--', '--port', str(port)]

        # ensure process is running and return its logfile
        logfile = xprocess.ensure("hislip_example", Starter)

        yield f'TCPIP::{addr}::hislip0,{port}::INSTR'

        # clean up whole process tree afterwards
        xprocess.getinfo("hislip_example").terminate()

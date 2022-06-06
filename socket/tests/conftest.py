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
def socket_example(xprocess, request):
    target = os.environ.get('DEBUG_TARGET')
    if target is not None:
        port = os.environ.get('SOCKET_PORT', default="5025")

        yield f'TCPIP::{target}::{port}::SOCKET'
    else:
        port = find_free_port()

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Hide warnings
            env = {'RUSTFLAGS': '-Awarnings', **os.environ}

            # command to start process
            args = ['cargo', 'run', '-q', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'server', '--', '--port', str(port)]

        # ensure process is running and return its logfile
        name = request.function.__name__
        logfile = xprocess.ensure(f"socket_example-{name}", Starter)

        yield f'TCPIP::localhost::{port}::SOCKET'

        # clean up whole process tree afterwards
        xprocess.getinfo(f"socket_example-{name}").terminate()

@pytest.fixture
def telnet_example(xprocess, request):
    target = os.environ.get('DEBUG_TARGET')
    if target is not None:
        port = os.environ.get('TELNET_PORT', default="5024")

        yield f'TCPIP::{target}::{port}::SOCKET'
    else:
        port = find_free_port()

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Hide warnings
            env = {'RUSTFLAGS': '-Awarnings', **os.environ}

            # command to start process
            args = ['cargo', 'run', '-q', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'telnet', '--', '--port', str(port)]

        # ensure process is running and return its logfile
        name = request.function.__name__
        logfile = xprocess.ensure(f"telnet_example-{name}", Starter)

        yield ('localhost', port)

        # clean up whole process tree afterwards
        xprocess.getinfo(f"telnet_example-{name}").terminate()

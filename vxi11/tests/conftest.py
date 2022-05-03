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
def vxi11_example(xprocess, request):
    port = find_free_port()

    class Starter(ProcessStarter):
        # startup pattern
        pattern = "Running server"
        max_read_lines = 500

        # command to start process
        args = ['cargo', 'run', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'server']


    # ensure process is running and return its logfile
    logfile = xprocess.ensure("vxi11_example", Starter)

    yield port

    # clean up whole process tree afterwards
    xprocess.getinfo("vxi11_example").terminate()

@pytest.fixture
def portmap_example(xprocess, request):

    class Starter(ProcessStarter):
        # startup pattern
        pattern = "Running server"
        max_read_lines = 500

        # command to start process
        args = ['cargo', 'run', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'portmap']


    # ensure process is running and return its logfile
    logfile = xprocess.ensure("portmap_example", Starter)

    yield

    # clean up whole process tree afterwards
    xprocess.getinfo("portmap_example").terminate()
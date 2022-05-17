import os
from time import sleep
import pytest
from pyvisa import ResourceManager
from xprocess import ProcessStarter

import socket
from contextlib import closing

@pytest.fixture(scope='session', autouse=True)
def resource_manager(request):
    return ResourceManager()


@pytest.fixture
def vxi11_example(xprocess, request):
    debug = os.environ.get('VXI11_TARGET')
    if debug is not None:
        yield debug
    else:
        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"
            max_read_lines = 500

            # command to start process
            args = ['cargo', 'run', '--manifest-path', request.fspath.dirname+'/../Cargo.toml', '--example', 'server', '--', '--register', 'localhost:4321', 'localhost:4322']


        # ensure process is running and return its logfile
        logfile = xprocess.ensure("vxi11_example", Starter)

        yield "TCPIP::localhost::inst0::INSTR"

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
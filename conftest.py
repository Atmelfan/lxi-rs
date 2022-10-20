import os
import subprocess
import pytest
from pyvisa import ResourceManager
import socket
from contextlib import closing

pytest.fixture(scope='session', autouse=True)
def prep_cargo(db, data):
    print("Building...")
    return_code = subprocess.call("cargo build --examples", shell=True)
    # yield, to let all tests within the scope run
    yield 


@pytest.fixture
def free_port(request):
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(("", 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]

@pytest.fixture(scope="session", autouse=True)
def resource_manager(request):
    return ResourceManager()

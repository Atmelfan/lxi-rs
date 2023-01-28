import os
import subprocess
import pytest
from pyvisa import ResourceManager
import socket
from contextlib import closing

@pytest.fixture
def free_port(request):
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(("", 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]

@pytest.fixture(scope="session", autouse=True)
def resource_manager(request):
    return ResourceManager()

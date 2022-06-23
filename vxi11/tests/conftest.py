import os
import pytest
from pyvisa import ResourceManager
from xprocess import ProcessStarter


@pytest.fixture(scope="session", autouse=True)
def resource_manager(request):
    return ResourceManager()


@pytest.fixture
def vxi11_example(xprocess, request):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        yield f"TCPIP::{target}::inst0::INSTR"
    else:

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Hide warnings
            env = {"RUSTFLAGS": "-Awarnings", **os.environ}

            # command to start process
            args = [
                "cargo",
                "run",
                "-q",
                "--example",
                "vxi11",
                "--",
                "--register",
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"vxi11_example-{name}", Starter)

        yield "TCPIP::127.0.0.1::inst0::INSTR"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"vxi11_example-{name}").terminate()

import os
import pytest
from xprocess import ProcessStarter

@pytest.fixture
def vxi11_example(xprocess, request, pytestconfig):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        yield f"TCPIP::{target}"
    else:

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Hide warnings
            env = {"RUSTFLAGS": "-Awarnings", "CARGO_TARGET_DIR": pytestconfig.cache.mkdir("target"), **os.environ}

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

        yield "TCPIP::localhost"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"vxi11_example-{name}").terminate()

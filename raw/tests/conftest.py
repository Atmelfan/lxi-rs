import os
import pytest
from xprocess import ProcessStarter

@pytest.fixture
def socket_example(xprocess, request, free_port):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        port = os.environ.get("SOCKET_PORT", default="5025")

        yield f"TCPIP::{target}::{port}::SOCKET"
    else:
        port = os.environ.get("SOCKET_PORT", default=str(free_port))

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
                "raw",
                "--",
                "--port",
                str(port),
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"socket_example-{name}", Starter)

        yield f"TCPIP::localhost::{port}::SOCKET"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"socket_example-{name}").terminate()

import os
import pytest
from xprocess import ProcessStarter

@pytest.fixture
def telnet_example(xprocess, request, free_port):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        port = os.environ.get("TELNET_PORT", default="5024")

        yield (target, port)
    else:
        port = port = os.environ.get("TELNET_PORT", default=str(free_port))

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
                "telnet",
                "--",
                "--port",
                str(port),
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"telnet_example-{name}", Starter)

        yield ("localhost", port)

        # clean up whole process tree afterwards
        xprocess.getinfo(f"telnet_example-{name}").terminate()

import os
import pytest
from xprocess import ProcessStarter

@pytest.fixture
def hislip_example(xprocess, request, free_port):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        port = os.environ.get("HISLIP_PORT")

        if port is not None:
            yield f"TCPIP::{target}::hislip0,{port}::INSTR"
        else:
            yield f"TCPIP::{target}::hislip0::INSTR"

    else:
        port = free_port

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
                "hislip",
                "--",
                "--port",
                str(port),
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"hislip_example-{name}", Starter)

        yield f"TCPIP::localhost::hislip0,{port}::INSTR"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"hislip_example-{name}").terminate()

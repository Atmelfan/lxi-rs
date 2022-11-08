import os
import pytest
from xprocess import ProcessStarter


@pytest.fixture
def hislip_example(xprocess, request, pytestconfig, free_port):
    target = os.environ.get("DEBUG_TARGET")
    
    # Add credentials if set
    credentials = os.environ.get("HISLIP_CRED")
    if credentials is not None:
        prefix = f"{credentials}@"
    else:
        prefix = ""
    
    if target is not None:
        port = os.environ.get("HISLIP_PORT")

        if port is not None:
            yield f"TCPIP::{prefix}{target}::hislip0,{port}::INSTR"
        else:
            yield f"TCPIP::{prefix}{target}::hislip0::INSTR"

    else:
        port = os.environ.get("HISLIP_PORT", default=str(free_port))

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Running server"

            # Hide warnings
            env = {
                "RUSTFLAGS": "-Awarnings",
                # "CARGO_TARGET_DIR": pytestconfig.cache.mkdir("target"),
                **os.environ,
            }

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

        yield f"TCPIP::{prefix}localhost::hislip0,{port}::INSTR"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"hislip_example-{name}").terminate()

import os
import pytest
from xprocess import ProcessStarter

@pytest.fixture
def http_example(xprocess, request, pytestconfig, free_port):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        port = os.environ.get("HTTP_PORT")

        if port is not None:
            yield f"http://{target}:{port}"
        else:
            yield f"http://{target}"

    else:
        port = os.environ.get("HTTP_PORT", default=str(free_port))

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Server listening"

            # Hide warnings
            env = {"RUSTFLAGS": "-Awarnings", "CARGO_TARGET_DIR": pytestconfig.cache.mkdir("target"), **os.environ}

            # command to start process
            args = [
                "cargo",
                "run",
                "-q",
                "--example",
                "http",
                "--",
                "--port",
                str(port),
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"http_example-{name}", Starter)

        yield f"http://localhost:{port}"

        # clean up whole process tree afterwards
        xprocess.getinfo(f"http_example-{name}").terminate()

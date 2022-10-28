import os
import ssl
import pytest
from xprocess import ProcessStarter

CERTIFICATES_DIR = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), "..", "..", "./certificates"
)


@pytest.fixture
def ssl_cert_file() -> str:
    return os.path.join(CERTIFICATES_DIR, "cert.pem")

@pytest.fixture
def ssl_key_file() -> str:
    return os.path.join(CERTIFICATES_DIR, "key.pem")

@pytest.fixture
def ssl_context(ssl_cert_file) -> str:
    myssl = ssl.create_default_context()
    myssl.check_hostname=False
    myssl.verify_mode=ssl.CERT_NONE
    myssl.load_verify_locations(ssl_cert_file)
    return myssl

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
            env = {
                "RUSTFLAGS": "-Awarnings",
                #"CARGO_TARGET_DIR": pytestconfig.cache.mkdir("target"),
                **os.environ,
            }

            # command to start process
            args = [
                "cargo",
                "run",
                "-q",
                "--example",
                "identification",
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


@pytest.fixture
def auth_example(xprocess, request, pytestconfig, free_port, ssl_cert_file, ssl_key_file):
    target = os.environ.get("DEBUG_TARGET")
    if target is not None:
        port = os.environ.get("HTTP_PORT")
        ports = os.environ.get("HTTPS_PORT")

        if port is not None:
            yield (f"http://{target}:{port}", f"https://{target}:{ports}")
        else:
            yield (f"http://{target}", f"https://{target}")

    else:
        port = os.environ.get("HTTP_PORT", default=str(free_port))
        ports = os.environ.get("HTTPS_PORT", default=str(free_port + 1))

        class Starter(ProcessStarter):
            # startup pattern
            pattern = "Server listening"

            # Hide warnings
            env = {
                "RUSTFLAGS": "-Awarnings",
                #"CARGO_TARGET_DIR": pytestconfig.cache.mkdir("target"),
                **os.environ,
            }

            # command to start process
            args = [
                "cargo",
                "run",
                "-q",
                "--example",
                "auth",
                "--",
                "--http-port",
                str(port),
                "--https-port",
                str(ports),
                "-c",
                ssl_cert_file,
                "-k",
                ssl_key_file,
            ]

        # ensure process is running and return its logfile
        name = request.function.__name__
        xprocess.ensure(f"auth_example-{name}", Starter)

        yield (f"http://localhost:{port}", f"https://localhost:{ports}")

        # clean up whole process tree afterwards
        xprocess.getinfo(f"auth_example-{name}").terminate()

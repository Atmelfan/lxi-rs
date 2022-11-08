# Only works with Keysight IO Libraries and Secure communications expert
# See README on how to setup
#

from pyvisa import highlevel


for backend in highlevel.list_backends():
    if backend.startswith("pyvisa-"):
        backend = backend[7:]

    try:
        cls = highlevel.get_wrapper_class(backend)
    except Exception as e:
        backend_details[backend] = [
            "Could not instantiate backend",
            "-> %s" % str(e),
        ]
        continue

    try:
        backend_details[backend] = cls.get_debug_info()
    except Exception as e:
        backend_details[backend] = [
            "Could not obtain debug info",
            "-> %s" % str(e),
        ]
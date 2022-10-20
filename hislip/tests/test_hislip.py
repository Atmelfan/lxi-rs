from time import time
import pytest
import pyvisa

IDN_RESPONSE = "GPA-Robotics,hislip-demo,0,0"

def test_connect(hislip_example, resource_manager):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    inst.close()


def test_hislip_idn(hislip_example, resource_manager):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    resp = inst.query("*IDN?\n")
    assert resp == IDN_RESPONSE

    inst.close()

# TODO: VISA refuses to even send the query when locked making us unable to test if the short-circuit works
# def test_hislip_idn_short(hislip_example, resource_manager):
#     if resource_manager.visalib.library_path == "py":
#         pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
#     inst1 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")
#     inst2 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

#     inst1.lock(requested_key="foo", timeout=0)
#     assert inst2.query("*IDN?") == IDN_RESPONSE

#     inst1.close()
#     inst2.close()


def test_clear(hislip_example, resource_manager: pyvisa.ResourceManager):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst: pyvisa.resources.MessageBasedResource = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    inst.send_end = False
    inst.write("GARBAGE")
    inst.send_end = True
    inst.clear()
    assert inst.query("*IDN?") == IDN_RESPONSE

    inst.close()


def test_trigger(hislip_example, resource_manager: pyvisa.ResourceManager):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    inst.assert_trigger()

    inst.close()


def test_hislip_exclusive_lock(
    hislip_example, resource_manager: pyvisa.ResourceManager
):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    # Lock and unlock
    inst.lock_excl(25.0)
    inst.unlock()

    inst.close()


def test_hislip_shared_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst1 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")
    inst2 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")
    inst3 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    # Lock
    inst1.lock(requested_key="foo", timeout=0)
    inst2.lock(requested_key="foo", timeout=1000000000)

    # Timeout
    t1 = time()
    with pytest.raises(pyvisa.VisaIOError):
        inst3.lock(1000, requested_key="bar")
    dt = time() - t1
    print(f"Timeout took {dt}s")
    assert dt > 1.0, "Timeout occured too fast"

    inst1.unlock()
    inst2.unlock()

    # inst3 may lock
    inst3.lock(1000, requested_key="bar")

    inst1.close()
    inst2.close()
    inst3.close()


def test_hislip_clear_in_progress(
    hislip_example, resource_manager: pyvisa.ResourceManager
):
    if resource_manager.visalib.library_path == "py":
        pytest.skip("pyvisa-py does not support HiSLIP", allow_module_level=True)
    inst1 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")
    inst2 = resource_manager.open_resource(hislip_example, read_termination = "", write_termination = "")

    # Lock
    inst1.lock(requested_key="foo")

    # Timeout
    inst1.close()
    inst2.close()

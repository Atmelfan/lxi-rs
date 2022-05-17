from time import time
import pytest
import pyvisa
import pyvisa.util

#if "ivi" not in pyvisa.util.get_system_details():
#    pytest.skip("Cannot test HiSLIP without NI-VISA installed", allow_module_level=True)

def test_connect(hislip_example, resource_manager):
    inst = resource_manager.open_resource(hislip_example)

    inst.close()

def test_hislip_idn(hislip_example, resource_manager):
    inst = resource_manager.open_resource(hislip_example)
    inst.read_termination = '\n'
    inst.write_termination = '\n'

    resp = inst.query("*IDN?")
    assert resp == "Cyberdyne systems,T800 Model 101,A9012.C,V2.4"

    inst.close()

def test_clear(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(hislip_example)

    inst.clear()

    inst.close()

def test_trigger(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(hislip_example)

    inst.assert_trigger()

    inst.close()

def test_hislip_exclusive_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(hislip_example)

    # Lock and unlock
    inst.lock_excl(25.0)
    inst.unlock()

    inst.close()

def test_hislip_shared_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst1 = resource_manager.open_resource(hislip_example)
    inst2 = resource_manager.open_resource(hislip_example)
    inst3 = resource_manager.open_resource(hislip_example)

    # Lock
    inst1.lock(requested_key="foo")
    inst2.lock(requested_key="foo")
    t1 = time()
    with pytest.raises(pyvisa.VisaIOError) as excinfo:
        inst3.lock(1000, requested_key="bar")
    dt = time() - t1

    inst1.unlock()
    inst2.unlock()

    inst1.close()
    inst2.close()
    inst3.close()





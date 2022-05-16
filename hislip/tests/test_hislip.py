import pytest
import pyvisa
import pyvisa.util

#if "ivi" not in pyvisa.util.get_system_details():
#    pytest.skip("Cannot test HiSLIP without NI-VISA installed", allow_module_level=True)

def test_hislip_idn(hislip_example, resource_manager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')
    inst.read_termination = '\n'
    inst.write_termination = '\n'

    resp = inst.query("*IDN?")
    assert resp == "Cyberdyne systems,T800 Model 101,A9012.C,V2.4"

    inst.close()

def test_hislip_exclusive_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')

    # Lock and unlock
    inst.lock_excl(25.0)
    inst.unlock()

    inst.close()

def test_hislip_shared_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst1 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')
    inst2 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')
    inst3 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')

    # Lock
    inst1.lock(25.0, requested_key="foo")
    inst2.lock(25.0, requested_key="foo")
    with pytest.raises(pyvisa.VisaIOError) as excinfo:
        inst3.lock(25.0, requested_key="bar")

    inst1.unlock()
    inst2.unlock()

    inst1.close()
    inst2.close()
    inst3.close()





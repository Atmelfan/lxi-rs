from time import time
import pytest
import pyvisa
from pyvisa.constants import AccessModes

@pytest.mark.order(0)
def test_create_link(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')
    inst.close()

@pytest.mark.order(1)
def test_query(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')
    inst.read_termination = '\n'
    inst.write_termination = '\n'

    resp = inst.query("*IDN?").strip()
    assert resp == "Cyberdyne systems,T800 Model 101,A9012.C,V2.4"

    inst.close()

@pytest.mark.order(2)
def test_read_stb(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    status = inst.read_stb()
    assert status == 0

    inst.close()

@pytest.mark.order(3)
def test_trigger(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    status = inst.read_stb()
    assert status == 0

    inst.assert_trigger()
    status = inst.read_stb()
    assert status == 64

    inst.clear()
    status = inst.read_stb()
    assert status == 0

    inst.close()

@pytest.mark.order(4)
def test_lock(resource_manager: pyvisa.ResourceManager):
    inst1 = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')
    inst2 = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    # Two clients cannot lock at the same time
    inst1.lock_excl()
    with pytest.raises(pyvisa.VisaIOError) as excinfo:
        inst2.lock_excl(timeout=100)
    
    # Unlock client1 and check that client2 can lock
    inst1.unlock()
    inst2.lock()

    inst1.close()
    inst2.close()



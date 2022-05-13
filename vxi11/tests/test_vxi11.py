import pytest
import pyvisa
from pyvisa.constants import AccessModes

@pytest.mark.order(0)
def test_create_link(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR', access_mode=AccessModes.exclusive_lock)
    inst.close()

@pytest.mark.order(1)
def test_query(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    resp = inst.query("*IDN?").strip()
    assert resp == "*IDN?"

    inst.close()

@pytest.mark.order(2)
def test_read_stb(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    status = inst.read_stb()
    assert status == 0

    inst.close()

@pytest.mark.order(3)
def test_lock(resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    inst.lock_excl()

    inst.unlock()

    inst.close()

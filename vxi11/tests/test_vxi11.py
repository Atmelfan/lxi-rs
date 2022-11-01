import pytest
import pyvisa


def test_inst0(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")
    inst.close()

def test_inst1(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f"{vxi11_example}::inst1::INSTR")
    inst.close()


def test_query(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")
    inst.read_termination = ""
    inst.write_termination = ""

    resp = inst.query("*IDN?").strip()
    assert resp == "Cyberdyne systems,T800 Model 101,A9012.C,V2.4"

    inst.close()


def test_read_stb(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")

    status = inst.read_stb()
    assert status == 0

    inst.close()


def test_trigger(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")

    status = inst.read_stb()
    assert status & 0x40 == 0

    inst.assert_trigger()
    status = inst.read_stb()
    assert status & 0x40 != 0, "Failed to trigger?"

    inst.clear()
    status = inst.read_stb()
    assert status & 0x40 == 0

    inst.close()


def test_lock(vxi11_example, resource_manager: pyvisa.ResourceManager):
    inst1 = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")
    inst2 = resource_manager.open_resource(f"{vxi11_example}::inst0::INSTR")

    # Two clients cannot lock at the same time
    inst1.lock_excl()
    with pytest.raises(pyvisa.VisaIOError):
        inst2.lock_excl(timeout=100)

    # Unlock client1 and check that client2 can lock
    inst1.unlock()
    inst2.lock()

    inst1.close()
    inst2.close()

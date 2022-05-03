import pyvisa

def test_hislip_idn(hislip_example, resource_manager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')

    resp = inst.query("*IDN?")
    assert resp == "*IDN?"

def test_hislip_exclusive_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')

    # Lock and unlock
    inst.lock_excl(25.0)
    inst.unlock()


def test_hislip_shared_lock(hislip_example, resource_manager: pyvisa.ResourceManager):
    inst1 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')
    inst2 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')
    inst3 = resource_manager.open_resource(f'TCPIP::127.0.0.1::hislip0,{hislip_example}::INSTR')

    # Lock
    inst1.lock(25.0, requested_key="foo")
    inst2.lock(25.0, requested_key="foo")
    try:
        inst3.lock(25.0, requested_key="bar")
        assert False, "Shouldnt be able to lock"
    except pyvisa.Error:
        pass

    inst1.unlock()
    inst2.unlock()





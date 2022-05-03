import pyvisa

def test_tcpip_vxi11(vxi11_example):
    rm = pyvisa.ResourceManager()

    inst = rm.open_resource(f'TCPIP::127.0.0.1::{vxi11_example}::INSTR')

    resp = inst.query("*IDN?")
    assert resp == "*IDN?"
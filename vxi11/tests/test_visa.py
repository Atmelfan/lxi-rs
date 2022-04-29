import pyvisa

def test_tcpip_socket(myserver):
    rm = pyvisa.ResourceManager()

    inst = rm.open_resource(f'TCPIP::127.0.0.1::{myserver}::INSTR')

    resp = inst.query("*IDN?")
    assert resp == "*IDN?"
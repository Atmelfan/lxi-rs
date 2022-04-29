import pyvisa

def test_tcpip_socket(myserver):
    rm = pyvisa.ResourceManager()

    inst = rm.open_resource(f'TCPIP::127.0.0.1::{myserver}::SOCKET')
    inst.read_termination = '\n'
    inst.write_termination = '\n'

    resp = inst.query("*IDN?")
    assert resp == "*IDN?"
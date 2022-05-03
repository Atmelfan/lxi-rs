import pyvisa

def test_tcpip_vxi11(resource_manager: pyvisa.ResourceManager):
    resource_manager
    inst = resource_manager.open_resource(f'TCPIP::127.0.0.1::inst0::INSTR')

    #resp = inst.query("*IDN?")
    #assert resp == "*IDN?"

    inst.close()
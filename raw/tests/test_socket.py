import pyvisa


def test_tcpip_socket(socket_example, resource_manager: pyvisa.ResourceManager):
    inst = resource_manager.open_resource(socket_example)
    inst.read_termination = "\n"
    inst.write_termination = "\n"

    resp = inst.query("*IDN?")
    assert resp == "Cyberdyne systems,T800 Model 101,A9012.C,V2.4"

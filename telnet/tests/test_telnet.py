from telnetlib import Telnet


def test_telnet(telnet_example):
    with Telnet(*telnet_example, timeout=1000) as tn:
        tn.read_until(b"SCPI> ")
        tn.write(b"*IDN?\r\n")
        resp = tn.read_until(b"\r\n", timeout=1000)
        assert resp == b"Cyberdyne systems,T800 Model 101,A9012.C,V2.4\r\n"

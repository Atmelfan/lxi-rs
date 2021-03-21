import visa

rm = visa.ResourceManager()
my_instrument = rm.open_resource('TCPIP::localhost::INSTR')


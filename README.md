# lxi-rs

This crate aims to simplify  the [LXI Device Specification](https://www.lxistandard.org/Specifications/Specifications.aspx). 
The specifications consists of a [core specification](https://www.lxistandard.org/members/Adopted%20Specifications/Latest%20Version%20of%20Standards_/LXI%20Standard%201.5%20Specifications/LXI%20Device%20Specification%20v1_5_01.pdf) and a optional set of extended functions. 


## Relevant standards:
* [IVI-6.1 High-Speed LAN Instrument Protocol (HiSLIP) v2.0 April 23, 2020](https://www.ivifoundation.org/specifications/)
* [VXI-11 REVISION 1.0](https://www.vxibus.org/specifications.html)


## Usage

```toml
[dependencies]
#TBD
```

## Organisation

- [device](device)

- [hislip](hislip)
- [socket](socket)
- [vxi11](vxi11)
- [telnet](telnet)

- [client](client)

## Scope
This crate does not handle command parsing and/or execution, look at [scpi-rs](https://github.com/Atmelfan/scpi-rs)(:crab:) or [libscpi](https://github.com/j123b567/scpi-parser)(C) for that.


## License
This project is licensed under the MIT License, see [LICENSE.txt](/LICENSE.txt).

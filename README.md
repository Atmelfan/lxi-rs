# lxi-rs

This crate aims to simplify implementation of the [LXI Device Specification](https://www.lxistandard.org/Specifications/Specifications.aspx).
The specifications consists of a [core specification](https://www.lxistandard.org/members/Adopted%20Specifications/Latest%20Version%20of%20Standards_/LXI%20Standard%201.5%20Specifications/LXI%20Device%20Specification%20v1_5_01.pdf) and a optional set of extended functions.

Currently the focus is on implementing HiSLIP/VXI-11/Socket protocols for Unix-like environments. A long-term goal is to support an async no-std environment like [](https://github.com/embassy-rs/embassy)


# Relevant standards:
* [IVI-6.1 High-Speed LAN Instrument Protocol (HiSLIP) v2.0](https://www.ivifoundation.org/specifications/)
* [VXI-11 REVISION v1.0](https://www.vxibus.org/specifications.html)
* [LXI Device specification v1.5](https://www.lxistandard.org/members/Adopted%20Specifications/Latest%20Version%20of%20Standards_/LXI%20Standard%201.5%20Specifications/LXI%20Device%20Specification%20v1_5_01.pdf)

# Scope
This crate does not handle command parsing and/or execution, look at [scpi-rs](https://github.com/Atmelfan/scpi-rs)(:crab:) or [libscpi](https://github.com/j123b567/scpi-parser)(C) for that.

# Certificates
Secure extensions and https server requires a certificate and key. 

The simplest method is to use [`mkcert`](https://github.com/FiloSottile/mkcert) to generate one in `.certificates` directory:

```mkcert -key-file .certificates/key.pem -cert-file .certificates/cert.pem localhost 127.0.0.1 ::1```

# Examples
Each protocol includes an example service, you can try them out with `cargo run --example <protocol>` where protocol is either `hislip`,`vxi11`,`raw`, or `telnet`. 

Run `cargo run --example <protocol> -- --help` for help and specific arguments for each protocol.

# Testing
This crate uses two types of tests, the cargo test framework and pytest. Cargo test is mostly used for unit-testing while pytest is integration tests against pyvisa.
 
1. Install python requirements: `pip install -r requirements.txt`
2. [Optional but required to test HiSLIP] Install [NI-VISA](https://www.ni.com/sv-se/support/downloads/drivers/download.ni-visa.html) for Linux, see [pyvisa guide here](https://pyvisa.readthedocs.io/en/latest/faq/getting_nivisa.html#faq-getting-nivisa) 
3. Run tests: `cargo test && pytest`

## Coverage
1. Install `cargo-llvm-cov` and testing dependencies above.
2. Run `./coverage --open`

# Licensing
Lxi-rs is available under GPLv3 License, see [LICENSE-GPL](./LICENSE-GPL).

Core crates like [lxi-device](device) are licensed under MIT and APACHE version 2.

# Architecture


# Crates
* [lxi-device](./device/)  
    Contains abstractions for a LXI-device so that multiple protocol servers can co-exist. 
    Most notably thee core device trait which executes commands and share lock which handles exclusive/shared locking between the protocols.
* [lxi-hislip](./hislip/)  
    Server for the [HiSLIP protocol](https://www.ivifoundation.org/specifications/).
* [lxi-socket](./socket/)  
    Server for a simple raw socket protocol.
* [lxi-vxi11](./vxi11/)  
    Server for the [VXI-11 protocol](https://www.vxibus.org/specifications.html)

Protocol crates are divided into roughly three parts, a server part, a client part, and a common part. The common part usaually contains protocol decoding and infrastructure which can be used by both client and server.

The main focus of this project is the server-end so client is usally small and simple for testing and verification purposes.


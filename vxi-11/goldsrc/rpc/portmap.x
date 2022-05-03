const PMAP_PORT = 111;      /* portmapper port number */

const IPPROTO_TCP = 6;      /* protocol number for TCP/IP */
const IPPROTO_UDP = 17;     /* protocol number for UDP/IP */

struct mapping {
    unsigned int prog;
    unsigned int vers;
    unsigned int prot;
    unsigned int port;
};

// Doesn't work with xdrgen
//struct *pmaplist {
//    mapping map;
//    pmaplist next;
//};

struct call_args {
    unsigned int prog;
    unsigned int vers;
    unsigned int proc;
    opaque args<>;
};

struct call_result {
    unsigned int port;
    opaque res<>;
};
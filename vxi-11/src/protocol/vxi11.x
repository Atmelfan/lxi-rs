/* Types */
typedef long Device_Link;
enum Device_AddrFamily { /* used by interrupts */
   DEVICE_TCP,
   DEVICE_UDP
};
typedef long Device_Flags;

/* Error types */
typedef long Device_ErrorCode;
struct Device_Error {
   Device_ErrorCode error;
};

struct Create_LinkParms {
   long clientId; /* implementation specific value */
   bool lockDevice; /* attempt to lock the device */
   unsigned long lock_timeout; /* time to wait on a lock */
   string device<>; /* name of device */
};

struct Create_LinkResp {
   Device_ErrorCode error;
   Device_Link lid;
   unsigned short abortPort; /* for the abort RPC */
   unsigned long maxRecvSize; /* specifies max data size in bytes device will accept on a write */
};

struct Device_WriteParms {
   Device_Link lid; /* link id from create_link */
   unsigned long io_timeout; /* time to wait for I/O */
   unsigned long lock_timeout; /* time to wait for lock */
   Device_Flags flags; opaque data<>; /* the data length and the data itself */
};

struct Device_WriteResp {
   Device_ErrorCode error;
   unsigned long size; /* Number of bytes written */
};

struct Device_ReadParms {
   Device_Link lid; /* link id from create_link */
   unsigned long requestSize; /* Bytes requested */
   unsigned long io_timeout; /* time to wait for I/O */
   unsigned long lock_timeout; /* time to wait for lock */
   Device_Flags flags;
   char termChar; /* valid if flags & termchrset */
};

struct Device_ReadResp {
   Device_ErrorCode error;
   long reason; /* Reason(s) read completed */
   opaque data<>; /* data.len and data.val */
};

struct Device_ReadStbResp {
   Device_ErrorCode error; /* error code */
   unsigned char stb; /* the returned status byte */
};

struct Device_GenericParms {
   Device_Link lid; /* Device_Link id from connect call */
   Device_Flags flags; /* flags with options */
   unsigned long lock_timeout; /* time to wait for lock */
   unsigned long io_timeout; /* time to wait for I/O */
};

struct Device_RemoteFunc {
   unsigned long hostAddr; /* Host servicing Interrupt */
   unsigned short hostPort; /* valid port # on client */
   unsigned long progNum; /* DEVICE_INTR */
   unsigned long progVers; /* DEVICE_INTR_VERSION */
   Device_AddrFamily progFamily; /* DEVICE_UDP | DEVICE_TCP */
};

struct Device_EnableSrqParms {
   Device_Link lid;
   bool enable; /* Enable or disable interrupts */
   opaque handle<40>; /* Host specific data */
};

struct Device_LockParms {
   Device_Link lid; /* link id from create_link */
   Device_Flags flags; /* Contains the waitlock flag */
   unsigned long lock_timeout; /* time to wait to acquire lock */
};

struct Device_DocmdParms {
   Device_Link lid; /* link id from create_link */
   Device_Flags flags; /* flags specifying various options */
   unsigned long io_timeout; /* time to wait for I/O to complete */
   unsigned long lock_timeout; /* time to wait on a lock */
   long cmd; /* which command to execute */
   bool network_order; /* client's byte order */
   long datasize; /* size of individual data elements */
   opaque data_in<>; /* docmd data parameters */
};

struct Device_DocmdResp {
   Device_ErrorCode error; /* returned status */
   opaque data_out<>; /* returned data parameter */
};

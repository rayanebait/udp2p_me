# UDP2P

## Overview

UDP2P is a peer to peer file sharing system. It uses UDP to transfer data between peers and can achieve NAT traversal. It relies on a central server exposing an HTTP REST API for peer discovery. 

## Project organisation

```
.
└── crates
    ├── libs
    │   ├── lib-file
    │   ├── lib-network
    │   └── lib-web
    └── services
        ├── cli
        ├── server
        └── web-server
```

### Libraries

- `lib-file` : utils for the manipulation of files (reading and writing, merkle tree of the file system, chunking etc...)
- `lib-network` : utils for networking capabilities (sending and receiving messages, dealing with timeouts and resending etc...)
- `lib-web` : utils for interacting with the web server (peer discovery, file exportation, keep-alive etc...)

### Services

- `cli` : command line interface to interact with the server and other peers
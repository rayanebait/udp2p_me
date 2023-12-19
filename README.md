# UDP P2P file sharing system

## Project organisation

```
.
└── crates
    ├── libs
    │   ├── lib-auth
    │   ├── lib-core
    │   ├── lib-file
    │   ├── lib-network
    │   └── lib-web
    └── services
        ├── cli
        ├── server
        └── web-server
```

### Libraries

- `lib-auth` : utils for authentication related functionalities (certificates generation, verification, encryption of messages etc...)
- `lib-core` : utils for the general application logic
- `lib-file` : utils for the manipulation of files (reading and writing, merkle tree of the file system, chunking etc...)
- `lib-network` : utils for networking capabilities (sending and receiving messages, dealing with timeouts and resending etc...)
- `lib-web` : utils for interacting with the web server (peer discovery, file exportation, keep-alive etc...)

### Services

- `cli` : command line interface to interact with the server and other peers
- `server` : server code to respond to client requests
- `web-server` : (optional) REST web server for peers discovery 

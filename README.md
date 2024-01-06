# UDP2P

## Overview

UDP2P is a peer to peer file sharing system. It uses UDP to transfer data between peers and can achieve NAT traversal. It relies on a central server exposing an HTTP REST API for peer discovery.

## How to use

After building the binaries with `cargo build`, the cli tool can be use as follows.

- To fetch all the connected peers and their information from the REST server :
```
udp2p peers -u <url of rest server>
```

- To download the content associated to a hash from a peer :
```
udp2p download -p <peer address> -d <hash> -o <output path>
```

The download command will detect if the hash is pointing to a directory or a file and will either show the file system structure of the directory or download the file. By default, if no hash is provided, the client will look for the root hash and if no output path is provided, it downloads the file in `./dump`.

## Project organisation

```
.
└── crates
    ├── libs
    │   ├── lib-file
    │   ├── lib-network
    │   └── lib-web
    └── services
        └── cli
```

### Libraries

- `lib-file` : utils for the manipulation of files (reading and writing, merkle tree of the file system, chunking etc...)
- `lib-network` : utils for networking capabilities (sending and receiving messages, dealing with timeouts and resending etc...)
- `lib-web` : utils for interacting with the web server (peer discovery, file exportation, keep-alive etc...)

### Services

- `cli` : command line interface to interact with the server and other peers
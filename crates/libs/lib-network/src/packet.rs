use log::debug;
use prelude::*;
use std::fmt::Display;

// use crate::peer_data::*;

/*For the id generation */
use nanorand::{wyrand::WyRand, BufferedRng, Rng};

use sha2::{Digest, Sha256};
/*Utilities */
use anyhow::Result;

/*Async/net libraries */
use std::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;

pub const HASH_OF_EMPTY_STRING: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Debug)]
pub enum PacketError {
    NoIdError,
    NoTypeError,
    NoLengthError,
    NoBodyError,
    NoSignatureError,
    InvalidIdError,
    InvalidFormatError,
    UnknownError,
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::NoIdError => write!(f, "Packet builder failed"),
            PacketError::NoTypeError => write!(f, "Packet builder failed"),
            PacketError::NoLengthError => write!(f, "Packet builder failed"),
            PacketError::NoBodyError => write!(f, "Packet builder failed"),
            PacketError::NoSignatureError => write!(f, "Unsigned Packet"),
            PacketError::InvalidIdError => write!(f, "Packet has invalid Id"),
            PacketError::InvalidFormatError => write!(f, "Packet has invalid format"),
            PacketError::UnknownError => write!(f, "Packet has invalid format"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketType {
    NoOp = 0,
    Error,
    Hello,
    PublicKey,
    Root,
    GetDatum,
    NatTraversalRequest,
    NatTraversal,
    ErrorReply = 128,
    HelloReply,
    PublicKeyReply,
    RootReply,
    Datum,
    NoDatum,
}

impl Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketType::NoOp => write!(f, "NoOp"),
            PacketType::Hello => write!(f, "Hello"),
            PacketType::HelloReply => write!(f, "HelloReply"),
            PacketType::Error => write!(f, "Error"),
            PacketType::ErrorReply => write!(f, "ErrorReply"),
            PacketType::PublicKey => write!(f, "PublicKey"),
            PacketType::PublicKeyReply => write!(f, "PublicKeyReply"),
            PacketType::Root => write!(f, "Root"),
            PacketType::RootReply => write!(f, "RootReply"),
            PacketType::GetDatum => write!(f, "GetDatum"),
            PacketType::Datum => write!(f, "Datum"),
            PacketType::NoDatum => write!(f, "NoDatum"),
            PacketType::NatTraversalRequest => write!(f, "NatTraversalRequest"),
            PacketType::NatTraversal => write!(f, "NatTraversal"),
        }
    }
}

impl PacketType {
    fn from_u8(val: u8) -> Result<PacketType, PacketError> {
        match val {
            0 => return Ok(PacketType::NoOp),
            1 => return Ok(PacketType::Error),
            2 => return Ok(PacketType::Hello),
            3 => return Ok(PacketType::PublicKey),
            4 => return Ok(PacketType::Root),
            5 => return Ok(PacketType::GetDatum),
            6 => return Ok(PacketType::NatTraversalRequest),
            7 => return Ok(PacketType::NatTraversal),
            128 => return Ok(PacketType::ErrorReply),
            129 => return Ok(PacketType::HelloReply),
            130 => return Ok(PacketType::PublicKeyReply),
            131 => return Ok(PacketType::RootReply),
            132 => return Ok(PacketType::Datum),
            133 => return Ok(PacketType::NoDatum),
            _ => return Err(PacketError::NoTypeError),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Packet {
    id: [u8; 4],
    packet_type: PacketType,
    length: usize,
    body: Vec<u8>,
    signature: Option<[u8; 64]>,
}

impl Default for Packet {
    fn default() -> Self {
        let mut rng = BufferedRng::new(WyRand::new());
        let mut id = [0u8; 4];
        rng.fill(&mut id);
        return Packet {
            id,
            packet_type: PacketType::NoOp,
            length: 0,
            body: Vec::new(),
            signature: None,
        };
    }
}

#[derive(Default)]
pub struct PacketBuilder {
    id: Option<[u8; 4]>,
    packet_type: Option<PacketType>,
    length: Option<usize>,
    body: Option<Vec<u8>>,
    signature: Option<[u8; 64]>,
}

impl PacketBuilder {
    pub fn build(&self) -> Result<Packet, PacketError> {
        let Some(id) = self.id else {
            return Err(PacketError::InvalidIdError);
        };
        let Some(packet_type) = self.packet_type else {
            return Err(PacketError::NoTypeError);
        };

        let Some(length) = self.length else {
            return Err(PacketError::NoLengthError);
        };
        let Some(body) = self.body.to_owned() else {
            return Err(PacketError::NoBodyError);
        };

        Ok(Packet {
            id: id,
            packet_type: packet_type,
            length: length,
            body: body,
            signature: self.signature,
        })
    }

    pub fn new() -> Self {
        Self {
            /*nanorand the id */
            packet_type: Some(PacketType::NoOp),
            body: Some(vec![]),
            length: Some(0),
            ..PacketBuilder::default()
        }
    }
    pub fn noop_packet() -> Packet {
        let hello_packet = PacketBuilder::new()
            .gen_id()
            .packet_type(PacketType::NoOp)
            .build();
        hello_packet.unwrap()
    }
    pub fn hello_packet(extensions: Option<&[u8; 4]>, name: Vec<u8>) -> Packet {
        let mut body = match extensions {
            Some(extensions) => extensions.to_vec(),
            None => vec![0, 0, 0, 0],
        };
        let mut name = name.clone();
        body.append(&mut name);
        let hello_packet = PacketBuilder::new()
            .gen_id()
            .body(body)
            .packet_type(PacketType::Hello)
            .build();
        hello_packet.unwrap()
    }

    pub fn hello_reply_packet(id: &[u8; 4], extensions: Option<[u8; 4]>, name: Vec<u8>) -> Packet {
        let mut body = match extensions {
            Some(extensions) => extensions.to_vec(),
            None => vec![0, 0, 0, 0],
        };
        let mut name = name.clone();
        body.append(&mut name);
        let hello_packet = PacketBuilder::new()
            .set_id(*id)
            .body(body)
            .packet_type(PacketType::HelloReply)
            .build();
        hello_packet.unwrap()
    }
    pub fn error_packet(err_msg: Option<Vec<u8>>) -> Packet {
        let err_msg = match err_msg {
            Some(err_msg) => err_msg.to_vec(),
            None => vec![],
        };

        let err_packet = PacketBuilder::new()
            .gen_id()
            .body(err_msg)
            .packet_type(PacketType::Error)
            .build();

        err_packet.unwrap()
    }
    pub fn error_reply_packet(id: &[u8; 4], err_msg: Option<Vec<u8>>) -> Packet {
        let err_msg = match err_msg {
            Some(err_msg) => err_msg.to_vec(),
            None => vec![],
        };

        let err_reply_packet = PacketBuilder::new()
            .set_id(*id)
            .body(err_msg)
            .packet_type(PacketType::ErrorReply)
            .build();

        err_reply_packet.unwrap()
    }
    pub fn public_key_packet(public_key: Option<[u8; 64]>) -> Packet {
        let public_key = match public_key {
            Some(public_key) => public_key.to_vec(),
            None => vec![],
        };

        let public_key_packet = PacketBuilder::new()
            .gen_id()
            .body(public_key)
            .packet_type(PacketType::PublicKey)
            .build();

        public_key_packet.unwrap()
    }
    pub fn public_key_reply_packet(public_key: Option<[u8; 64]>, id: [u8; 4]) -> Packet {
        let public_key = match public_key {
            Some(public_key) => public_key.to_vec(),
            None => vec![],
        };
        let public_key_packet = PacketBuilder::new()
            .set_id(id)
            .body(public_key)
            .packet_type(PacketType::PublicKeyReply)
            .build();

        public_key_packet.unwrap()
    }

    pub fn root_packet(root: Option<[u8; 32]>) -> Packet {
        let root = match root {
            Some(root) => root.to_vec(),
            None => hex::decode(HASH_OF_EMPTY_STRING).unwrap(),
        };

        let root_packet = PacketBuilder::new()
            .gen_id()
            .body(root)
            .packet_type(PacketType::Root)
            .build();

        root_packet.unwrap()
    }

    pub fn root_reply_packet(id: &[u8; 4], root: Option<[u8; 32]>) -> Packet {
        let root = match root {
            Some(root) => root.to_vec(),
            None => hex::decode(HASH_OF_EMPTY_STRING).unwrap(),
        };

        let root_packet = PacketBuilder::new()
            .set_id(*id)
            .body(root)
            .packet_type(PacketType::RootReply)
            .build();

        root_packet.unwrap()
    }
    pub fn get_datum_packet(hash: [u8; 32]) -> Packet {
        let get_datum_packet = PacketBuilder::new()
            .gen_id()
            .body(hash.to_vec())
            .packet_type(PacketType::GetDatum)
            .build();

        get_datum_packet.unwrap()
    }
    pub fn datum_packet(id: &[u8; 4], hash: [u8; 32], datum: Vec<u8>) -> Packet {
        let mut body = hash.to_vec();
        let mut datum = datum.clone();
        body.append(&mut datum);
        let datum_packet = PacketBuilder::new()
            .set_id(*id)
            .body(body)
            .packet_type(PacketType::Datum)
            .build();

        datum_packet.unwrap()
    }
    pub fn nodatum_packet(id: &[u8; 4]) -> Packet {
        let nodatum_packet = PacketBuilder::new()
            .set_id(*id)
            .body(vec![])
            .packet_type(PacketType::NoDatum)
            .build();

        nodatum_packet.unwrap()
    }
    pub fn nat_traversal_request_packet(behind_nat_addr: Vec<u8>) -> Packet {
        let nat_traversal_requet_packet = PacketBuilder::new()
            .gen_id()
            .body(behind_nat_addr)
            .packet_type(PacketType::NatTraversalRequest)
            .build();

        nat_traversal_requet_packet.unwrap()
    }
    pub fn nat_traversal_request_from_addr_packet(behind_nat_addr: SocketAddr) -> Packet {
        let raw_socketaddr = {
            let ip_addr = behind_nat_addr.ip();
            let port = behind_nat_addr.port();

            let mut parse_sock_addr = match ip_addr {
                IpAddr::V4(ip4_addr) => ip4_addr.octets().to_vec(),
                IpAddr::V6(ip6_addr) => ip6_addr.octets().to_vec(),
            };

            parse_sock_addr.push((port >> 8) as u8);
            parse_sock_addr.push((port & 0xff) as u8);
            parse_sock_addr
        };

        PacketBuilder::nat_traversal_request_packet(raw_socketaddr)
    }

    pub fn packet_type(&mut self, packet_type: PacketType) -> &mut Self {
        self.packet_type = Some(packet_type);
        self
    }

    /*WyRand is not cryptographically secure but it is really
    fast (16 Gb/s) so won't slow down the p2p protocol */
    pub fn gen_id(&mut self) -> &mut Self {
        let mut rng = BufferedRng::new(WyRand::new());
        let mut buf: [u8; 4] = [0; 4];

        rng.fill(&mut buf);
        self.id = Some(buf);

        self
    }

    pub fn set_id(&mut self, id: [u8; 4]) -> &mut Self {
        self.id = Some(id);
        self
    }
    pub fn body(&mut self, body: Vec<u8>) -> &mut Self {
        let body_length = body.len();

        self.length = match body_length {
            0..=1128 => Some(body_length),
            1129.. => panic!("Invalid packet"),
            _ => panic!("Shouldn't happen"),
        };

        self.body = Some(body);

        self
    }
    pub fn signature(&mut self, signature: Option<[u8; 64]>) -> &mut Self {
        self.signature = signature;
        self
    }
}

impl Packet {
    pub fn get_id(&self) -> &[u8; 4] {
        &self.id
    }
    pub fn get_body(&self) -> &Vec<u8> {
        &self.body
    }
    pub fn get_body_length(&self) -> usize {
        self.length
    }
    pub fn get_packet_type(&self) -> &PacketType {
        &self.packet_type
    }
    pub fn get_signature(&self) -> Result<&[u8; 64], PacketError> {
        match &self.signature {
            Some(sig) => Ok(sig),
            None => Err(PacketError::NoSignatureError),
        }
    }
    pub fn is_response(&self) -> bool {
        let packet_type = self.packet_type as u8;
        match packet_type {
            0..=7 => return false,
            _ => return true,
        }
    }

    pub fn is(&self, other_packet_type: PacketType) -> bool {
        *self.get_packet_type() == other_packet_type
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut packet_buf: Vec<u8> = vec![];
        for byte in self.id {
            packet_buf.push(byte);
        }

        packet_buf.push(self.packet_type as u8);

        packet_buf.push(((self.length >> 8) & 0xff).try_into().unwrap());
        packet_buf.push((self.length & 0xff).try_into().unwrap());

        for byte in &self.body {
            /*byte is copied ? */
            packet_buf.push(*byte);
        }

        match self.signature {
            Some(sig) => {
                for byte in sig {
                    packet_buf.push(byte);
                }
            }
            None => (),
        }

        packet_buf
    }

    /*Using drain to check if empty after,
    maybe should be verified manually so that
    an invalid packet doesn't make the program
        panic */
    pub fn from_bytes(raw_packet: &mut Vec<u8>) -> Result<Packet, PacketError> {
        /*Should check len */
        let mut len = raw_packet.len();
        let mut copied = 0;
        if len < 7 {
            return Err(PacketError::InvalidFormatError);
        }
        len -= 7;

        let mut id: [u8; 4] = [0; 4];
        id.copy_from_slice(&raw_packet.as_slice()[copied..4]);
        copied += 4;

        let packet_type = PacketType::from_u8(raw_packet[copied])?;
        copied += 1;

        let length = {
            let mut length_as_bytes: [u8; 2] = [0; 2];
            length_as_bytes.copy_from_slice(&raw_packet.as_slice()[copied..(copied + 2)]);

            (*length_as_bytes.get(0).unwrap() as usize) * (256)
                + (*length_as_bytes.get(1).unwrap() as usize)
        };
        copied += 2;

        if len < length {
            return Err(PacketError::InvalidFormatError);
        }

        let body: Vec<u8>;
        if length == 0 {
            body = vec![];
        } else {
            body = raw_packet.as_slice()[copied..(copied + length)].to_vec();
        }
        copied += length;
        len -= length;

        let signature: Option<[u8; 64]>;
        if len == 0 {
            signature = None;
        } else if len < 64 {
            return Err(PacketError::InvalidFormatError);
        } else {
            signature = {
                let mut signature = [0; 64];
                signature.copy_from_slice(&raw_packet.as_slice()[copied..copied + 64]);
                Some(signature)
            }
        }

        Ok(PacketBuilder::new()
            .set_id(id)
            .packet_type(packet_type)
            .body(body)
            .signature(signature)
            .build()
            .unwrap())
    }

    pub fn raw_length(&self) -> usize {
        let add_64_if_signed = match self.signature {
            Some(_) => 64,
            None => 0,
        };

        32 + 1 + 2 + self.length + add_64_if_signed
    }

    pub async fn send_to_addr(
        &self,
        sock: &UdpSocket,
        addr: &SocketAddr,
    ) -> Result<usize, PacketError> {
        match sock.writable().await {
            Ok(_) => {
                let res = sock.try_send_to(self.as_bytes().as_slice(), addr.clone());
                match res {
                    Ok(size) => return Ok(size),
                    Err(_e) => return Err(PacketError::UnknownError),
                }
            }
            Err(_) => return Err(PacketError::UnknownError),
        }
    }

    pub async fn recv_from(sock: &UdpSocket) -> Result<(SocketAddr, Packet), PacketError> {
        /*1095=4+1+2+(32+1024)+64 being the maximum packet size*/
        let mut packet_buf: [u8; 1128] = [0; 1128];

        let (recvd_packet_size, peer_addr) = sock.recv_from(&mut packet_buf).await.unwrap();

        let response = (
            peer_addr,
            match Packet::from_bytes(&mut {
                let mut packet_buf = packet_buf.to_vec();
                packet_buf.truncate(recvd_packet_size);
                packet_buf
            }) {
                Ok(packet) => packet,
                Err(e) => return Err(e),
            },
        );

        Ok(response)
    }

    /*Verify the hash of a Packet during p2p export/import */
    /*Should also take a hash  */
    pub fn valid_hash(&self) -> bool {
        debug!("PACKET HASH CHECKING : {self:?}");
        let body = self.get_body();
        let given_hash = &body.as_slice()[0..32];

        let calculated_hash = {
            let data = &body.as_slice()[32..self.get_body_length()];
            let mut hasher = Sha256::new();
            hasher.update(data);

            let mut calculated_hash = <[u8; 32]>::default();
            calculated_hash.copy_from_slice(hasher.finalize().as_slice());
            calculated_hash
        };

        calculated_hash == given_hash
    }
}

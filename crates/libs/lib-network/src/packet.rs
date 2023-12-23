
use std::default;
use prelude::*;

// use crate::peer_data::*;

/*For the id generation */
use nanorand::{Rng, BufferedRng, wyrand::WyRand};

use sha2::{Sha256, Digest};
/*Utilities */
use anyhow::Result;
    
/*Async/net libraries */
use tokio::net::UdpSocket;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum PacketError{
    NoIdError,
    NoTypeError,
    NoLengthError,
    NoBodyError,
    NoSignatureError,
    InvalidIdError,
    InvalidFormatError,
    UnknownError,
}

impl std::fmt::Display for PacketError{
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
pub enum PacketType{
    NoOp=0,
    Error,
    Hello,
    PublicKey,
    Root,
    GetDatum,
    NatTraversal,
    ErrorReply=128,
    HelloReply,
    PublicKeyReply,
    RootReply,
    Datum,
    NatTraversalReply,
}

impl PacketType {
    fn from_u8(val: u8)-> Result<PacketType, PacketError>{
        match val {
            0 => return Ok(PacketType::NoOp),
            1 => return Ok(PacketType::Error),
            2 => return Ok(PacketType::Hello),
            3 => return Ok(PacketType::PublicKey),
            4 => return Ok(PacketType::Root),
            5 => return Ok(PacketType::GetDatum),
            6 => return Ok(PacketType::NatTraversal),
            128 => return Ok(PacketType::ErrorReply),
            129 => return Ok(PacketType::HelloReply),
            130 => return Ok(PacketType::PublicKeyReply),
            131 => return Ok(PacketType::RootReply),
            132 => return Ok(PacketType::Datum),
            133 => return Ok(PacketType::NatTraversalReply),
            _ => return Err(PacketError::NoTypeError),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Packet{
    id: [u8;4],
    packet_type: PacketType,
    length: usize,
    body: Vec<u8>,
    signature: Option<[u8; 64]>,
}

#[derive(Default)]
pub struct PacketBuilder{
    id: Option<[u8;4]>,
    packet_type: Option<PacketType>,
    length: Option<usize>,
    body: Option<Vec<u8>>,
    signature: Option<[u8; 64]>,
}

impl PacketBuilder {
    pub fn build(&self)-> Result<Packet, PacketError>{
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
            return Err(PacketError::NoBodyError)
        };

        Ok(Packet {
            id: id, 
            packet_type: packet_type,
            length: length,
            body: body,
            signature: self.signature,
        })
    }

    pub fn new()->Self{
        Self{
            /*nanorand the id */
            packet_type: Some(PacketType::NoOp),
            body: Some(vec![]),
            length: Some(0),
            ..PacketBuilder::default()
        }
    }
    pub fn hello_packet()->Packet{
        let hello_packet = PacketBuilder::new()
                                .gen_id()
                                .packet_type(PacketType::Hello)
                                .build();
        hello_packet.unwrap()
    }

    pub fn hello_reply_packet(id: &[u8;4])->Packet{
        let hello_packet = PacketBuilder::new()
                                .set_id(*id)
                                .packet_type(PacketType::HelloReply)
                                .build();
        hello_packet.unwrap()
    }
    pub fn public_key_reply_packet(public_key: Option<[u8;64]>, id: [u8;4])->Packet{
        let public_key = match public_key{
            Some(public_key)=> public_key.to_vec(),
            None => vec![],
        };
        let public_key_packet = PacketBuilder::new()
                        .set_id(id)
                        .body(public_key)
                        .packet_type(PacketType::PublicKeyReply)
                        .build();

        public_key_packet.unwrap()
    }
    pub fn public_key_packet(public_key: Option<[u8;64]>)->Packet{
        let public_key = match public_key{
            Some(public_key)=> public_key.to_vec(),
            None => vec![],
        };

        let public_key_packet = PacketBuilder::new()
                        .gen_id()
                        .body(public_key)
                        .packet_type(PacketType::PublicKey)
                        .build();

        public_key_packet.unwrap()
    }
    pub fn root_reply_packet(root: Option<[u8;32]>)->Packet{
    
        let root = match root{
            Some(root)=> root.to_vec(),
            None => vec![],
        };

        let root_packet = PacketBuilder::new()
                        .gen_id()
                        .body(root)
                        .packet_type(PacketType::RootReply)
                        .build();

        root_packet.unwrap()
    }

    pub fn root_packet(root: Option<[u8;32]>)->Packet{
        let root = match root{
            Some(root)=> root.to_vec(),
            None => vec![],
        };

        let root_packet = PacketBuilder::new()
                        .gen_id()
                        .body(root)
                        .packet_type(PacketType::Root)
                        .build();

        root_packet.unwrap()
    }

    pub fn packet_type(&mut self, packet_type: PacketType)->&mut Self{
        self.packet_type = Some(packet_type);
        self
    }

    /*WyRand is not cryptographically secure but it is really 
    fast (16 Gb/s) so won't slow down the p2p protocol */
    pub fn gen_id(&mut self)-> &mut Self{
        let mut rng = BufferedRng::new(WyRand::new());
        let mut buf : [u8;4] = [0; 4];

        rng.fill(&mut buf);
        self.id = Some(buf);

        self
    }

    pub fn set_id(&mut self, id: [u8;4])-> &mut Self{
        self.id = Some(id);
        self
    }
    pub fn body(&mut self, body: Vec<u8>)-> &mut Self{
        let body_length = body.len();

        self.length = match body_length{
            0..=1024 => Some(body_length),
            1025.. => panic!("Invalid packet"),
            _ => panic!("Shouldn't happen"),
        };

        self.body = Some(body);

        self
    }
    pub fn signature(&mut self, signature: Option<[u8;64]>)->&mut Self{
        self.signature = signature;
        self
    }

}

impl Packet {
    pub fn get_id(&self)-> &[u8;4]{
        &self.id
    }
    pub fn get_body(&self)-> &Vec<u8>{
        &self.body
    }
    pub fn get_body_length(&self)-> &usize{
        &self.length
    }
    pub fn get_packet_type(&self)-> &PacketType{
        &self.packet_type
    }
    pub fn get_signature(&self)-> Result<&[u8;64], PacketError>{
        match &self.signature {
            Some(sig)=> Ok(sig),
            None => Err(PacketError::NoSignatureError),
        }
    }
    pub fn is_response(&self)->bool{
        let packet_type = self.packet_type as u8;
        match packet_type{
            0..=7 => return true,
            _ => return false,
        }
    }

    pub fn is(&self, other_packet_type: &PacketType)->bool{
        *self.get_packet_type() == *other_packet_type
    }


    pub fn as_bytes(&self)->Vec<u8>{
        let mut packet_buf : Vec<u8> = vec![];
        for byte in self.id {
            packet_buf.push(byte);
        }

        packet_buf.push(self.packet_type as u8);

        packet_buf.push(((self.length>>8)&0xff).try_into().unwrap());
        packet_buf.push((self.length&0xff).try_into().unwrap());

        for byte in &self.body {
            /*byte is copied ? */
            packet_buf.push(*byte);
        }

        match self.signature {
            Some(sig)=> for byte in sig {
                packet_buf.push(byte);
            },
            None => (),
        }

        packet_buf
    }

    /*Using drain to check if empty after,
    maybe should be verified manually so that
    an invalid packet doesn't make the program
        panic */
    pub fn from_bytes(raw_packet: &mut Vec<u8>)->Self{
        /*Should check len */
        let id : [u8;4]= raw_packet.drain(0..=3).as_slice().try_into()
                            .unwrap();
        let packet_type= PacketType::from_u8(
                        raw_packet.drain(0..1)
                                            .as_slice()[0])
                                            .unwrap();
        let length ={
                let length_bytes : [u8; 2] = raw_packet
                                .drain(0..=1)
                                .as_slice()
                                .try_into()
                                .unwrap();
                let length = (length_bytes[0] as usize)>>8
                                    + (length_bytes[1] as usize);
                length
            };

        Self{
            id: id,
            packet_type: packet_type,
            length: length.clone(),
            body:{
                let body : Vec<u8>= raw_packet.drain(0..length).collect();
                body
            },
            signature: match raw_packet.is_empty() {
                true=> None,
                false => Some(
                    raw_packet.drain(0..64).as_slice().try_into().unwrap()
                ),
            }
        }
    }

    pub fn raw_length(&self)-> usize{
        let add_64_if_signed = match self.signature {
            Some(_)=> 64,
            None => 0,
        };

        32+1+2+self.length+add_64_if_signed
    }

    pub async fn send_to_addr(&self, sock: &UdpSocket,
             addr: &SocketAddr)->Result<usize, PacketError>{

        sock.writable().await;
        let res = sock.try_send_to(self
                                .as_bytes()
                                .as_slice(),
                            addr.clone());


        match res {
            Ok(size)=> return Ok(size),
            Err(e)=> return Err(PacketError::UnknownError),
        }
    }

    pub async fn recv_from(sock: &UdpSocket)
                            ->Result<(SocketAddr ,Packet), PacketError>{
        /*1095=4+1+2+(32+1024)+64 being the maximum packet size*/
        let mut packet_buf: [u8; 1127] = [0;1127];

        let (recvd_packet_size, peer_addr) =
                        sock.recv_from(&mut packet_buf).await.unwrap();

        let response = 
            (peer_addr,
             Packet::from_bytes(&mut {
                                    let mut packet_buf = packet_buf.to_vec();
                                    packet_buf.truncate(recvd_packet_size);
                                    packet_buf
                                })
            );

        Ok(response)
    }

    // pub async fn send_to(&self, sock: &UdpSocket,
    //          peer: &PeerData)->Result<usize, PeerError>{

    //     /*If peer has a working addr, send to it, else try all addrs. */
    //     let addr = match peer.get_good_socketaddr(){
    //         Ok(good_addrs)=> good_addrs.get(0),
    //         Err(PeerError::NoGoodAddrError)=> {
    //             let addrs = peer.get_socketaddr().unwrap();
    //             addrs.get(0)
    //         }
    //         _=> panic!("Shouldn't happen"),
    //     };

    //     let addr = match addr {
    //         Some(addr) => addr,
    //         None => return Err(PeerError::NoAddrsError),
    //     };
            
    //     self.send_to_addr(sock, &addr).await
    // }


    // pub async fn send_hello(sock: &UdpSocket,
    //             peer_addr: &SocketAddr)->Result<[u8;4], PeerError>{

    //     let hello_packet = PacketBuilder::hello_packet();            

    //     match hello_packet.send_to_addr(sock, peer_addr).await {
    //         Ok(_)=> Ok(hello_packet.get_id().clone()),
    //         Err(e)=>Err(e),
    //     }
    // }

    // pub async fn send_hello_reply(sock: &UdpSocket,
    //             peer_addr: &SocketAddr)->Result<(), PeerError>{

    //     let hello_packet = PacketBuilder::hello_reply_packet();            

    //     match hello_packet.send_to_addr(sock, peer_addr).await {
    //         Ok(_)=> Ok(()),
    //         Err(e)=>Err(e),
    //     }
    // }

    // pub async fn send_public_key_reply(sock: &UdpSocket, id: [u8;4],
    //             peer_addr: &SocketAddr, public_key: Option<[u8;64]>)->Result<(), PeerError>{
    //     let body = match public_key {
    //         Some(pkey) => {
    //             pkey.to_vec()
    //         },
    //         None => vec![],
    //     };
    //     let public_key_packet = PacketBuilder::new()
    //                                             .set_id(id)
    //                                             .packet_type(PacketType::PublicKeyReply)
    //                                             .body(body)
    //                                             .build()
    //                                             .unwrap();

    //     match public_key_packet.send_to_addr(sock, peer_addr).await {
    //         Ok(_)=> Ok(()),
    //         Err(e)=>Err(e),
    //     }
    // }

    // pub async fn send_root_reply(sock: &UdpSocket, id: [u8;4],
    //         peer_addr: &SocketAddr, root: Option<[u8;32]>)->Result<(), PeerError>{
    //     let body = match root{
    //         Some(root_hash) => {
    //             root_hash.to_vec()
    //         },
    //         None => vec![],
    //     };
    //     let root_packet = PacketBuilder::new()
    //                                             .set_id(id)
    //                                             .packet_type(PacketType::RootReply)
    //                                             .body(body)
    //                                             .build()
    //                                             .unwrap();

    //     match root_packet.send_to_addr(sock, peer_addr).await {
    //         Ok(_)=> Ok(()),
    //         Err(e)=>Err(e),
    //     }
        
    // }

    // pub async fn send_datum(sock: &UdpSocket, id: [u8;4],
    //         peer_addr: &SocketAddr, datum: [u8;1024], hash: [u8; 32])->Result<(), PeerError>{

    //     let body = {
    //         let mut body : Vec<u8> = hash.to_vec();
    //         body.append(&mut datum.to_vec());
    //         body
    //     };
    //     let datum_packet = PacketBuilder::new()
    //                                             .set_id(id)
    //                                             .packet_type(PacketType::Datum)
    //                                             .body(body)
    //                                             .build()
    //                                             .unwrap();

    //     match datum_packet.send_to_addr(sock, peer_addr).await {
    //         Ok(_)=> Ok(()),
    //         Err(e)=>Err(e),
    //     }

    // // }

    /*Verify the hash of a Packet during p2p export/import */
    pub fn valid_hash(&self)->bool{
        let body = self.get_body();
        let given_hash = &body.as_slice()[0..32];

        let calculated_hash = {
            let data = &body.as_slice()[32..(body.len())];
            let mut hasher = Sha256::new();
            hasher.update(data);

            let mut calculated_hash = <[u8; 32]>::default();
            calculated_hash.copy_from_slice(hasher.finalize().as_slice());
            calculated_hash
        };

        calculated_hash == given_hash
    }

}
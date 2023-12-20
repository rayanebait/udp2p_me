use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use futures::future::pending;

use crate::congestion_handler::*;
use crate::packet::*;
use crate::peer_data::PeerData;

pub enum Action {
    A,
    /*A hashmap is available in ActivePeers: SocketAddr to Peer */
    SendHelloReply(SocketAddr),
    ProcessHelloReply(SocketAddr),

    SendDatumWithHash([u8;32], SocketAddr),
    ProcessDatum(Vec<u8>, SocketAddr),

    StorePublicKey(Option<[u8;64]>, SocketAddr),
    StoreRoot(Option<[u8;32]>, SocketAddr),

    SendPublicKey(Option<[u8;64]>, SocketAddr),
    SendRoot([u8;32], SocketAddr),

    SendError(Vec<u8>, SocketAddr),
    ProcessErrorReply(Vec<u8>, SocketAddr),

    NoOp(SocketAddr)
}

pub enum HandlingError{
    InvalidPacketError,
    InvalidHashError,
}

pub fn handle_packet(packet: Packet, socket_addr: SocketAddr,
                        pending_ids: Arc<Mutex<PendingIds>>)->Result<Action, HandlingError>{
    
    let id_exists = {
        /*get mutex to check and pop id if it is a response */
        let mut pending_ids_guard = 
            match pending_ids.lock() {
                Ok(guard) => guard,
                /*If Mutex is poisoned stop every thread, something is wrong */
                Err(PoisonError)=> panic!("Poisoned Ids Mutex"),
            };

            /*Check if id exists */
            match pending_ids_guard.search_id(&packet){
                Ok(sock_addr) => {
                    /*if id exists, pop the packet before handling it. */
                    pending_ids_guard.pop_packet_id(packet.get_id());

                    /*Now check if the address it was sent to is
                     the same as the address it was received from */
                    let addr_matches_id = {
                        let addr_match_id;

                        if sock_addr != socket_addr {
                            addr_match_id = Err(CongestionHandlerError::AddrAndIdDontMatchError);
                        }else {
                            addr_match_id= Ok(sock_addr);
                        }
                        addr_match_id
                    };
                    addr_matches_id
                },
                Err(CongestionHandlerError::NoPacketWithIdError) =>
                             Err(CongestionHandlerError::NoPacketWithIdError),
                _ => panic!("Shouldn't happen"),
            }
            /*Mutex is dropped here */
    };

    match id_exists {
            /*Packet is a response */
        Ok(sock_addr) => handle_response_packet(
                                packet, socket_addr, pending_ids),
            /*Packet is a request */
        Err(CongestionHandlerError::NoPacketWithIdError)=> handle_request_packet(
                                                        packet, socket_addr,
                                                        pending_ids), 

            /*Id is known but now Peer: discard packet.
            Note: There could be collisions between ids
            but with negligible proba ? */
        Err(CongestionHandlerError::AddrAndIdDontMatchError)=>
                             Err(HandlingError::InvalidPacketError),
        Err(_) => panic!("Shouldn't happen"),
    }

}

/*Server */
fn handle_request_packet(packet: Packet, socket_addr: SocketAddr,
                                    pending_ids: Arc<Mutex<PendingIds>>)
                                            ->Result<Action, HandlingError>{

    match packet.get_packet_type() {
        PacketType::NoOp => Ok(Action::A),
        PacketType::Error => Ok(Action::A),
        PacketType::Hello => Ok(Action::A),
        PacketType::PublicKey=>Ok(Action::A),
        PacketType::Root => Ok(Action::A),
        /*Exports should have its own send/receive queue?*/
        PacketType::Datum => Ok(Action::A),
        PacketType::NatTraversal => Ok(Action::A),
        /*Invalid packet, should send error*/
        _ => Err(HandlingError::InvalidPacketError),
    }
}

/*Client */
fn handle_response_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: Arc<Mutex<PendingIds>>)
                                        ->Result<Action, HandlingError>{

    match packet.get_packet_type() {
        PacketType::ErrorReply => {
            let error_message = packet.get_body().to_owned();
            println!("Received ErrorReply with body: {}\n",
                    String::from_utf8_lossy(error_message.as_slice()));
            Ok(Action::ProcessErrorReply(error_message, socket_addr))
        },
        PacketType::HelloReply => {
            println!("Receive HelloReply from peer at {}\n", socket_addr);
            Ok(Action::ProcessHelloReply(socket_addr))
        },
        PacketType::PublicKeyReply => {
            println!("Receive PublicKeyReply from peer at {}\n", socket_addr);
            Ok(Action::StorePublicKey(
                match packet.get_body_length(){
                    /*Peer doesn't implement signatures */
                0 => None,
                    /*Peer implements signatures */
                _ => Some({
                        let mut public_key: [u8;64];
                        public_key.copy_from_slice(&packet.get_body().as_slice()[0..64]);
                        public_key
                    }),
                }, socket_addr))
        },
        PacketType::RootReply =>{
            println!("Receive RootReply from peer at {}\n", socket_addr);
            Ok(Action::StoreRoot(
                match packet.get_body_length(){
                    /*Peer doesn't implement signatures */
                0 => None,
                    /*Peer implements signatures */
                _ => Some({
                        let mut root: [u8;32];
                        root.copy_from_slice(&packet.get_body().as_slice()[0..32]);
                        root
                    }),
                }, socket_addr))
        },
        PacketType::Datum =>{
            println!("Receive Datum from peer at {}\n", socket_addr);
            match packet.valid_hash() {
                true => Ok(Action::ProcessDatum(packet.get_body().to_owned(), socket_addr)),
                false => Err(HandlingError::InvalidHashError),
            }
        },
        PacketType::NatTraversalReply =>{
            println!("Receive NatTraversalReply from peer at {}\n", socket_addr);
            Ok(Action::A)
        },
        _=> Err(HandlingError::InvalidPacketError),
    }

}
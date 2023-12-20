use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use futures::future::pending;

use crate::congestion_handler::*;
use crate::packet::*;
use crate::peer_data::PeerData;

pub enum PacketState {
    New,
    Received
}

pub fn handle_packet(packet: Packet, socket_addr: SocketAddr,
                        pending_ids: Arc<Mutex<PendingIds>>,
                         receive_queue: Arc<Mutex<ReceiveQueue>>,
                          send_queue: Arc<Mutex<SendQueue>>){
    
    {
        let mut pending_ids_guard = 
            match pending_ids.lock() {
                Ok(guard) => guard,
                Err(PoisonError)=> panic!("Poisoned Ids Mutex"),
            };

    }

}

fn handle_request_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: &mut PendingIds){

}

fn handle_response_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: &mut PendingIds){
    if !packet.is_response() {
        return;
    }
    
    match packet.get_packet_type() {
        PacketType::ErrorReply => {
            let error_message = packet.get_body();
            println!("Received ErrorReply with body: {}\n",
                    String::from_utf8_lossy(error_message.clone().as_slice()));
        },
        PacketType::HelloReply => {
            println!("Receive HelloReply from Peer: {}\n",)
        },
        PacketType::PublicKeyReply => match,
        PacketType::RootReply =>,
        PacketType::Datum =>,
        PacketType::NatTraversalReply =>,
        _=> return,
    }

}
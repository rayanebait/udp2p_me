use std::os::unix::net::SocketAddr;

use crate::congestion_handler::*;
use crate::packet::*;
use crate::peer_data::PeerData;

pub enum PacketState {
    New,
    Received
}

pub fn handle_packet(packet: Packet, socket_addr: SocketAddr, pending: &mut PendingResponseIds){
    match pending.search_id(&packet){
        Ok(peer) => handle_response_packet(packet, socket_addr, pending),
        Err(e)=> handle_request_packet(packet,  socket_addr, pending),
    }
}

fn handle_request_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: &mut PendingResponseIds){

}

fn handle_response_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: &mut PendingResponseIds){
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
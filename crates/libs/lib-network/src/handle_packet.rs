use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError, Condvar};

use futures::future::pending;

use crate::congestion_handler::*;
use crate::packet::*;
use crate::action::*;
// use crate::peer_data::PeerData;
pub enum HandlingError{
    InvalidPacketError,
    InvalidHashError,
}

pub async fn handle_packet_task(pending_ids: Arc<Mutex<PendingIds>>,
                          receive_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
                          receive_queue_state: Arc<QueueState>,
                          action_queue: Arc<Mutex<Queue<Action>>>,
                          action_queue_state: Arc<QueueState>){

        tokio::spawn(async move {
            loop {
                let action_or_error = 
                    match Queue::lock_and_pop(Arc::clone(&receive_queue)){
                        Some((packet, sock_addr))=> 
                            /*receive queue is not empty get a packet and handle it*/
                            /*verif packet? */
                            handle_packet(packet, sock_addr,
                                 Arc::clone(&pending_ids)
                                 /*return the action required */
                            ),
                        None=>{
                            /*
                                receive queue is empty wait for the activity of 
                                the receive queue
                            */
                            println!("handle packet waits");
                            QueueState::set_empty_queue(Arc::clone(&receive_queue_state));
                            receive_queue_state.wait();
                            continue
                        }
                    };

                match action_or_error {
                    Ok(action)=> {
                            /* we have an action, push it to the queue*/
                            println!("push action (handle packet)");
                            Queue::lock_and_push(Arc::clone(&action_queue), action);
                            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                            continue
                        }
                    Err(HandlingError::InvalidPacketError)=>todo!(),
                    _ => panic!("Shouldn't happen"),
                };
            }
        }).await;
}

pub fn handle_packet(packet: Packet, socket_addr: SocketAddr,
                        pending_ids: Arc<Mutex<PendingIds>>)->Result<Action, HandlingError>{
    
    let id_exists = {
        /*get mutex to check and pop id if it is a response */
        let mut pending_ids_guard = 
            match pending_ids.lock() {
                Ok(guard) => guard,
                /*If Mutex is poisoned stop every thread, something is wrong */
                Err(poison_error)=> panic!("Poisoned Ids Mutex"),
            };

            /*Check if id exists */
            match pending_ids_guard.search_id(&packet){
                Ok(sock_addr) => {
                    /*if id exists, pop the packet before handling it. */
                    /*We choose to not handle the collisions. */
                    pending_ids_guard.pop_packet_id(packet.get_id());

                    /*Now check if the address it was sent to is
                     the same as the address it was received from */
                    let addr_matches_id = {
                        let addr_match_id;

                        if sock_addr != socket_addr {
                            addr_match_id = Err(CongestionHandlerError::AddrAndIdDontMatchError);
                        } else {
                            addr_match_id= Ok(sock_addr);
                        }
                        addr_match_id
                    };
                    addr_matches_id
                },
                Err(CongestionHandlerError::NoPacketWithIdError) =>
                             Err(CongestionHandlerError::NoPacketWithIdError),
                _ => panic!("Shouldn't happen, handle packet"),
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
        Err(_) => panic!("Shouldn't happen, handle packet "),
    }

}

/*Server */
fn handle_request_packet(packet: Packet, socket_addr: SocketAddr,
                                    pending_ids: Arc<Mutex<PendingIds>>
                                //should add self_info with public key root, etc..
                                )
                                            ->Result<Action, HandlingError>{
    let id = packet.get_id();
    match packet.get_packet_type() {
        PacketType::NoOp =>{
            println!("Received NoOp from peer at {}\n", socket_addr);
            Ok(Action::ProcessNoOp(socket_addr))
        },
        PacketType::Error => {
            println!("Received error from peer at {}\n", socket_addr);
            Ok(Action::ProcessError(*id, packet.get_body().to_owned(), socket_addr))
        },
        PacketType::Hello =>{
            println!("Received hello from peer at {}\n", &socket_addr);
            /*change to process hello reply */
            {
                let mut extensions : [u8;4] = [0;4];
                extensions.copy_from_slice(&packet.get_body().as_slice()[0..4]);
                let extensions = match extensions[3] {
                    /*No extensions */
                    0 => None,
                    _=> Some(extensions),
                };
                let name = packet.get_body().as_slice()[4..*packet.get_body_length()].to_vec();
                /*id is transmitted to be reused in a send hello reply */
                Ok(Action::ProcessHello(*id, extensions, name,
                                    socket_addr))
            }
        }
        PacketType::PublicKey=> {
            println!("Received PublicKey from peer at {}\n", socket_addr);
            Ok(Action::ProcessPublicKey(*id,
                match packet.get_body_length(){
                    /*Peer doesn't implement signatures */
                0 => None,
                    /*Peer implements signatures */
                _ => Some({
                        let mut public_key: [u8;64] = [0;64];
                        public_key.copy_from_slice(&packet.get_body().as_slice()[0..64]);
                        public_key
                    }),
                }, socket_addr))
        },
        PacketType::Root => Ok(Action::ProcessRoot(*id,
                match packet.get_body_length(){
                    /*Peer doesn't implement signatures */
                0 => None,
                    /*Peer implements signatures */
                _ => Some({
                        let mut root: [u8;32] = [0;32];
                        root.copy_from_slice(&packet.get_body().as_slice()[0..32]);
                        root
                    }),
                }, socket_addr)),
        /*Exports should have its own send/receive queue?*/
        PacketType::GetDatum => Ok(Action::ProcessGetDatum(*id,
            {
                let mut hash : [u8;32] = [0;32];
                hash.copy_from_slice( &packet.get_body().as_slice()[0..32]);
                hash
            }
            , socket_addr)),
        PacketType::NatTraversal => Ok(Action::A),
        /*Invalid packet, should send error*/
        _ => Err(HandlingError::InvalidPacketError),
    }
}

/*Client */
fn handle_response_packet(packet: Packet, socket_addr: SocketAddr,
                                     pending: Arc<Mutex<PendingIds>>)
                                        ->Result<Action, HandlingError>{

    match packet.get_packet_type(){
        PacketType::ErrorReply => {
                let error_message = packet.get_body().to_owned();
                println!("Received ErrorReply with body: {}\n",
                        String::from_utf8_lossy(error_message.as_slice()));
                Ok(Action::ProcessErrorReply(error_message, socket_addr))
            },
        PacketType::HelloReply => {
                println!("Received HelloReply from peer at {}\n", socket_addr);
                let mut extensions : [u8;4] = [0;4];
                extensions.copy_from_slice(&packet.get_body().as_slice()[0..4]);
                let extensions = match extensions[3] {
                    /*No extensions */
                    0 => None,
                    _=> Some(extensions),
                };
                let name = packet.get_body().as_slice()[4..*packet.get_body_length()].to_vec();
                Ok(Action::ProcessHelloReply(extensions, name,
                                    socket_addr))
            },
        PacketType::PublicKeyReply => {
                println!("Received PublicKeyReply from peer at {}\n", socket_addr);
                Ok(Action::ProcessPublicKeyReply(
                    match packet.get_body_length(){
                        /*Peer doesn't implement signatures */
                    0 => None,
                        /*Peer implements signatures */
                    _ => Some({
                            let mut public_key: [u8;64] = [0;64];
                            public_key.copy_from_slice(&packet.get_body().as_slice()[0..64]);
                            public_key
                        }),
                    }, socket_addr))
            },
        PacketType::RootReply =>{
                println!("Receive RootReply from peer at {}\n", socket_addr);
                Ok(Action::ProcessRootReply(
                    match packet.get_body_length(){
                        /*Peer doesn't implement signatures */
                    0 => None,
                        /*Peer implements signatures */
                    _ => Some({
                            let mut root: [u8;32] = [0;32];
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

use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::str::Bytes;
use std::sync::{Arc, Condvar, Mutex, PoisonError, RwLock};

use futures::future::pending;
use log::{debug, error, info, warn};
use tokio_util::bytes::Buf;

use crate::action::*;
use crate::congestion_handler::*;
use crate::packet::*;
// use crate::peer_data::PeerData;
pub enum HandlingError {
    InvalidPacketError,
    InvalidHashError,
}

pub fn handle_packet_task(
    pending_ids: Arc<Mutex<PendingIds>>,
    receive_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    receive_queue_state: Arc<QueueState>,
    process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    process_queue_readers_state: Arc<QueueState>,
) {
    tokio::spawn(async move {
        loop {
            let action_or_error = match Queue::lock_and_pop(Arc::clone(&receive_queue)) {
                Some((packet, sock_addr)) =>
                /*receive queue is not empty get a packet and handle it*/
                /*verif packet? */
                {
                    handle_packet(
                        packet,
                        sock_addr,
                        Arc::clone(&pending_ids), /*return the action required */
                    )
                }
                None => {
                    /*
                        receive queue is empty wait for the activity of
                        the receive queue
                    */
                    // println!("handle packet waits");
                    QueueState::set_empty_queue(Arc::clone(&receive_queue_state));
                    receive_queue_state.wait();
                    continue;
                }
            };

            match action_or_error {
                Ok(action) => {
                    /* we have an action, push it to the queue*/
                    Queue::write_lock_and_push(Arc::clone(&process_queue), action);
                    QueueState::set_non_empty_queue(Arc::clone(&process_queue_readers_state));
                    QueueState::set_non_empty_queue(Arc::clone(&process_queue_state));
                    continue;
                }
                Err(HandlingError::InvalidPacketError) => return,
                _ => {
                    error!("Should not happen");
                    panic!("Shouldn't happen")
                }
            };
        }
    });
}

pub fn handle_packet(
    packet: Packet,
    socket_addr: SocketAddr,
    pending_ids: Arc<Mutex<PendingIds>>,
) -> Result<Action, HandlingError> {
    let id_exists = PendingIds::id_exists(Arc::clone(&pending_ids), &packet, socket_addr);

    match id_exists {
        /*Packet is a response, handles the case where packet is a nat traversal */
        Ok(sock_addr) => handle_response_packet(packet, socket_addr, pending_ids),
        /*Packet is a request */
        Err(CongestionHandlerError::NoPacketWithIdError) => {
            handle_request_packet(packet, socket_addr, pending_ids)
        }

        /*Id is known but now Peer: discard packet.
        Note: There could be collisions between ids
        but with negligible proba ? */
        Err(CongestionHandlerError::AddrAndIdDontMatchError) => {
            Err(HandlingError::InvalidPacketError)
        }
        Err(e) => {
            error!("{e}");
            panic!("Shouldn't happen, handle packet ")
        }
    }
}

/*Server */
fn handle_request_packet(
    packet: Packet,
    socket_addr: SocketAddr,
    pending_ids: Arc<Mutex<PendingIds>>, //should add self_info with public key root, etc..
) -> Result<Action, HandlingError> {
    let id = packet.get_id();
    let body = packet.get_body();
    match packet.get_packet_type() {
        PacketType::NoOp => Ok(Action::ProcessNoOp(socket_addr)),
        PacketType::Error => Ok(Action::ProcessError(
            *id,
            packet.get_body().to_owned(),
            socket_addr,
        )),
        PacketType::Hello => {
            /*change to process hello reply */
            {
                let mut extensions: [u8; 4] = [0; 4];
                extensions.copy_from_slice(&packet.get_body().as_slice()[0..4]);
                let extensions = match extensions[3] {
                    /*No extensions */
                    0 => None,
                    _ => Some(extensions),
                };
                let name = packet.get_body().as_slice()[4..*packet.get_body_length()].to_vec();
                /*id is transmitted to be reused in a send hello reply */
                Ok(Action::ProcessHello(*id, extensions, name, socket_addr))
            }
        }
        PacketType::PublicKey => {
            Ok(Action::ProcessPublicKey(
                *id,
                match packet.get_body_length() {
                    /*Peer doesn't implement signatures */
                    0 => None,
                    /*Peer implements signatures */
                    _ => Some({
                        let mut public_key: [u8; 64] = [0; 64];
                        public_key.copy_from_slice(&packet.get_body().as_slice()[0..64]);
                        public_key
                    }),
                },
                socket_addr,
            ))
        }
        PacketType::Root => {
            if body.len() < 32 {
                return Ok(Action::SendErrorReply(
                    *id,
                    Some(b"root is too short".to_vec()),
                    socket_addr,
                ));
            } else {
                let mut root = [0u8; 32];
                root.copy_from_slice(&body.as_slice()[0..32]);

                if root == hex::decode(HASH_OF_EMPTY_STRING).unwrap().as_slice() {
                    return Ok(Action::ProcessRoot(*id, None, socket_addr));
                }
                Ok(Action::ProcessRoot(*id, Some(root), socket_addr))
            }
        }
        /*Exports should have its own send/receive queue?*/
        PacketType::GetDatum => Ok(Action::ProcessGetDatum(
            *id,
            {
                let mut hash: [u8; 32] = [0; 32];
                hash.copy_from_slice(&packet.get_body().as_slice()[0..32]);
                hash
            },
            socket_addr,
        )),
        /*Invalid packet, should send error*/
        _ => Err(HandlingError::InvalidPacketError),
    }
}

/*Client */
fn handle_response_packet(
    packet: Packet,
    socket_addr: SocketAddr,
    pending: Arc<Mutex<PendingIds>>,
) -> Result<Action, HandlingError> {
    let body = packet.get_body();
    match packet.get_packet_type() {
        PacketType::ErrorReply => {
            let error_message = packet.get_body().to_owned();
            Ok(Action::ProcessErrorReply(error_message, socket_addr))
        }
        PacketType::HelloReply => {
            let mut extensions: [u8; 4] = [0; 4];
            extensions.copy_from_slice(&packet.get_body().as_slice()[0..4]);
            let extensions = match extensions[3] {
                /*No extensions */
                0 => None,
                _ => Some(extensions),
            };
            let name = packet.get_body().as_slice()[4..*packet.get_body_length()].to_vec();
            Ok(Action::ProcessHelloReply(extensions, name, socket_addr))
        }
        PacketType::PublicKeyReply => {
            Ok(Action::ProcessPublicKeyReply(
                match packet.get_body_length() {
                    /*Peer doesn't implement signatures */
                    0 => None,
                    /*Peer implements signatures */
                    _ => Some({
                        let mut public_key: [u8; 64] = [0; 64];
                        public_key.copy_from_slice(&packet.get_body().as_slice()[0..64]);
                        public_key
                    }),
                },
                socket_addr,
            ))
        }
        PacketType::RootReply => {
            if body.len() < 32 {
                return Ok(Action::SendError(
                    b"root is too short".to_vec(),
                    socket_addr,
                ));
            } else {
                let mut root = [0u8; 32];
                root.copy_from_slice(&body.as_slice()[0..32]);

                if root == hex::decode(HASH_OF_EMPTY_STRING).unwrap().as_slice() {
                    return Ok(Action::ProcessRootReply(None, socket_addr));
                }
                Ok(Action::ProcessRootReply(Some(root), socket_addr))
            }
        }
        PacketType::Datum => match packet.valid_hash() {
            true => Ok(Action::ProcessDatum(
                packet.get_body().to_owned(),
                socket_addr,
            )),
            false => Err(HandlingError::InvalidHashError),
        },
        PacketType::NatTraversal => {
            if socket_addr == "81.194.27.155:8443".parse().unwrap() {
                debug!("Received NatTraversal from server\n");
                return Ok(Action::ProcessNatTraversal(
                    packet.get_body().to_owned(),
                    socket_addr,
                ));
            } else {
                return Err(HandlingError::InvalidPacketError);
            };
        }
        _ => return Err(HandlingError::InvalidPacketError),
    }
}

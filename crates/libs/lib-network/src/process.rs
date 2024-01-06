use core::panic;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, RwLock};

use log::{debug, error, info};
use tokio_util::sync::CancellationToken;

use crate::action::Action;

use crate::peer::*;

use crate::congestion_handler::*;

/*Chaque sous task du CLI lit passivement la process queue
et push des paquets dans l'action queue en conséquence ?*/
pub fn process_task(
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    active_peers: Arc<Mutex<ActivePeers>>,
    my_data: Arc<Peer>,
    // to_export: Arc<HashMap<[u8;32], &MktFsNode>>,
    //tree: ?
    //For each peer build a physical tree and fill it ?
    //Option<mk_fs>, when receiving a getdatum, check if None or some
    //if None simply reply nodatum, if Some: read hash and
    //send datum.
    //hash_map:?
    //self_data:?
    cancel: CancellationToken
) {
    //Should pop only if too full ? For subtasks to have time to read
    tokio::spawn(async move {
        loop {
            if cancel.is_cancelled(){
                break
            }
            match Queue::write_lock_and_get(Arc::clone(&process_queue)) {
                Some(action) => {
                    /*action queue is not empty get an action and handle it*/
                    // println!("process: {:?}\n", action);
                    process_action(
                        action.clone(),
                        Arc::clone(&action_queue),
                        Arc::clone(&action_queue_state),
                        Arc::clone(&active_peers),
                        Arc::clone(&my_data),
                    );
                    debug!("{:?}", action)
                    /*return the action required */
                }
                None => {
                    /*
                    action queue is empty wait for the activity of
                    the receive queue
                    */

                    // println!("process wait");
                    QueueState::set_empty_queue(Arc::clone(&process_queue_state));
                    process_queue_state.wait();
                    continue;
                }
            };
        }
    });
}

pub fn process_action(
    action: Action,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    active_peers: Arc<Mutex<ActivePeers>>,
    my_data: Arc<Peer>, //Pour store public key et root ->
                        // to_export: Arc<HashMap<[u8;32], &MktFsNode>>       //hashmap: sockaddr vers peer
                        //peer.set_public_key...
                        //peer.set_root..
                        //active_peers: Arc<ActivePeers>
                        //struct ActivePeers {
                        //    peers: Vec<Peer>,
                        //    map: Hashmap<SocketAddr, Peer>
                        //}
) {
    let my_name = my_data.get_name().unwrap().as_bytes().to_vec();
    match action {
        Action::ProcessNoOp(_sock_addr) => {
            /*DONE */
            return;
        }
        Action::ProcessHello(id, extensions, name, sock_addr) => {
            /*DONE */
            /*Send Hello reply then process the hello. Fails when the name is invalid utf8.*/
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendHelloReply(id, my_data.get_extensions(), my_name, sock_addr),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            /*Add peer and set peer timer (30s) */
            let _ = ActivePeers::set_peer_extensions_and_name(
                active_peers,
                sock_addr,
                extensions,
                name,
            );
            return;
        }
        Action::ProcessError(_id, error_msg, sock_addr) => {
            /*DONE */
            info!(
                "Received Error with body: {}\n from {}\n",
                String::from_utf8(error_msg).unwrap(),
                sock_addr
            );
            return;
        }
        Action::ProcessPublicKey(id, public_key, sock_addr) => {
            /*DONE */
            match ActivePeers::set_peer_public_key(active_peers, sock_addr, public_key) {
                Ok(()) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendPublicKeyReply(id, my_data.get_public_key(), sock_addr),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::PeerTimedOut) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendErrorReply(
                            id,
                            Some(
                                b"Connection timedout,
                                             proceed to handshake again.\n"
                                    .to_vec(),
                            ),
                            sock_addr,
                        ),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer) => (),
                Err(e) => {
                    error!("{e}");
                    panic!("Unkown error in process_action {}\n", e)
                }
            }
            return;
        }
        Action::ProcessRoot(id, root, sock_addr) => {
            /*DONE */
            match ActivePeers::set_peer_root(active_peers, sock_addr, root) {
                Ok(()) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendRootReply(id, root, sock_addr),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::PeerTimedOut) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendErrorReply(
                            id,
                            Some(
                                b"Connection timedout,
                                             proceed to handshake again.\n"
                                    .to_vec(),
                            ),
                            sock_addr,
                        ),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer) => (),
                Err(e) => {
                    error!("{e}");
                    panic!("Unkown error in process_action {}\n", e)
                }
            }
            return;
        }
        Action::ProcessGetDatum(id, _hash, sock_addr) => {
            Queue::lock_and_push(action_queue.clone(), Action::SendNoDatum(id, sock_addr));
            QueueState::set_non_empty_queue(action_queue_state.clone());
            /*to do */
            return;
        }
        // Action::ProcessNatTraversalRequest(id, body, sock_addr) => {
        //     /*Shouldn't receive ? */
        //     return;
        // }
        Action::ProcessHelloReply(extensions, name, sock_addr) => {
            /*DONE */
            let _ = ActivePeers::set_peer_extensions_and_name(
                active_peers,
                sock_addr,
                extensions,
                name,
            );
            return;
        }
        Action::ProcessErrorReply(err_msg_reply, sock_addr) => {
            /*DONE */
            info!(
                "Received ErrorReply with body:{}\n from {}\n",
                String::from_utf8_lossy(&err_msg_reply),
                sock_addr
            );
            return;
        }
        Action::ProcessPublicKeyReply(public_key, sock_addr) => {
            /*DONE */
            match ActivePeers::set_peer_public_key(active_peers, sock_addr, public_key) {
                Ok(()) => {}
                Err(PeerError::PeerTimedOut) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendError(
                            b"Connection timedout,
                                             proceed to handshake again.\n"
                                .to_vec(),
                            sock_addr,
                        ),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer) => (),
                Err(e) => {
                    error!("{e}");
                    panic!("Unkown error in process_action {}\n", e)
                }
            }
            return;
        }
        Action::ProcessRootReply(root, sock_addr) => {
            /*DONE */
            match ActivePeers::set_peer_root(active_peers, sock_addr, root) {
                Ok(()) => {}
                Err(PeerError::PeerTimedOut) => {
                    Queue::lock_and_push(
                        Arc::clone(&action_queue),
                        Action::SendError(
                            b"Connection timedout,
                                             proceed to handshake again.\n"
                                .to_vec(),
                            sock_addr,
                        ),
                    );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer) => (),
                Err(e) => {
                    error!("{e}");
                    panic!("Unkown error in process_action {}\n", e)
                }
            }
            return;
        }
        Action::ProcessDatum(_datum, _sock_addr) => {
            /*DONE? */
            return;
        }
        Action::ProcessNoDatum(_sock_addr) => {
            /*DONE? */
            return;
        }
        Action::ProcessNatTraversal(body, _sock_addr) => {
            let addr: SocketAddr;
            if body.len() == 6 {
                addr = SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(body[0], body[1], body[2], body[3])),
                    (body[4] as u16) * 256 + (body[5] as u16),
                );
            } else if body.len() == 18 {
                addr = SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(
                        (body[0] as u16) * 256 + (body[1] as u16),
                        (body[2] as u16) * 256 + (body[3] as u16),
                        (body[4] as u16) * 256 + (body[5] as u16),
                        (body[6] as u16) * 256 + (body[7] as u16),
                        (body[8] as u16) * 256 + (body[9] as u16),
                        (body[10] as u16) * 256 + (body[11] as u16),
                        (body[12] as u16) * 256 + (body[13] as u16),
                        (body[14] as u16) * 256 + (body[15] as u16),
                    )),
                    (body[16] as u16) * 256 + (body[17] as u16),
                );
            } else {
                return;
            }

            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendHello(my_data.get_extensions(), my_name, addr),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            return;
        }
        _ => {
            Queue::lock_and_push(Arc::clone(&action_queue), action);
        }
    }
}

use core::panic;
use std::collections::HashMap;
use std::error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::Future;
use tokio::join;
use tokio::select;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{sleep, Sleep};

use crate::action::Action;
use crate::packet::PacketBuilder;
use crate::peer::peer::*;
use crate::store::{get_child_to_parent_hashmap, get_parent_to_child_hashmap};
use crate::{action, congestion_handler::*};

use lib_file::mk_fs::{self, MktFsNode};

/*Chaque sous task du CLI lit passivement la process queue
et push des paquets dans l'action queue en conséquence ?*/
pub async fn process_task(
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
) {
    //Should pop only if too full ? For subtasks to have time to read
    tokio::spawn(async move {
        loop {
            match Queue::write_lock_and_get(Arc::clone(&process_queue)) {
                Some(action) => {
                    /*action queue is not empty get an action and handle it*/
                    // println!("process: {:?}\n", action);
                    process_action(
                        action,
                        Arc::clone(&action_queue),
                        Arc::clone(&action_queue_state),
                        Arc::clone(&active_peers),
                        Arc::clone(&my_data),
                        // Arc::clone(&to_export)
                    );
                    /*return the action required */
                }
                None => {
                    /*
                    action queue is empty wait for the activity of
                    the receive queue
                    */

                    println!("process wait");
                    QueueState::set_empty_queue(Arc::clone(&process_queue_state));
                    process_queue_state.wait();
                    continue;
                }
            };
        }
    })
    .await;
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
    match action {
        Action::ProcessNoOp(sock_addr) => {
            /*DONE */
            return;
        }
        Action::ProcessHello(id, extensions, name, sock_addr) => {
            /*DONE */
            /*Send Hello reply then process the hello. Never fails. */
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendHelloReply(
                    id,
                    my_data.get_extensions().unwrap(),
                    my_data.get_name().unwrap(),
                    sock_addr,
                ),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            /*Add peer and set peer timer (180s) */
            ActivePeers::set_peer_extensions_and_name(active_peers, sock_addr, extensions, name);
            return;
        }
        Action::ProcessError(id, error_msg, sock_addr) => {
            /*DONE */
            println!(
                "Received Error with body: {}\n from {}\n",
                // String::from_utf8_lossy(&error_msg),
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
                Err(PeerError::ResponseTimeout) => {
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
                _ => panic!("Unkown error in process_action\n"),
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
                Err(PeerError::ResponseTimeout) => {
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
                _ => panic!("Unkown error in process_action\n"),
            }
            return;
        }
        Action::ProcessGetDatum(id, hash, sock_addr) => {
            return;
        }
        Action::ProcessHelloReply(extensions, name, sock_addr) => {
            /*DONE */
            ActivePeers::set_peer_extensions_and_name(active_peers, sock_addr, extensions, name);
            return;
        }
        Action::ProcessErrorReply(err_msg_reply, sock_addr) => {
            /*DONE */
            println!(
                "Received ErrorReply packet with message:{}\n From {}\n",
                String::from_utf8_lossy(&err_msg_reply),
                sock_addr
            );
            return;
        }
        Action::ProcessPublicKeyReply(public_key, sock_addr) => {
            /*DONE */
            ActivePeers::set_peer_public_key(active_peers, sock_addr, public_key);
            return;
        }
        Action::ProcessRootReply(root, sock_addr) => {
            /*DONE */
            ActivePeers::set_peer_root(active_peers, sock_addr, root);
            return;
        }
        Action::ProcessDatum(datum, sock_addr) => {
            /*datum is valid at this point (verified in handle packet*/
            /*or not ? */
            return;
        }
        _ => println!("Shouldn't happen: {:?}", action),
    }
}

/*Sends Hello then peeks the process queue to check for a helloreply, if no
helloreply in 3 seconds, sends hello again, repeats 3 times.

 */
pub async fn register(
    peek_process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    process_queue_readers_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    my_data: Arc<Peer>,
) {
    let server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();

    for attempt in 0..10 {
        Queue::lock_and_push(
            Arc::clone(&action_queue),
            Action::SendHello(None, my_data.get_name().unwrap(), server_sock_addr),
        );
        QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
        println!("attempt : {}", attempt);

        match peek_until_hello_reply_from(
            Arc::clone(&peek_process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            server_sock_addr,
        )
        .await
        {
            Ok(_) => {
                keep_alive_to_peer(
                    Arc::clone(&action_queue),
                    Arc::clone(&action_queue_state),
                    *&server_sock_addr,
                );
                break;
            }
            Err(_) => continue,
        }
    }
    println!("here")
}

pub fn keep_alive_to_peer(
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    sock_addr: SocketAddr,
) {
    tokio::spawn(async move {
        loop {
            std::thread::sleep(Duration::from_secs(5));
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendHello(None, vec![97, 110, 105, 116], sock_addr),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
        }
    });
}

pub async fn peek_until_hello_reply_from(
    peek_process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    process_queue_readers_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    sock_addr: SocketAddr,
) -> Result<Action, PeerError> {
    /*Repeatedly peeks process queue to check  */
    let task_handle = tokio::spawn(async move {
        loop {
            /*Wait notify all from receive task */
            let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                Some(front) => front,
                None => {
                    process_queue_readers_state.wait();
                    continue;
                }
            };
            match front {
                Action::ProcessHelloReply(.., addr) => {
                    if addr == sock_addr {
                        break Ok::<Action, PeerError>(front);
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
        }
    });
    let abort_task_handle = task_handle.abort_handle();

    let timeout_handle = tokio::spawn(async move {
        let timeout = sleep(Duration::from_secs(3)).await;
        if abort_task_handle.is_finished() {
            /*Never happens */
            return Err::<Action, PeerError>(PeerError::ResponseTimeout);
        } else {
            abort_task_handle.abort();
            return Err::<Action, PeerError>(PeerError::ResponseTimeout);
        }
    });
    /* Return when either response time out or received a packet
    corresponding */
    select! {
        timed_out = timeout_handle => {
            match timed_out {
                Ok(result)=> match result {
                    Err(PeerError::ResponseTimeout)=> return Err(PeerError::ResponseTimeout),
                    _=>panic!("Shouldn't happen"),
                },
                Err(_)=> panic!("time out task panicked"),
            }
        }
        action = task_handle => {
            match action {
                Ok(result)=> match result {
                    Ok(action)=> return Ok(action),
                    _=>panic!("Shouldn't happen"),
                },
                Err(_)=> panic!("time out task panicked"),
            }
        }
    };
}

pub async fn fetch_subtree_from(
    peek_process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    active_peers: Arc<Mutex<ActivePeers>>,
    hash: [u8;32],
    sock_addr: SocketAddr,
    child_to_parent_hashmap: Arc<Mutex<HashMap<[u8; 32], Vec<[u8; 32]>>>>,
    parent_to_child_hashmap: Arc<Mutex<HashMap<[u8;32], [u8;32]>>>,
){
    tokio::spawn(async move {
        loop {
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendGetDatumWithHash(
                    hash.clone(),
                    *&sock_addr,
                ),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            match peek_until_datum_with_hash_from(
                Arc::clone(&peek_process_queue),
                Arc::clone(&process_queue_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                *&hash,
                *&sock_addr,
            )
            .await
            {
                Ok(datum_action) => {
                    get_parent_to_child_hashmap(&datum_action, Arc::clone(&child_to_parent_hashmap));
                    get_child_to_parent_hashmap(&datum_action, Arc::clone(&parent_to_child_hashmap));
                }
                Err(PeerError::ResponseTimeout) => break Err::<Action,PeerError>(PeerError::ResponseTimeout),
                _ => todo!(),
            };


            // for child in childs
        }
    })
    .await;
}

/*Given a hash it downloads all files under it  */
pub async fn download(
    peek_process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    active_peers: Arc<Mutex<ActivePeers>>,
    hash: [u8;32],
    sock_addr: SocketAddr,
) {
    /*Peer has to have been handshaked before */
    /*Constructs the whole tree structure without storing it */
    // let hash_trees_or_error = fetch_subtree_from(
    //     Arc::clone(&peek_process_queue),
    //     Arc::clone(&process_queue_state),
    //     Arc::clone(&action_queue),
    //     Arc::clone(&action_queue_state),
    //     Arc::clone(&active_peers),
    //     hash,
    //     sock_addr,
    // )
    // .await;

    // let (parent_to_child_map, child_to_parent_map) = match hash_trees_or_error {
    //     Ok(hash_trees)=> hash_trees,
    //     Err(PeerError::ResponseTimeout)=> todo!(),
    //     _=>todo!(),
    // };


}

pub async fn peek_until_datum_with_hash_from(
    peek_process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    hash: [u8;32],
    sock_addr: SocketAddr,
) -> Result<Action, PeerError> {
    /*Curr not working because stuck in waiting process queue
    try with two tasks or two futures */
    let task_handle = tokio::spawn(async move {
        loop {
            /*Wait notify all from receive task */
            let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                Some(front) => front,
                None => {
                    process_queue_state.wait();
                    continue;
                }
            };
            match front {
                Action::ProcessDatum(datum, addr) => {
                    let mut datum_hash = [0u8;32];
                    datum_hash.copy_from_slice(&datum.as_slice()[0..32]);
                    if (addr == sock_addr) && (datum_hash == hash){
                        break Ok::<Action, PeerError>(Action::ProcessDatum(datum, addr));
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
        }
    });
    let abort_task_handle = task_handle.abort_handle();

    let timeout_handle = tokio::spawn(async move {
        let timeout = sleep(Duration::from_secs(3)).await;
        if abort_task_handle.is_finished() {
            /*Never happens */
            return Err::<Action, PeerError>(PeerError::ResponseTimeout);
        } else {
            abort_task_handle.abort();
            return Err::<Action, PeerError>(PeerError::ResponseTimeout);
        }
    });
    /* Return when either response time out or received a packet
    corresponding */
    select! {
        timed_out = timeout_handle => {
            match timed_out {
                Ok(result)=> match result {
                    Err(PeerError::ResponseTimeout)=> return Err(PeerError::ResponseTimeout),
                    _=>panic!("Shouldn't happen"),
                },
                Err(_)=> panic!("time out task panicked"),
            }
        }
        action = task_handle => {
            match action {
                Ok(result)=> match result {
                    Ok(action)=> return Ok(action),
                    _=>panic!("Shouldn't happen"),
                },
                Err(_)=> panic!("time out task panicked"),
            }
        }
    };
}

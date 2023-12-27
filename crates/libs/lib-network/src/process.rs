use core::panic;
use std::collections::HashMap;
use std::error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex,RwLock};
use std::time::Duration;

use crate::{congestion_handler::*, action};
use crate::action::Action;
use crate::packet::PacketBuilder;
use crate::peer::peer::*;

/*Chaque sous task du CLI lit passivement la process queue 
et push des paquets dans l'action queue en conséquence ?*/
pub async fn process_task(action_queue: Arc<Mutex<Queue<Action>>>,
                          action_queue_state: Arc<QueueState>,
                          process_queue: Arc<RwLock<Queue<Action>>>,
                          process_queue_state: Arc<QueueState>,
                          active_peers: Arc<Mutex<ActivePeers>>,
                          my_data: Arc<Peer>,
                          //tree: ?
                          //For each peer build a physical tree and fill it ?
                          //Option<mk_fs>, when receiving a getdatum, check if None or some
                          //if None simply reply nodatum, if Some: read hash and
                          //send datum. 
                          //hash_map:?
                          //self_data:?
                        ){
    tokio::spawn(async move {
        loop {
            match Queue::write_lock_and_pop(Arc::clone(&process_queue)){
                Some(action)=> {
                    /*action queue is not empty get an action and handle it*/
                    // println!("process: {:?}\n", action);
                    process_action(action,
                      Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                      Arc::clone(&active_peers),
                                    Arc::clone(&my_data)
                     );
                /*return the action required */

                },
                None=>{
                    /*
                        action queue is empty wait for the activity of 
                        the receive queue
                        */
                    
                    println!("process wait");
                    QueueState::set_empty_queue(Arc::clone(&process_queue_state));
                    process_queue_state.wait();
                    continue
                }
            };
        }
    }).await;
}

pub fn process_action(action : Action,
                      action_queue: Arc<Mutex<Queue<Action>>>,
                      action_queue_state: Arc<QueueState>,
                      active_peers: Arc<Mutex<ActivePeers>>,
                      my_data: Arc<Peer>
                      //Pour store public key et root -> 
                      //hashmap: sockaddr vers peer
                      //peer.set_public_key...
                      //peer.set_root..
                      //active_peers: Arc<ActivePeers>
                      //struct ActivePeers {
                      //    peers: Vec<Peer>,
                      //    map: Hashmap<SocketAddr, Peer>
                      //}
                      ){
    match action {
        Action::ProcessNoOp(sock_addr) =>{
            /*DONE */
            println!("Received NoOp packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessHello(id, extensions, name, sock_addr) =>{
            /*DONE */
            /*Send Hello reply then process the hello. Never fails. */
            Queue::lock_and_push(Arc::clone(&action_queue),
                                    Action::SendHelloReply(id,
                                          my_data.get_extensions().unwrap(),
                                          my_data.get_name().unwrap(),
                                          sock_addr
                                        )
                    );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            /*Add peer and set peer timer (180s) */
            ActivePeers::set_peer_extensions_and_name(active_peers, sock_addr, extensions, name);
            return;
        }, 
        Action::ProcessError(id, error_msg, sock_addr) =>{
            /*DONE */
            println!("Received Error with body: {}\n from {}\n",
                                String::from_utf8_lossy(&error_msg),
                                sock_addr);
            return;
        }, 
        Action::ProcessPublicKey(id, public_key, sock_addr) =>{
            /*DONE */
            match ActivePeers::set_peer_public_key(active_peers, sock_addr, public_key){
                Ok(())=>{
                    Queue::lock_and_push(Arc::clone(&action_queue),
                                        Action::SendPublicKeyReply(id,
                                            public_key,
                                            sock_addr)
                        );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::ResponseTimeout)=>{
                    Queue::lock_and_push(Arc::clone(&action_queue),
                                        Action::SendErrorReply(id,
                                            Some(b"Connection timedout,
                                             proceed to handshake again.\n".to_vec()),
                                            sock_addr)
                        );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer)=>(),
                _=> panic!("Unkown error in process_action\n"),
            }
            return;
        }, 
        Action::ProcessRoot(id, root, sock_addr) =>{
            /*DONE */
            match ActivePeers::set_peer_root(active_peers, sock_addr, root){
                Ok(())=>{
                    Queue::lock_and_push(Arc::clone(&action_queue),
                                        Action::SendRootReply(id,
                                            root,
                                            sock_addr)
                        );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::ResponseTimeout)=>{
                    Queue::lock_and_push(Arc::clone(&action_queue),
                                        Action::SendErrorReply(id,
                                            Some(b"Connection timedout,
                                             proceed to handshake again.\n".to_vec()),
                                            sock_addr)
                        );
                    QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                }
                Err(PeerError::UnknownPeer)=>(),
                _=> panic!("Unkown error in process_action\n"),
            }
            return;
        }, 
        Action::ProcessGetDatum(id, hash, sock_addr) =>{
            return;
        }, 
        Action::ProcessHelloReply(extensions, name,sock_addr) =>{
            /*DONE */
            ActivePeers::set_peer_extensions_and_name(active_peers, sock_addr, extensions, name);
            return;
        }, 
        Action::ProcessErrorReply(err_msg_reply, sock_addr) =>{
            /*DONE */
            println!("Received ErrorReply packet with message:{}\n From {}\n",
                                 String::from_utf8_lossy(&err_msg_reply), sock_addr);
            return;
        }, 
        Action::ProcessPublicKeyReply(public_key, sock_addr) =>{
            /*DONE */
            ActivePeers::set_peer_public_key(active_peers, sock_addr, public_key);
            return;
        }, 
        Action::ProcessRootReply(root, sock_addr) =>{
            /*DONE */
            ActivePeers::set_peer_root(active_peers, sock_addr, root);
            return;
        }, 
        Action::ProcessDatum(datum, sock_addr) =>{
            /*datum is valid at this point (verified in handle packet*/
            /*or not ? */
            return;
        }, 
        _=>println!("Shouldn't happen: {:?}", action),
    }
}

    /*Queue to peek the main queues */
    pub async fn register(peek_process_queue: Arc<RwLock<Queue<Action>>>,
                          process_queue_state: Arc<QueueState>,
                          action_queue: Arc<Mutex<Queue<Action>>>,
                          action_queue_state: Arc<QueueState>,
                          my_data: Arc<Peer>
                         )
    {
        tokio::spawn(async move {
                /*Wait notify all from receive task */

        }).await;
    }


    pub async fn peek_until_hello_from(
                            peek_process_queue: Arc<RwLock<Queue<Action>>>,
                            process_queue_state: Arc<QueueState>,
                            action_queue: Arc<Mutex<Queue<Action>>>,
                            action_queue_state: Arc<QueueState>,
                            sock_addr: SocketAddr
                        )-> Result<Action, tokio::task::JoinError>
    {
        tokio::spawn(async move {
            loop {
                /*Wait notify all from receive task */
                let front = match
                     Queue::read_lock_and_peek(
                        Arc::clone(&peek_process_queue)
                        ){
                            Some(front) => front,
                            None => {
                                process_queue_state.wait();
                                continue
                            }
                        };
                match front {
                    Action::ProcessHelloReply(..,
                                 addr)=> 
                                    if addr == sock_addr{
                                        break front
                                    } else {
                                        continue
                                    },
                    _ => continue,
                }
            }
        }).await
    }

    pub async fn peek_until_public_key_from(
                            peek_process_queue: Arc<RwLock<Queue<Action>>>,
                            process_queue_state: Arc<QueueState>,
                            action_queue: Arc<Mutex<Queue<Action>>>,
                            action_queue_state: Arc<QueueState>,
                            sock_addr: SocketAddr
                        )-> Result<Result<Action, PeerError>, tokio::task::JoinError>
    {
        tokio::spawn(async move {
            let timeout = tokio::time::sleep(Duration::from_secs(3));
            loop {
                if timeout.is_elapsed() {
                    break Err(PeerError::ResponseTimeout)
                }
                /*Wait notify all from receive task */
                let front = match
                     Queue::read_lock_and_peek(
                        Arc::clone(&peek_process_queue)
                        ){
                            Some(front) => front,
                            None => {
                                process_queue_state.wait();
                                continue
                            }
                        };
                match front {
                    Action::ProcessHelloReply(..,
                                 addr)=> 
                                    if addr == sock_addr{
                                        break Ok(front)
                                    } else {
                                        continue
                                    },
                    _ => continue,
                }
            }
        }).await
    }
    pub async fn peek_until_root_reply_from(
                            peek_process_queue: Arc<RwLock<Queue<Action>>>,
                            process_queue_state: Arc<QueueState>,
                            action_queue: Arc<Mutex<Queue<Action>>>,
                            action_queue_state: Arc<QueueState>,
                            sock_addr: SocketAddr
                        )-> Result<Result<Action, PeerError>, tokio::task::JoinError>
    {
        tokio::spawn(async move {
            loop {
                /*Wait notify all from receive task */
                let front = match
                     Queue::read_lock_and_peek(
                        Arc::clone(&peek_process_queue)
                        ){
                            Some(front) => front,
                            None => {
                                process_queue_state.wait();
                                continue
                            }
                        };
                match front {
                    Action::ProcessHelloReply(..,
                                 addr)=> 
                                    if addr == sock_addr{
                                        break Ok(front)
                                    } else {
                                        continue
                                    },
                    _ => continue,
                }
            }
        }).await
    }
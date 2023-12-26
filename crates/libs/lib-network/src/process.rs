use std::sync::{Arc, Mutex,RwLock};

use crate::congestion_handler::*;
use crate::action::Action;
use crate::packet::PacketBuilder;
use crate::peer::peer::*;

pub async fn client_task(){

}

pub async fn process_task(action_queue: Arc<Mutex<Queue<Action>>>,
                          action_queue_state: Arc<QueueState>,
                          process_queue: Arc<RwLock<Queue<Action>>>,
                          process_queue_state: Arc<QueueState>,
                          active_peers: Arc<Mutex<ActivePeers>>,
                          my_data: Arc<Peer>,
                        //   subtasks: Vec<Arc<RwLock<(Queue<Action>, QueueState)>>>
                          //tree: ?
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
                      //Hashmap: sockaddr vers peer
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
            println!("Received NoOp packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessHello(id, extensions, name, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            /*Send Hello reply then process the hello */
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
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessPublicKey(id, public_key, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessRoot(id, public_key, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessGetDatum(id, hash, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessHelloReply(extensions, name,sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            ActivePeers::set_peer_extensions_and_name(active_peers, sock_addr, extensions, name);
            /*keep alive ? */
            // Queue::lock_and_push(Arc::clone(&action_queue),
            //                         Action::SendHelloReply(id,
            //                               my_data.get_extensions().unwrap(),
            //                               my_data.get_name().unwrap(),
            //                               sock_addr
            //                             )
            //         );
            // QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            return;
        }, 
        Action::ProcessErrorReply(err_msg_reply, sock_addr) =>{
            println!("Received ErrorReply packet with message:{}\n From {}\n",
                                 String::from_utf8_lossy(&err_msg_reply), sock_addr);
            return;
        }, 
        Action::ProcessPublicKeyReply(public_key, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessRootReply(root, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessDatum(datum, sock_addr) =>{
            println!("Received Hello packet from {}\n", sock_addr);
            return;
        }, 
        _=>println!("Shouldn't happen: {:?}", action),
    }

    /*Queue to peek the main queues */
    pub async fn register(peek_queue: Arc<RwLock<Queue<Action>>>,
                        ){

    }

}
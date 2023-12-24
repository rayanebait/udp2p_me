use std::sync::{Arc, Mutex};

use crate::congestion_handler::*;
use crate::action::Action;

pub async fn process_task(process_queue: Arc<Mutex<Queue<Action>>>,
                          process_queue_state: Arc<QueueState>,
                          action_queue: Arc<Mutex<Queue<Action>>>,
                          action_queue_state: Arc<QueueState>,
                          //tree: ?
                          //hash_map:?
                          //self_data:?
                        ){
    tokio::spawn(async move {
        loop {
            match Queue::lock_and_pop(Arc::clone(&process_queue)){
                Some(action)=> {
                    /*action queue is not empty get an action and handle it*/
                    println!("process");
                    process_action(action,
                        Arc::clone(&action_queue),
                    Arc::clone(&action_queue_state));
                /*return the action required */

                },
                None=>{
                    /*
                        action queue is empty wait for the activity of 
                        the receive queue
                        */
                    
                    println!("action wait");
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
        Action::StorePublicKey(..)=>todo!(),
        Action::StoreRoot(..)=>todo!(),
        Action::ProcessHelloReply(..)=>todo!(),
        Action::ProcessDatum(..)=>todo!(),
        Action::ProcessErrorReply(..)=>todo!(),
        _=>panic!("Shouldn't happen"),
    }

}
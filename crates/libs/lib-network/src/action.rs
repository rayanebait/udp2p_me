use std::net::UdpSocket;
use std::sync::{Arc, Mutex};

use crate::packet::{Packet, PacketBuilder};
use crate::handle_packet::{Action, HandlingError};
use crate::congestion_handler::*;


pub async fn handle_action_task(send_queue: Arc<Mutex<SendQueue>>,
                          receive_queue_state: Arc<QueueState>,
                          action_queue: Arc<Mutex<ActionQueue>>){
        tokio::spawn(async move {
            loop {
                println!("la");
                match ActionQueue::lock_and_pop(Arc::clone(&action_queue)){
                    Some(action)=> 
                        /*action queue is not empty get an action and handle it*/
                        handle_action(action,
                                Arc::clone(&send_queue)
                                /*return the action required */
                        ),
                    None=>{
                        /*
                            action queue is empty wait for the activity of 
                            the receive queue
                        */
                        // receive_queue_state.wait();
                        println!("la");
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue
                    }
                };
            }
        }).await;
}

pub fn handle_action(action: Action, send_queue: Arc<Mutex<SendQueue>>){
    match action {
        Action::NoOp(sock_addr) =>{
            println!("Received NoOp packet from {}\n", sock_addr);
            return;
        }, 
        Action::ProcessDatum(..) =>(), 
        Action::ProcessErrorReply(..) =>(), 
        Action::ProcessHelloReply(..) =>(), 
        
        Action::SendHelloReply(id, sock_addr)=>{
            let packet = PacketBuilder::hello_reply_packet(&id);
            SendQueue::lock_and_push(Arc::clone(&send_queue), packet, sock_addr)
        },
        Action::SendError(..) =>(), 
        Action::SendDatumWithHash(..) =>(), 
        Action::SendPublicKey(optional_key, sock_addr) =>{
            let packet = PacketBuilder::public_key_packet(optional_key);
            SendQueue::lock_and_push(Arc::clone(&send_queue), packet, sock_addr)
        }, 
        Action::SendRoot(optional_root, sock_addr) =>{
            let packet = PacketBuilder::root_packet(optional_root);
            SendQueue::lock_and_push(Arc::clone(&send_queue), packet, sock_addr)
        },

        Action::SendRootReply(optional_root, sock_addr) =>{
            let packet = PacketBuilder::root_reply_packet(optional_root);
            SendQueue::lock_and_push(Arc::clone(&send_queue), packet, sock_addr)
        },

        Action::StoreRoot(..) =>(), 
        Action::StorePublicKey(..) =>(), 
        _ =>(),
    };
}
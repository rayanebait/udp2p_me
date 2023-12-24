use std::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::packet::{Packet, PacketBuilder};
use crate::handle_packet::HandlingError;
use crate::action::Action;

use crate::congestion_handler::*;
use crate::process;


/*Waits for the signal that the action queue is not empty 
then handles the action. Can push to the send queue so
it also notifies the send queue wether is it empty or not. */
pub async fn handle_action_task(send_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
                                send_queue_state: Arc<QueueState>,
                                action_queue: Arc<Mutex<Queue<Action>>>,
                                action_queue_state: Arc<QueueState>,
                                process_queue: Arc<Mutex<Queue<Action>>>,
                                process_queue_state: Arc<QueueState>
                            ){
    tokio::spawn(async move {
        loop {
            match Queue::lock_and_pop(Arc::clone(&action_queue)){
                Some(action)=> {
                    /*action queue is not empty get an action and handle it*/
                    println!("handle action");
                    handle_action(action,
                        Arc::clone(&send_queue),
                    Arc::clone(&send_queue_state),
                                  Arc::clone(&process_queue),
                                  Arc::clone(&process_queue_state)
                );
                /*return the action required */

                },
                None=>{
                    /*
                        action queue is empty wait for the activity of 
                        the receive queue
                        */
                    
                    println!("action wait");
                    QueueState::set_empty_queue(Arc::clone(&action_queue_state));
                    action_queue_state.wait();
                    continue
                }
            };
        }
    }).await;
}

pub fn handle_action(action: Action,
                     send_queue: Arc<Mutex<Queue<(Packet,SocketAddr)>>>,
                     send_queue_state: Arc<QueueState>,
                     process_queue: Arc<Mutex<Queue<Action>>>,
                     process_queue_state: Arc<QueueState>
                        ){
    match action {
        Action::NoOp(sock_addr) =>{
            println!("Received NoOp packet from {}\n", sock_addr);
            return;
        }, 
        Action::SendHelloReply(id, sock_addr)=>{
            let packet = PacketBuilder::hello_reply_packet(&id);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
        },
        Action::SendError(..) =>(), 
        Action::SendDatumWithHash(..) =>(), 
        Action::SendPublicKey(optional_key, sock_addr) =>{
            let packet = PacketBuilder::public_key_packet(optional_key);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
        }, 
        Action::SendRoot(optional_root, sock_addr) =>{
            let packet = PacketBuilder::root_packet(optional_root);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
        },

        Action::SendRootReply(optional_root, sock_addr) =>{
            let packet = PacketBuilder::root_reply_packet(optional_root);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
        },

        _ =>{
            Queue::lock_and_push(Arc::clone(&process_queue), action);
            QueueState::set_non_empty_queue(Arc::clone(&process_queue_state));
        },
    };
}
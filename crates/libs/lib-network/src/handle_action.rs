
use std::net::SocketAddr;

use std::sync::RwLock;
use std::sync::{Arc, Mutex};

use crate::action::Action;

use log::debug;

use crate::packet::{Packet, PacketBuilder};
use log::error;

use crate::congestion_handler::*;


/*Waits for the signal that the action queue is not empty
then handles the action. Can push to the send queue so
it also notifies the send queue wether is it empty or not. */
pub fn handle_action_task(
    send_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    send_queue_state: Arc<QueueState>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
) {
    tokio::spawn(async move {
        loop {
            match Queue::lock_and_pop(Arc::clone(&action_queue)) {
                Some(action) => {
                    /*action queue is not empty get an action and handle it*/
                    handle_action(
                        action,
                        Arc::clone(&send_queue),
                        Arc::clone(&send_queue_state),
                        Arc::clone(&process_queue),
                        Arc::clone(&process_queue_state),
                    );
                    /*return the action required */
                }
                None => {
                    /*
                        action queue is empty wait for the activity of
                        the receive/process queue
                    */
                    QueueState::set_empty_queue(Arc::clone(&action_queue_state));
                    // println!("action wait");
                    debug!("handle wait");
                    action_queue_state.wait();
                    continue
                    }
            };
        }

    });
}

/*Add NatTraversal and NatTraversal reply */
pub fn handle_action(
    action: Action,
    send_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    send_queue_state: Arc<QueueState>,
    process_queue: Arc<RwLock<Queue<Action>>>,
    process_queue_state: Arc<QueueState>,
) {
    match action {
        Action::SendNoOp(sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::noop_packet();
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendHello(extensions, name, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::hello_packet(extensions.as_ref(), name);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendError(error_msg, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::error_packet(Some(error_msg));
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendPublicKey(public_key, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::public_key_packet(public_key);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendRoot(root, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::root_packet(root);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendGetDatumWithHash(hash, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::get_datum_packet(hash);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendNatTraversalRequest(behind_nat, server_sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::nat_traversal_request_packet(behind_nat);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, server_sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendHelloReply(id, extensions, name, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::hello_reply_packet(&id, extensions, name);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendErrorReply(id, err_reply_msg, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::error_reply_packet(&id, err_reply_msg);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendPublicKeyReply(id, public_key, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::public_key_reply_packet(public_key, id);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendRootReply(id, root, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::root_reply_packet(&id, root);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendDatumWithHash(id, hash, datum, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::datum_packet(&id, hash, datum);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        Action::SendNoDatum(id, sock_addr) => {
            /*DONE */
            let packet = PacketBuilder::nodatum_packet(&id);
            Queue::lock_and_push(Arc::clone(&send_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&send_queue_state));
            return;
        }
        _ => {
            /*TO DO*/
            error!("Shouldn't happen if not planned.");
            Queue::write_lock_and_push(Arc::clone(&process_queue), action);
            QueueState::set_non_empty_queue(Arc::clone(&process_queue_state));
        }
    };
}

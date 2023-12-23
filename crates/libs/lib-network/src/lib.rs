// pub mod peer_data;
pub mod packet;
pub mod congestion_handler;
pub mod handle_packet;
pub mod action;
pub mod sender_receiver;

pub mod import_export{
    use std::default;
    use std::future::IntoFuture;
    use prelude::*;

    use std::sync::{Arc, Mutex};

    /*Utilities */
    use anyhow::{bail, Context, Result};
    use log::{debug, error, info, warn};
    
    /*Async/net libraries */
    use futures::stream::FuturesUnordered;
    use futures::{Future,StreamExt, select};
    use tokio::net::UdpSocket;
    use std::net::SocketAddr;
    use tokio::test;

    use crate::congestion_handler::*;
    // use crate::peer_data::*;
    use crate::packet::*;
    use crate::handle_packet::*;
    
    pub enum Error{
        Packet(PacketError),
        // Peer(PeerError),
    }

    pub async fn timeout(dur: Duration)->Result<(SocketAddr ,Packet), PacketError>{
        tokio::time::sleep(Duration::from_secs(4)).await;

        Err(PacketError::UnknownError)
    }

    // pub async fn handshake_addr(sock: Arc<UdpSocket>, peer_addr: SocketAddr,
    //                 pending: Arc<Mutex<PendingIds>>,
    //                 active_peers: Arc<Mutex<ActivePeers>>)->Result<(), PeerError>{

    //     /*Launch async task*/
    //     let handle = 
    //         tokio::spawn(async move{
    //         let Ok(packet_id) 
    //             = Packet::send_hello(&sock, &peer_addr.clone()).await 
    //             else{
    //                 return Err(PeerError::UnknownError);
    //             };

    //         /*Lock pending ids to push the id of the packet sent */
    //         let mut arc_pending_guard = pending
    //                                     .lock().unwrap();

    //         /*Push the id of the packet sent */
    //         arc_pending_guard.add_packet_id_raw(packet_id , &peer_addr.clone());

    //         /*Unlock the ids for other tasks */
    //         /*It used to not work and the actions taken on 
    //         a lock should be done in a sub {} scope, it should
    //         work now*/
    //         drop(arc_pending_guard);
    //         /*Remark: add_packet_id is not async so that the mutex guard 
    //         is not sent in an async where it could be awaited and create 
    //         a deadlock */

    //         /*
    //             waiting.iter().next() launches its elements (functions)
    //             concurrently and waits for the first to finish then returns
    //         */
    //         let waiting = FuturesUnordered::new();

    //         /*Push a timer of 1 second*/
    //         waiting.push(timeout(Duration::from_secs(1)).await);

    //         loop{
    //             let mut wait_packet =
    //                     Packet::recv_from(&sock);
                
    //             /*Push a listener on the socket*/
    //             waiting.push(wait_packet.await);

    //             /*Wait until either the timer finishes or a packet is received*/
    //             let packet_or_timeout =
    //                      waiting.iter().next().unwrap();
                
    //             match packet_or_timeout {
    //                 Ok((sock_addr, packet)) =>{
    //                     if !packet.is(packet.get_packet_type()){
    //                         continue;
    //                     } else {
    //                         // let arc_active_peers_guard =
    //                         //              active_peers.lock().unwrap();
    //                         // arc_active_peers_guard.ad
    //                         return Ok(());
    //                     }
    //                 },
    //                 Err(PacketError::UnknownError) =>
    //                          return Err(PeerError::ResponseTimeoutError),
    //                 _ => panic!("Shouldn't happen"),
    //             }
    //         }
    //     }).await;

    //     handle.unwrap()

    // }

    // pub async fn handshake(sock: Arc<UdpSocket>, peer: PeerData,
    //                 pending: Arc<Mutex<PendingIds>>,
    //                 active_peers: Arc<Mutex<ActivePeers>>)->Result<(), PeerError>{

    //     tokio::spawn(async move {
    //             let addrs = peer.get_socketaddr().unwrap();
    //             let addr = addrs.get(0).unwrap();

    //             handshake_addr(Arc::clone(&sock),
    //                  addr.clone(), Arc::clone(&pending),
    //                 Arc::clone(&active_peers)).await
    //         }
    //     ).await.unwrap()
    // }

    // pub async fn send_public_key(){

    // }

    // pub async 
    // fn register(sock: Arc<UdpSocket>, root: Option<[u8;32]>,
    //          public_key: Option<[u8;64]>,
    //         pending: Arc<Mutex<PendingIds>>,
    //         active_peers: Arc<Mutex<ActivePeers>>)->Result<(),PeerError>{

    //     let mut server = PeerData::new();
    //     server.set_socketaddr( vec!["81.194.27.155:8443".parse().unwrap()]);

    //     handshake(sock, server,
    //                     Arc::clone(&pending),
    //                     Arc::clone(&active_peers)).await;
        

    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::SocketAddr, thread::sleep, time::Duration};

    use super::*;

    use nanorand::{Rng, BufferedRng, wyrand::WyRand};

    use import_export::*;
    use tokio::{self, net::UdpSocket, runtime::Handle, runtime};
    use futures::join;
    use std::sync::{Arc, Mutex};

    use crate::congestion_handler::ReceiveQueue;
    use crate::congestion_handler::build_queues;
    use crate::handle_packet::handle_packet_task;
    use crate::packet::*;
    use crate::sender_receiver::*;
    use crate::action::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
    #[tokio::test]
    async fn packet_bytes_conversion(){
        let mut rng = BufferedRng::new(WyRand::new());
        let mut rand_id : [u8;4]= [0;4];
        rng.fill(&mut rand_id);
        drop(rng);

        let packet = PacketBuilder::new()
                        .set_id(rand_id.clone())
                        .body(b"what is this".to_vec())
                        .packet_type(PacketType::Datum)
                        .build()
                        .unwrap();

        let hello_packet = PacketBuilder::hello_packet();
        let raw_hello_packet = hello_packet.as_bytes();
        let raw_packet = packet.as_bytes();

        println!("{:?}\n{:?}", raw_packet, packet);
        println!("{:?}\n{:?}", raw_hello_packet, hello_packet);
    }
    #[tokio::test]
    async fn handshake_with_pi(){
        let packet = PacketBuilder::hello_packet();
        let packet2 = PacketBuilder::new()
                            .body(b"j'ai rotey :)".to_vec())
                            .gen_id()
                            .build()
                            .unwrap();
        let sock = UdpSocket::bind("172.20.10.7:0").await
                    .unwrap();
        packet.send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap()).await;
        sleep(Duration::from_millis(500));
        packet2.send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap()).await;
        sleep(Duration::from_millis(500));
    }

    #[tokio::test(flavor="multi_thread",worker_threads=5)]
    async fn register_to_server(){
        /*0 lets the os assign the port. The port is 
        then accessible with the local_addr method */
        let sock = Arc::new(
            UdpSocket::bind("192.168.1.90:0").await
                                            .unwrap()
                                        );
        
        let (receive_queue,
             send_queue,
             pending_ids,
             action_queue,
             receive_queue_state,
             action_queue_state,
             send_queue_state)
             = build_queues();
             
        {
            let packet = PacketBuilder::hello_packet();
            let sock_addr = "176.169.27.221:9157".parse().unwrap();
            ReceiveQueue::lock_and_push(Arc::clone(&receive_queue), packet, sock_addr);
            let packet = PacketBuilder::hello_packet();
            let sock_addr2 = "176.169.27.221:37086".parse().unwrap();
            ReceiveQueue::lock_and_push(Arc::clone(&receive_queue), packet, sock_addr2)
        }
        /*Do a launch tasks func ? */
        let f1 = receiver(Arc::clone(&sock),
                 Arc::clone(&receive_queue),
                 Arc::clone(&receive_queue_state));
                 
                 
        let f3 = handle_packet_task(
            Arc::clone(&pending_ids),
                Arc::clone(&receive_queue),
                Arc::clone(&receive_queue_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state));
        let f4 = handle_action_task(
                Arc::clone(&send_queue),
                Arc::clone(&send_queue_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
            );
                          
        let f2 = sender(Arc::clone(&sock),
                Arc::clone(&send_queue),
                Arc::clone(&send_queue_state),
                Arc::clone(&pending_ids));

        // let metrics = Handle::current().metrics();
        // let n = metrics.active_tasks_count();
        // println!("Runtime has {} active tasks", n);

        join!(f1,f2,f3,f4);
    }

}

use std::sync::Arc;

use tokio::{net::UdpSocket, signal};
use tracing::{info, error};
use serde::{Serialize, Deserialize};
use bincode::{self};

use crate::SerfMember;

#[derive(Deserialize, Serialize, Debug)]
pub(crate) enum SerfMessageType {
    Ping(String),
    Ack(String),
    Join(SerfMember),
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct SerfMessage {
    pub(crate) msg_num: u64,
    pub(crate) msg_type: SerfMessageType,
}

#[derive(Debug)]
pub struct Gossiper {
    pub(crate) sock: Arc<UdpSocket>,
}

impl Gossiper {
    pub async fn listen(sock: Arc<UdpSocket>, tx: tokio::sync::mpsc::Sender<(SerfMessage, String)>) {
        let mut buf = vec![0u8; 65535];
        info!("starting gossip listener");
        loop{
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("signal to shutdown, stopping");
                    return
                },
                res = sock.recv_from(&mut buf) => {
                    let (len, addr) = match res {
                        Ok((len, addr)) => (len, addr),
                        Err(err) => {
                            error!("failed to receive message {:?}", err);
                            break;
                        }
                    };

                    info!("got message from {:?}", addr);

                    let msg: SerfMessage = match bincode::deserialize(&buf){
                        Ok(msg) => msg,
                        Err(err) => {
                            error!("failed to deserialize message of size {:?} from {:?}: {:?}", len, addr, err);
                            break;
                        }
                    };

                    match tx.send((msg, addr.to_string())).await {
                        Ok(_) => continue,
                        Err(err) => {
                            error!("failed to send message over channel: {:?}", err);
                            continue;
                        },
                    };
                }
            }
        }
    }

    pub async fn send_msg(&self, addr: String, msg: SerfMessage) -> Result<(), String> {
        let buf = match bincode::serialize(&msg){
            Ok(buf) => buf,
            Err(err) => return Err(format!("failed to serialize message: {:?}", err)),
        };
        match self.sock.send_to(buf.as_slice(), addr).await {
            Ok(_) => {},
            Err(err) => return Err(format!("failed to send gossip message: {:?}", err))
        };

        Ok(())
    }
}
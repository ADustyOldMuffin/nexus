mod communication;

use std::{sync::Arc, collections::HashMap};
use rand::seq::SliceRandom;

use serde::{Serialize, Deserialize};
use tokio::{net::UdpSocket, sync::mpsc, signal};

use communication::{Gossiper, SerfMessage};
use tokio::time::{self, Duration};
use tracing::{debug, error, info};


#[derive(Debug)]
pub struct SerfClient {
    udp_port: u16,
    client_addr: String,
    start_join: Vec<String>,

    gossiper: Option<Gossiper>,
    labels: HashMap<String, String>,

    current_state: SerfState,
    updates_buffer: Vec<SerfMember>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum SerfStatus {
    Up,
    Suspect,
    Down,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SerfState {
    members: Vec<SerfMember>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SerfMember {
    addr: String,
    status: SerfStatus,
    labels: HashMap<String, String>
}

impl SerfClient {
    // This method will help users to discover the builder
    pub fn builder() -> SerfMemberBuilder {
        SerfMemberBuilder::default()
    }

    pub async fn start(&mut self) -> Result<(), std::io::Error> {
        let sock = Arc::new(UdpSocket::bind(format!("{:?}:{:?}", self.client_addr, self.udp_port)).await.expect("failed to bind to gossip socket"));
        self.gossiper = Some(Gossiper{sock: sock.clone()});
        let (tx, mut rx) = mpsc::channel::<(SerfMessage, String)>(1);
        tokio::task::spawn(Gossiper::listen(sock.clone(), tx));
        let mut interval = time::interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                val = rx.recv() => {
                    match val {
                        Some((msg, sender)) => {
                            match msg.msg_type {
                                communication::SerfMessageType::Ping(_) => self.handle_ping_msg(sender).await,
                                communication::SerfMessageType::Ack(_) => self.handle_ack_msg(sender).await,
                                communication::SerfMessageType::Join(member) => self.handle_join_msg(sender, member).await,
                            }
                        },
                        None => {},
                    }
                },
                _ = interval.tick() => {
                    let g = self.gossiper.as_ref().unwrap();
                    let to_ping = self.current_state.members.choose(&mut rand::thread_rng()).unwrap();
                    debug!("sending ping message to {:?}", to_ping.addr);
                    
                },
                _ = signal::ctrl_c() => {
                    info!("got signal to stop... stopping gossip");
                    return Ok(());
                },
            }
        }
    }

    pub async fn handle_ping_msg(&self, addr: String) {
        debug!("got a ping message from {:?}", addr);
        let g = self.gossiper.as_ref().unwrap();
        let msg = SerfMessage {
            msg_num: 0,
            msg_type: communication::SerfMessageType::Ack(self.client_addr.to_owned()) 
        };
        match g.send_msg(addr, msg).await {
            Ok(_) => debug!("successfully sent ack"),
            Err(err) => error!("failed to send ack {:?}", err),
        };
    }

    pub async fn handle_join_msg(&self, addr: String, member: SerfMember) {

    }

    pub async fn handle_ack_msg(&self, addr: String) {
        debug!("got an ack message from {:?}", addr);
    }
}

#[derive(Default)]
pub struct SerfMemberBuilder {
    udp_port: u16,
    client_addr: String,
    start_join: Vec<String>,
    labels: HashMap<String, String>,
}

impl SerfMemberBuilder {
    pub fn new() -> SerfMemberBuilder {
        SerfMemberBuilder {
            udp_port: 9120,
            client_addr: "127.0.0.1".to_string(),
            start_join: vec![],
            labels: HashMap::new()
        }
    }

    pub fn with_udp_port(&mut self, port: u16) -> &mut SerfMemberBuilder {
        self.udp_port = port;
        self
    }

    pub fn with_start_join(&mut self, members: Vec<String>) -> &mut SerfMemberBuilder {
        self.start_join = members;
        self
    }

    pub fn with_client_addr(&mut self, client_addr: String) -> &mut SerfMemberBuilder {
        self.client_addr = client_addr;
        self
    }

    pub fn with_labels(&mut self, labels: HashMap<String, String>) -> &mut SerfMemberBuilder {
        self.labels = labels;
        self
    }

    pub fn build(self) -> Result<SerfClient, std::io::Error> {
        Ok(SerfClient { 
            udp_port: self.udp_port, 
            start_join: self.start_join, 
            gossiper: None,
            client_addr: self.client_addr,
            labels: self.labels,
            updates_buffer: vec![],
            current_state: SerfState{
                members: vec![],
            }
        })
    }
}

use std::{collections::{HashMap, HashSet}, net::SocketAddr};

use futures::Stream;
use async_stream::stream;
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncWriteExt, net::TcpStream,
    sync::{broadcast, mpsc::{self, Receiver, Sender}},
    task::JoinHandle,
};


/// messages sent from the server
#[derive(Serialize, Deserialize)]
enum ServerMessage {
    SetLed(bool),
    SetPin(bool),
}

/// messages sent from the client
#[derive(Serialize, Deserialize)]
enum ClientMessage {
    Connected,
}

struct MessageHandler<T> {
    addr: SocketAddr,
    sender: mpsc::Sender<(SocketAddr, T)>,
}

impl<T> MessageHandler<T> {
    fn new(
        addr: SocketAddr,
        sender: mpsc::Sender<(SocketAddr, T)>
    ) -> Self {
        Self { addr, sender }
    }
    
    pub async fn send(&self, msg: T) {
        self.sender.send((self.addr, msg)).await
            .expect("could not send message");
    }
}

struct ConnectedNode {
    task: JoinHandle<()>,
    sender: Sender<ServerMessage>,
    receiver: Receiver<ClientMessage>,
    connections: HashSet<SocketAddr>,
    handler: MessageHandler<ClientMessage>,
}

impl ConnectedNode {
    fn new(
        mut conn: TcpStream,
        handler: MessageHandler<ClientMessage>,
    ) -> Self {
        let (sender, mut rx) = mpsc::channel::<ServerMessage>(16);
        
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        let msg = postcard::to_allocvec(&msg).expect("could not serialize server message");
                        conn.write_all(&msg[..]).await
                            .expect("could not send message");
                    },
                }
            }
        });
        
        Self { task, sender, handler }
    }
    
    async fn send(&self, msg: ServerMessage) {
        self.sender.send(msg).await
            .expect("could not send")
    }
}

struct Mesh {
    nodes: HashMap<SocketAddr, ConnectedNode>,
    events: broadcast::Sender<MeshEvent>,
}

#[derive(Clone)]
enum MeshEvent {
    UpdateConnections,
}

impl Mesh {
    pub fn new() -> Self {
        
    }
    
    pub fn events(&self) -> impl Stream<Item = MeshEvent> {
        let mut events = self.events.subscribe();
        stream! {
            while let Ok(ev) = events.recv().await {
                yield ev;
            }
        }
    }
    
    pub async fn check_connections(&self) {
        use ServerMessage::*;
        for node in self.nodes.values() {
            node.send(SetPin(true)).await;
            
        }
    }
}




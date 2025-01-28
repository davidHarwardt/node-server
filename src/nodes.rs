
use std::{any::Any, collections::{HashMap, HashSet}, net::SocketAddr, sync::Arc};

use futures::Stream;
use async_stream::stream;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream,
    sync::{broadcast, mpsc::{self, Sender}, oneshot, Mutex},
    task::JoinHandle,
};


/// messages sent from the server
#[derive(Serialize, Deserialize, Clone)]
enum ServerMessage {
    SetLed(bool),
    SetPin(bool),
    CheckConnection,
}

/// messages sent from the client
#[derive(Serialize, Deserialize)]
enum ClientMessage {
    Connected,
    DidSet,
    ConnectedResult(bool),
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

type MapFn = Box<dyn FnMut(&ClientMessage) -> Option<Box<dyn Any + Send>> + Send>;
type MapVec = Vec<(MapFn, Option<oneshot::Sender<Box<dyn Any + Send>>>)>;

struct ConnectedNode {
    task: JoinHandle<()>,
    addr: SocketAddr,
    sender: Sender<ServerMessage>,
    waiting: Arc<Mutex<MapVec>>,
    // receiver: Receiver<ClientMessage>,
    connections: HashSet<SocketAddr>,
}

impl PartialEq for ConnectedNode {
    fn eq(&self, other: &Self) -> bool { self.addr == other.addr }
}

impl ConnectedNode {
    fn new(
        mut conn: TcpStream,
        handler: MessageHandler<ClientMessage>,
    ) -> Self {
        let (sender, mut rx) = mpsc::channel::<ServerMessage>(16);
        let waiting = Arc::new(Mutex::new(MapVec::new()));
        let addr = conn.peer_addr().unwrap();
        
        let task = tokio::spawn({
            let waiting = Arc::clone(&waiting);
            async move {
                loop {
                    tokio::select! {
                        // send message to node
                        msg = rx.recv() => {
                            let Some(msg) = msg else { break };
                            let msg = postcard::to_allocvec(&msg).expect("could not serialize server message");
                            let Ok(_) = conn.write_u64(msg.len() as _).await else { break };
                            let Ok(_) = conn.write_all(&msg[..]).await else { break };
                        },
                        // forward messages from node to mesh
                        res = conn.readable() => {
                            let Ok(_) = res else { break };
                            let Ok(buf_len) = conn.read_u64().await else { break };
                            let mut buf = vec![0u8; buf_len as usize];
                            let Ok(_) = conn.read_exact(&mut buf[..]).await else { break };
                            match postcard::from_bytes::<ClientMessage>(&buf[..]) {
                                Ok(m) => {
                                    let mut l = waiting.lock().await;
                                    let mut did_send = false;
                                    for (f, s) in &mut *l {
                                        if let Some(r) = f(&m) {
                                            s.take().unwrap().send(r).unwrap();
                                            did_send = true;
                                            break
                                        }
                                    }
                                    l.retain(|(_, s)| { s.is_some() });
                                    if !did_send { handler.send(m).await };
                                },
                                Err(err) => {
                                    tracing::error!("could not deserialize: {err}");
                                    break
                                },
                            }
                        },
                    }
                }
            }
        });
        let connections = HashSet::new();
        
        Self { task, sender, connections, waiting, addr }
    }
    
    async fn send(&self, msg: ServerMessage) {
        self.sender.send(msg).await
            .expect("could not send")
    }
    
    async fn wait_for<U: Any + Send>(&self, mut f: impl FnMut(&ClientMessage) -> Option<U> + Send + 'static) -> U {
        let (tx, rx) = oneshot::channel();
        self.waiting.lock().await.push((Box::new(move |m| {
            f(m).map(|v| Box::new(v) as Box<dyn Any + Send>)
        }), Some(tx)));
        *rx.await.unwrap().downcast().unwrap()
    }
}

struct Mesh {
    nodes: HashMap<SocketAddr, ConnectedNode>,
    task: JoinHandle<()>,
    events: broadcast::Sender<MeshEvent>,
}

#[derive(Clone)]
enum MeshEvent {
    UpdateConnections,
}

impl Mesh {
    pub fn new() -> Self {
        // let (rx, tx) = mpsc::channel(16);
        let (events, _) = broadcast::channel(16);
        let nodes = HashMap::new();
        
        let task = tokio::spawn(async move {
            loop {
                
            }
        });
        Self { task, nodes, events }
    }
    
    pub fn events(&self) -> impl Stream<Item = MeshEvent> {
        let mut events = self.events.subscribe();
        stream! {
            while let Ok(ev) = events.recv().await {
                yield ev;
            }
        }
    }
    
    pub fn without<'a>(&'a self, node: &'a ConnectedNode) -> impl Iterator<Item = &'a ConnectedNode> + 'a {
        self.nodes.values().filter(move |n| n != &node)
    }
    
    pub async fn broadcast_without(&self, msg: ServerMessage, without: &ConnectedNode) {
        for n in self.nodes.values() {
            if n != without { n.send(msg.clone()).await }
        }
    }
    
    pub async fn check_connections(&self) {
        use ServerMessage::*;
        for node in self.nodes.values() {
            node.send(SetPin(true)).await;
            node.wait_for(|m| matches!(m, ClientMessage::DidSet).then_some(())).await;
            self.broadcast_without(ServerMessage::CheckConnection, node).await;
            
            let mut tasks = vec![];
            for n in self.without(node) {
                tasks.push(tokio::spawn(
                    node.wait_for(|m| match m {
                        ClientMessage::ConnectedResult(r) => Some(*r),
                        _ => None,
                    })
                ));
            }
            for (n, task) in self.without(node).zip(tasks) {
                let connected = task.await;
            }
            
            // which node received the message
            node.send(SetPin(false)).await;
        }
    }
}




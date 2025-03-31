// use futures_util::stream::SplitSink;
// use futures_util::{SinkExt, StreamExt};
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::net::{TcpListener, TcpStream};
// use tokio::sync::Mutex;
// use tokio_tungstenite::WebSocketStream;
// use tokio_tungstenite::tungstenite::Message;
// use tokio_tungstenite::{accept_async, tungstenite::Result};

// type Room =
//     Arc<Mutex<HashMap<String, Vec<Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>>>>>;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let addr = "127.0.0.1:8080";
//     let listener = TcpListener::bind(addr).await?;
//     let rooms: Room = Arc::new(Mutex::new(HashMap::new()));
//     println!("Server running on {}", addr);

//     while let Ok((stream, _)) = listener.accept().await {
//         let rooms = rooms.clone();
//         tokio::spawn(handle_connection(stream, rooms));
//     }

//     Ok(())
// }

// async fn handle_connection(stream: TcpStream, rooms: Room) -> Result<()> {
//     let ws_stream = accept_async(stream).await?;
//     println!("New WebSocket connection established");

//     let (write_half, mut read_half) = ws_stream.split();
//     let write_half = Arc::new(Mutex::new(write_half));

//     let mut current_room: Option<String> = None;

//     while let Some(msg) = read_half.next().await {
//         println!("hello1");
//         let msg = msg?;
//         if msg.is_text() {
//             let text = msg.to_string();
//             println!("hello1: {text}");
//             if text.starts_with("JOIN:") {
//                 println!("someone joined with msg: {}", text);
//                 let room_name = text.strip_prefix("JOIN:").unwrap().to_string();
//                 current_room = Some(room_name.clone());

//                 let mut rooms_lock = rooms.lock().await;
//                 rooms_lock
//                     .entry(room_name)
//                     .or_insert_with(Vec::new)
//                     .push(write_half.clone());
//             } else if let Some(room) = &current_room {
//                 println!("hello122");
//                 let clients = {
//                     let rooms_lock = rooms.lock().await;
//                     rooms_lock.get(room).cloned()
//                 };

//                 if let Some(clients) = clients {
//                     let mut disconnected_clients = vec![];
//                     for (i, client) in clients.iter().enumerate() {
//                         let mut client_lock = client.lock().await;
//                         if client_lock
//                             .send(Message::Text(text.clone().into()))
//                             .await
//                             .is_err()
//                         {
//                             disconnected_clients.push(i);
//                         }
//                     }

//                     // Remove disconnected clients
//                     let mut rooms_lock = rooms.lock().await;
//                     if let Some(clients) = rooms_lock.get_mut(room) {
//                         println!("DISCONNETING USER");
//                         for &index in disconnected_clients.iter().rev() {
//                             clients.remove(index);
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(())
// }

// -------------------------------with redis ----------------------------

// use std::sync::Arc;

// use futures_util::{SinkExt, StreamExt, lock::Mutex};
// use redis::AsyncCommands;
// use tokio::{
//     net::{TcpListener, TcpStream},
//     sync::mpsc,
//     task::{self, JoinHandle},
// };
// use tokio_tungstenite::{
//     accept_async,
//     tungstenite::{Message, Result},
// };

// type RedisClient = Arc<Mutex<redis::Client>>;

// #[tokio::main]
// async fn main() {
//     let listener = TcpListener::bind("127.0.0.1:8080")
//         .await
//         .expect("failed to connect to TcpListener");
//     println!("connect to the TcpListener 127.0.0.1:8080");

//     let redis_client = Arc::new(Mutex::new(
//         redis::Client::open("redis://127.0.0.1/").unwrap(),
//     ));

//     while let Ok((stream, _)) = listener.accept().await {
//         let redis_client = Arc::clone(&redis_client);
//         tokio::spawn(handle_connection(stream, redis_client));
//     }
// }

// async fn handle_connection(stream: TcpStream, redis_client: RedisClient) -> Result<()> {
//     let ws_stream = accept_async(stream)
//         .await
//         .expect("failed to get WebSocketStream");
//     println!("Connected to the ws_stream");

//     let (mut write, mut read) = ws_stream.split();
//     let (tx, mut rx) = mpsc::unbounded_channel::<String>();

//     let mut current_room: Option<String> = None;

//     let redis_client_sub = Arc::clone(&redis_client);

//     task::spawn(async move {
//         while let Some(msg) = rx.recv().await {
//             if write.send(Message::Text(msg.into())).await.is_err() {
//                 break;
//             }
//         }
//     });

//     while let Some(msg) = read.next().await {
//         let msg = msg?;
//         if msg.is_text() {
//             let text = msg.to_string();
//             println!("worked {}", text);
//             let mut redis_conn = redis_client
//                 .lock()
//                 .await
//                 .get_multiplexed_async_connection()
//                 .await
//                 .unwrap();
//             // let mut redis_conn = redis_client.lock().await.get_async_connection().await.unwrap();
//             println!("worked");

//             if text.starts_with("JOIN:") {
//                 let room_name = text.strip_prefix("JOIN:").unwrap().to_string();
//                 current_room = Some(room_name.clone());

//                 // Subscribe to Redis channel
//                 let redis_client_sub = Arc::clone(&redis_client);
//                 let tx_clone = tx.clone();
//                 let room_clone = room_name.clone();

//                 task::spawn(async move {
//                     let mut pubsub = redis_client_sub.lock().await.get_async_pubsub().await.unwrap();
//                     pubsub.subscribe(room_clone).await.unwrap();

//                     while let Some(msg) = pubsub.on_message().next().await {
//                         let payload:String = msg.get_payload().unwrap();
//                         let _ = tx_clone.send(payload);
//                     }
//                     // let mut subs:JoinHandle<> = redis_client_sub.lock().await.get_multiplexed_async_connection().await.unwrap().subscribe(room_clone).await.unwrap();
//                 });
//             } else if let Some(room) = &current_room {
//                 let _ :()= redis_conn.publish(room, text).await.unwrap();
//             }
//         }
//     }
//     Ok(())
// }


use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use redis::{AsyncCommands, Client, aio::PubSub};

type Room = Arc<Mutex<HashMap<String, Vec<mpsc::Sender<String>>>>>;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    let rooms: Room = Arc::new(Mutex::new(HashMap::new()));

    let redis_client = Client::open("redis://127.0.0.1/")?;
    let mut redis_con = redis_client.get_multiplexed_async_connection().await?;

    let rooms_clone = rooms.clone();
    tokio::spawn(async move {
        subscribe_to_redis(redis_client, rooms_clone).await;
    });

    println!("WebSocket Server running on {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let rooms = rooms.clone();
        let redis_con = redis_con.clone();
        tokio::spawn(handle_connection(stream, rooms, redis_con));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, rooms: Room, mut redis_con: redis::aio::MultiplexedConnection) {
    let ws_stream = accept_async(stream).await.expect("WebSocket handshake failed");
    println!("New WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut current_room: Option<String> = None;
    let (tx, mut rx) = mpsc::channel::<String>(100);

    while let Some(msg) = ws_receiver.next().await {
        if let Ok(Message::Text(text)) = msg {
            if text.starts_with("JOIN:") {
                let room_name = text.strip_prefix("JOIN:").unwrap().to_string();
                current_room = Some(room_name.clone());

                let mut rooms_lock = rooms.lock().await;
                rooms_lock.entry(room_name.clone()).or_insert_with(Vec::new).push(tx.clone());

                println!("User joined room: {}", room_name);
            } else if let Some(room) = &current_room {
                let _: () = redis_con.publish(room, &text).await.expect("Failed to publish to Redis");
            }
        }
    }
}

async fn subscribe_to_redis(redis_client: Client, rooms: Room) {
    let mut pubsub = redis_client.get_async_pubsub().await.expect("Failed to get pubsub");
    pubsub.subscribe("*").await.expect("Failed to subscribe");

    let mut pubsub_stream = pubsub.on_message();
    while let Some(msg) = pubsub_stream.next().await {
        let room_name = msg.get_channel_name().to_string();
        let payload: String = msg.get_payload().expect("Failed to get payload");

        let rooms_lock = rooms.lock().await;
        if let Some(clients) = rooms_lock.get(&room_name) {
            for client in clients {
                let _ = client.send(payload.clone()).await;
            }
        }
    }
}
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::usize;

use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
type Clients = Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>;

async fn handle_client(stream: TcpStream, clients: Clients) {
    let (tcp_read, mut tcp_write) = stream.into_split();
    let mut lines = BufReader::new(tcp_read).lines();
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // --- name negotiation ---
    tcp_write.write_all(b"Welcome! What's your name?\n").await.unwrap();
    let name = match lines.next_line().await.unwrap() {
        Some(n) if n.chars().all(|c| c.is_alphanumeric()) && !n.is_empty() => n,
        _ => {
            tcp_write.write_all(b"Invalid name.\n").await.unwrap();
            return; // drops stream, disconnects client
        }
    };

    // --- join: add to shared map, notify others ---
    {
        let mut map = clients.lock().await;
        let present: Vec<String> = map.keys().cloned().collect();
        tcp_write
            .write_all(format!("* The room contains: {}\n", present.join(", ")).as_bytes())
            .await.unwrap();
        broadcast(&map, &name, format!("* {} has entered the room\n", name)).await;
        map.insert(name.clone(), tx);
    }

    // --- main loop: select on tcp input OR inbound broadcast ---
    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(msg)) => {
                        let map = clients.lock().await;
                        broadcast(&map, &name, format!("[{}] {}\n", name, msg)).await;
                    }
                    _ => break, // client disconnected
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(m) => { tcp_write.write_all(m.as_bytes()).await.unwrap(); }
                    None => break,
                }
            }
        }
    }

    // --- leave: remove from map, notify others ---
    {
        let mut map = clients.lock().await;
        map.remove(&name);
        broadcast(&map, &name, format!("* {} has left the room\n", name)).await;
    }
}

async fn broadcast(map: &HashMap<String, mpsc::Sender<String>>, skip: &str, msg: String) {
    for (name, tx) in map.iter() {
        if name != skip {
            let _ = tx.send(msg.clone()).await;
        }
    }
}

const NEW_USER_MSG: &str = "Welcome to budgetchat! What shall I call you?";
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8888").await.unwrap();
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _addr) = listener.accept().await.unwrap();
        let clients = Arc::clone(&clients);
        tokio::spawn(async move {
            handle_client(socket, clients).await;
        });
    }
}
#[cfg(test)]
mod test {
    use crate::check_username;

    #[test]
    fn test_good_username() {
        let name = "Ben123";
        assert!(check_username(name))
    }
    #[test]
    fn test_no_username() {
        let name = "";
        assert!(!check_username(name))
    }
    #[test]
    fn test_bad_char() {
        let name = "$Ben123";
        assert!(!check_username(name))
    }
}

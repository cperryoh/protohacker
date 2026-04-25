use std::fmt::write;
use std::i64;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf};
use tokio::net::TcpListener;
use tokio::net::tcp::OwnedWriteHalf;
#[derive(Deserialize, Debug)]
struct Request {
    method: String,
    number: f64,
}
#[derive(Serialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}
fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut divisor: i64 = 3;
    while divisor * divisor <= n {
        if n % divisor == 0 {
            return false;
        }
        divisor += 2;
    }
    return true;
}
const res_bytes: &[u8] = "{\"method\":\"asdfasd\"}".as_bytes();
async fn respond_with_bad(writer: &mut OwnedWriteHalf) {
    writer
        .write_all(res_bytes)
        .await
        .expect("Failed to respond with bad object");
}
fn increment_time(counter: &Arc<Mutex<(u64, u128)>>, start_time: Instant) {
    let duration = Instant::now() - start_time;
    let mut time = counter.lock().expect("Failed to get lock");
    time.0 += 1;
    time.1 += duration.as_micros();
    let res = time.1 as f64 / time.0 as f64;
    let res = res.round() as u64;
    println!("Avg time: {}", res);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:8888").await?;
    let miliseconds = Arc::new(Mutex::new((0u64, 0u128)));
    let biggest_number_mutex = Arc::new(Mutex::new(0u64));

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Accepted connection from {addr}");
        let counter_arc = Arc::clone(&miliseconds);
        let biggest_number_arc = Arc::clone(&biggest_number_mutex);

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                let start_time = Instant::now();
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // Client disconnected
                    Ok(_) => {
                        println!("Got: {}", line);
                        let Ok(object) = serde_json::from_str::<Request>(line.as_str()) else {
                            respond_with_bad(&mut writer).await;
                            println!("Failed to parse to object, sending bad data");
                            break;
                        };
                        if object.method != "isPrime" {
                            respond_with_bad(&mut writer).await;
                            println!("Bad method, sending bad data back");
                            break;
                        }
                        let  prime=  if object.number.fract() != 0.0 || object.number > i64::MAX as f64 || object.number < i64::MIN as f64 {
                            false
                        } else {
                            {
                                let mut biggest_number =
                                    biggest_number_arc.lock().expect("Failed to get lock");
                                let big_num = *biggest_number;
                                let number = (object.number as i64).abs() as u64;
                                *biggest_number = if big_num > number { big_num } else { number };
                                println!("Biggest number: {}",*biggest_number);
                            }
                            is_prime(object.number as i64)
                        };
                        let res = Response {
                            method: "isPrime".into(),
                            prime: prime,
                        };
                        let res = format!(
                            "{}\n",
                            serde_json::to_string(&res).expect("Failed to make json")
                        );
                        println!("Sending back: {res}");
                        writer
                            .write_all(res.as_bytes())
                            .await
                            .expect("Failed to respond with good response");
                    }
                    Err(_) => break,
                }
                increment_time(&counter_arc, start_time);
            }
            println!("Connection closed: {addr}");
        });
    }
}

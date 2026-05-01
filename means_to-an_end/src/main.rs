use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
#[derive(Debug)]
struct PriceLog {
    timestamp: i32,
    price: i32,
}
fn insert_new_item(list: &mut Vec<PriceLog>, new_item: PriceLog) {
    let pos = list
        .binary_search_by(|a| a.timestamp.cmp(&new_item.timestamp))
        .unwrap_or_else(|i| i);
    list.insert(pos, new_item);
}
fn run_query(list: &[PriceLog], start_t: &i32, end_t: &i32) -> i32 {
    let start_pos = list
        .binary_search_by(|a| a.timestamp.cmp(&start_t))
        .unwrap_or_else(|i| i);
    let end_pos= list
        .binary_search_by(|a| a.timestamp.cmp(&end_t))
        .map(|i| i + 1)
        .unwrap_or_else(|i| i);
    let mut sum:i64 = 0;
    let cnt: i64 = (end_pos - start_pos) as i64;
    if cnt==0{
        return 0;
    }
    println!("{start_pos}..{end_pos}");
    for i in start_pos..end_pos {
        let log = &list[i];
        sum += log.price as i64;
    }
    println!("{sum}/{cnt}");
    (sum / cnt)as i32
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:8888").await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Accepted connection from {addr}");

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut buf = [0u8; 9];
            let mut list: Vec<PriceLog> = Vec::new();

            loop {
                match reader.read_exact(&mut buf).await {
                    Ok(0) => break, // Client disconnected
                    Ok(_) => {
                        let query_type: char = buf[0] as char;
                        let num1 =
                            i32::from_be_bytes(buf[1..5].try_into().expect("Failed to parse num1"));
                        let num2 =
                            i32::from_be_bytes(buf[5..9].try_into().expect("Failed to parse num2"));
                        match query_type {
                            'I' => {
                                let new_log = PriceLog {
                                    timestamp: num1,
                                    price: num2,
                                };
                                insert_new_item(&mut list, new_log);
                            }
                            'Q' => {
                                let res = run_query(&list, &num1, &num2);
                                let out = res.to_be_bytes();
                                println!("Avg: {res}");
                                writer.write_all(&out).await.expect("Failed to write bytes");
                            }
                            _ => {}
                        }
                        println!("Query: {query_type} Num1: {num1} Num2: {num2}");
                    }
                    Err(_) => break,
                }
            }
            println!("Connection closed: {addr}");
        });
    }
}

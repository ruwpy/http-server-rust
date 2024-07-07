use std::{
    fs::File,
    io::{BufRead, Read, Write},
    net::{TcpListener, TcpStream},
};

use itertools::Itertools;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream),
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8(buffer.into()).unwrap();
    let lines = request.lines().collect_vec();

    let start_line = lines.get(0).unwrap();

    let (method, uri, http_version) = start_line.split(" ").next_tuple().unwrap();

    let (status_line, contents) = if method.eq("GET") & uri.starts_with("/echo") {
        let message = uri.split("/").nth(2).unwrap();

        ("HTTP/1.1 200 OK\r\n", message)
    } else {
        ("HTTP/1.1 404 Not Found\r\n", "Not Found")
    };

    let content_type_header = "Content-Type: text/plain";
    let content_length_header = format!("Content-Length: {}", contents.len());

    let response = format!(
        "{}{}\r\n{}",
        status_line,
        format!("{}\r\n{}\r\n", content_type_header, content_length_header),
        contents
    );

    println!("{}", &response);

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

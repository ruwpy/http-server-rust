use std::{
    collections::HashMap,
    io::{Read, Write},
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
    let mut buffer = [0; 2048];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8(buffer.into()).unwrap();

    println!("{}", request);

    let mut start_line = String::new();
    let mut headers: HashMap<String, String> = HashMap::new();

    for (index, line) in request.lines().enumerate() {
        if index == 0 {
            start_line = line.to_string();
        };

        if index >= 1 {
            if line.is_empty() {
                break;
            }

            println!("{}\r\n", line);

            let (key, value) = line.split(": ").next_tuple().unwrap();

            headers.insert(key.to_string(), value.to_string());
        }
    }

    let (method, uri, http_version) = start_line.split(" ").next_tuple().unwrap();

    if method.eq("GET") {
        let (status_line, contents) = if uri.starts_with("/echo") {
            let message = uri.split("/").nth(2).unwrap();

            ("HTTP/1.1 200 OK\r\n", message)
        } else if uri.eq("/") {
            ("HTTP/1.1 200 OK\r\n", "Hello, World!")
        } else if uri.eq("/user-agent") {
            let user_agent_header = headers.get("User-Agent").unwrap().as_str();

            ("HTTP/1.1 200 OK\r\n", user_agent_header)
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

        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use itertools::Itertools;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| handle_connection(stream));
            }
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

    let mut headers: HashMap<String, String> = HashMap::new();

    let (start_line, request_details) = request.split_once("\r\n").unwrap();
    let (headers_data, _) = request_details.split("\r\n\r\n").next_tuple().unwrap();

    for line in headers_data.lines() {
        let (key, value) = line.split(": ").next_tuple().unwrap();

        headers.insert(key.to_string(), value.to_string());
    }

    let (method, uri, _) = start_line.split(" ").next_tuple().unwrap();

    if method.eq("GET") {
        let (status_line, contents, content_type, content_length) = match uri {
            user_agent if user_agent == "/user-agent" => {
                let user_agent_header = headers.get("User-Agent").unwrap().as_str();

                (
                    "HTTP/1.1 200 OK\r\n",
                    user_agent_header,
                    "text/plain",
                    user_agent_header.len(),
                )
            }
            echo if echo.starts_with("/echo/") => {
                let message = uri.split("/").nth(2).unwrap();

                ("HTTP/1.1 200 OK\r\n", message, "text/plain", message.len())
            }
            file if file.starts_with("/files/") => {
                let filename = uri.split("/").nth(2).unwrap();

                let env_args: Vec<String> = env::args().collect();
                let mut dir = env_args[2].clone();
                dir.push_str(filename);

                let file = fs::read(dir);

                match file {
                    Ok(f) => (
                        "HTTP/1.1 200 OK\r\n",
                        "",
                        "application/octet-stream",
                        f.len(),
                    ),
                    Err(..) => ("HTTP/1.1 404 Not Found\r\n", "Not Found", "text/plain", 9),
                }
            }
            index if index == ("/") => ("HTTP/1.1 200 OK\r\n", "Hello, World!", "text/plain", 13),
            _ => ("HTTP/1.1 404 Not Found\r\n", "Not Found", "text/plain", 9),
        };

        let content_type_header = format!("Content-Type: {}", content_type);
        let content_length_header = format!("Content-Length: {}", content_length);

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

use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use itertools::Itertools;

enum ContentType {
    PlainText,
    OctetStream,
}

struct Response {
    status_line: String,
    data: String,
    content_type: String,
    content_length: u16,
}

impl Response {
    fn format_to_string(self) -> String {
        let headers = HashMap::from([
            ("Content-Type", self.content_type),
            ("Content-Length", self.content_length.to_string()),
        ]);

        let headers_str = headers
            .into_iter()
            .map(|header| {
                let (k, v) = header;

                return format!("{}: {}", k, v);
            })
            .join("\r\n");

        let formatted_response = format!(
            "{}\r\n{}\r\n\r\n{}",
            self.status_line, headers_str, self.data
        );

        formatted_response
    }
}

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
        let response: Response = match uri {
            user_agent if user_agent == "/user-agent" => {
                let user_agent_header = headers.get("User-Agent").unwrap().as_str();

                create_response(200, user_agent_header.to_string(), ContentType::PlainText)
            }
            echo if echo.starts_with("/echo/") => {
                let message = uri.split("/").nth(2).unwrap();

                create_response(200, message.to_string(), ContentType::PlainText)
            }
            file if file.starts_with("/files/") => {
                let filename = uri.split("/").nth(2).unwrap();

                let env_args: Vec<String> = env::args().collect();
                let mut dir = env_args[2].clone();
                dir.push_str(filename);

                let file = fs::read_to_string(dir);

                match file {
                    Ok(f) => create_response(200, f, ContentType::OctetStream),
                    Err(_) => create_response(404, "Not Found".to_string(), ContentType::PlainText),
                }
            }
            index if index == ("/") => {
                create_response(200, "Hello, World!".to_string(), ContentType::PlainText)
            }
            _ => create_response(404, "Not Found".to_string(), ContentType::PlainText),
        };

        stream
            .write(response.format_to_string().as_bytes())
            .unwrap();
        stream.flush().unwrap();
    }
}

fn create_response(status_code: u16, data: String, content_type: ContentType) -> Response {
    let status_line = match status_code {
        200 => "HTTP/1.1 200 OK",
        404 => "HTTP/1.1 404 Not Found",
        _ => "HTTP/1.1 400 Bad Request",
    };

    let content_type_str = match content_type {
        ContentType::OctetStream => "application/octet-stream",
        ContentType::PlainText => "text/plain",
    };

    Response {
        status_line: status_line.to_string(),
        content_length: data.len() as u16,
        content_type: content_type_str.to_string(),
        data,
    }
}

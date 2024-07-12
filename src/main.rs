use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
};

use flate2::{write::GzEncoder, Compression};
use itertools::Itertools;

enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

struct Header {
    name: String,
    value: String,
}

struct Response {
    status_line: String,
    headers: Vec<Header>,
    data: String,
}

impl RequestMethod {
    fn from_str(input: &str) -> Result<Self, ()> {
        match input {
            "GET" => Ok(Self::GET),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "DELETE" => Ok(Self::DELETE),
            "PATCH" => Ok(Self::PATCH),
            _ => Err(()),
        }
    }
}

impl Header {
    fn new(name: String, value: String) -> Header {
        Header { name, value }
    }

    fn format_to_string(self) -> String {
        format!("{}: {}", self.name, self.value)
    }
}

impl Response {
    fn format_to_string(self) -> String {
        let headers_str = self
            .headers
            .into_iter()
            .map(|h| h.format_to_string())
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

    let (start_line, request_details) = request.split_once("\r\n").expect("Invalid request");
    let (headers_data, body) = request_details.split("\r\n\r\n").next_tuple().unwrap();

    for line in headers_data.lines() {
        let (key, value) = line.split(": ").next_tuple().unwrap();

        headers.insert(
            key.to_string().to_ascii_lowercase(),
            value.to_string().to_ascii_lowercase(),
        );
    }

    let (method, uri, _) = start_line.split(" ").next_tuple().unwrap();

    let method = RequestMethod::from_str(method).expect("Method Not Allowed");

    let response: Response = match uri {
        user_agent if user_agent == "/user-agent" => match method {
            RequestMethod::GET => {
                let user_agent_header = headers.get("user-agent").unwrap().as_str();

                create_response(headers.clone(), 200, user_agent_header.to_string(), None)
            }
            _ => create_response(headers, 405, "Method Not Allowed".to_string(), None),
        },
        echo if echo.starts_with("/echo/") => match method {
            RequestMethod::GET => {
                let message = uri.split("/").nth(2).unwrap();

                create_response(headers, 200, message.to_string(), None)
            }
            _ => create_response(headers, 405, "Method Not Allowed".to_string(), None),
        },
        file if file.starts_with("/files/") => match method {
            RequestMethod::GET => {
                let filename = uri.split("/").nth(2).unwrap();

                let env_args: Vec<String> = env::args().collect();
                let dir = env_args[2].clone();

                let mut path = PathBuf::from(dir);
                path.push(filename);

                let file = fs::read_to_string(path);

                match file {
                    Ok(f) => create_response(
                        headers,
                        200,
                        f,
                        Some(Vec::from([Header::new(
                            "Content-Type".to_string(),
                            "application/octet-stream".to_string(),
                        )])),
                    ),
                    Err(_) => create_response(headers, 404, "Not Found".to_string(), None),
                }
            }
            RequestMethod::POST => {
                let filename = uri.split("/").nth(2).unwrap();

                let env_args: Vec<String> = env::args().collect();
                let dir = env_args[2].clone();

                let mut path = PathBuf::from(dir);
                path.push(filename);

                let file = fs::write(path, body.trim_matches(char::from(0)).to_string());

                match file {
                    Ok(_) => create_response(headers, 201, "Created".to_string(), None),
                    Err(e) => create_response(headers, 500, e.to_string(), None),
                }
            }
            _ => create_response(headers, 405, "Method Not Allowed".to_string(), None),
        },
        index if index == ("/") => match method {
            RequestMethod::GET => create_response(headers, 200, "Hello, World!".to_string(), None),
            _ => create_response(headers, 405, "Method Not Allowed".to_string(), None),
        },
        _ => create_response(
            headers,
            404,
            "Not Found".to_string(),
            Some(Vec::from([Header::new(
                "Content-Type".to_string(),
                "text/plain".to_string(),
            )])),
        ),
    };

    println!("{}", response.format_to_string());

    // stream
    //     .write(response.format_to_string().as_bytes())
    //     .unwrap();
    // stream.flush().unwrap();
}

fn create_response(
    request_headers: HashMap<String, String>,
    status_code: u16,
    mut data: String,
    headers: Option<Vec<Header>>,
) -> Response {
    let mut data_len = data.len().to_string();

    let status_line = match status_code {
        200 => "HTTP/1.1 200 OK",
        201 => "HTTP/1.1 201 Created",
        404 => "HTTP/1.1 404 Not Found",
        405 => "HTTP/1.1 405 Method Not Allowed",
        500 => "HTTP/1.1 500 Internal Server Error",
        _ => "HTTP/1.1 400 Bad Request",
    };

    let mut new_headers = match headers {
        Some(headers) => headers,
        None => Vec::from([Header::new(
            "Content-Type".to_string(),
            "text/plain".to_string(),
        )]),
    };

    if request_headers.contains_key("accept-encoding") {
        let accepted_algorithms: Vec<&str> = request_headers
            .get("accept-encoding")
            .unwrap()
            .split(", ")
            .collect();

        if accepted_algorithms.contains(&"gzip") {
            new_headers.push(Header::new(
                "Content-Encoding".to_string(),
                "gzip".to_string(),
            ));

            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data.as_bytes()).unwrap();

            let encoded_data = encoder.finish().unwrap();

            data = to_hex_string(encoded_data.clone());
            data_len = encoded_data.len().to_string();
        }
    }

    new_headers.push(Header::new("Content-Length".to_string(), data_len));

    Response {
        status_line: status_line.to_string(),
        headers: new_headers,
        data,
    }
}

fn to_hex_string(bytes: Vec<u8>) -> String {
    let hex_chars: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    hex_chars.join(" ")
}

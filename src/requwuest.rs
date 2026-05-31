use crate::router::Router;

use std::io::{Read, Write};
use std::net::TcpStream;

pub fn http_get(router: &Router) -> String
{
    let mut stream = match TcpStream::connect(format!("{}:80", &router.base))
    {
        Ok(tcp_stream) => { tcp_stream }
        Err(_) => { return String::from("<whoopsies dayzeyehes<") }
    };

    let request = format!("GET /{}{} ",
                                &router.path,
                                  if &router.path == "" { "" } else { "/?raw=true" }
                         )
                + "HTTP/1.1\r\n"
                + &format!("Host: {}\r\n", &router.base)
                + "Connection: close\r\n"
                + "User-Agent: anipure\r\n\r\n";

    stream.write_all(request.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

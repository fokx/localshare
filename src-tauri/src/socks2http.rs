use reqwest::Method;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const REQ_HOST: &str = "xjtu.app";
const REQ_PORT: u16 = 443;

pub async fn main() {
    warn!("socks2http bind 4802");
    let listener = TcpListener::bind("127.0.0.1:4802").await.unwrap();
    warn!("socks2http bind 4802 finished");
    let socks5_url = reqwest::Url::parse("socks5h://127.0.0.1:4807").unwrap();

    // Build the client with SOCKS5 proxy
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(socks5_url).unwrap())
        .cookie_store(true)
        .build()
        .unwrap();

    while let Ok((mut inbound, client_addr)) = listener.accept().await {
        let client = client.clone();

        tokio::spawn(async move {
            let mut buf = [0; 1024 * 8];
            match inbound.read(&mut buf).await {
                Ok(read_bytes) if read_bytes > 0 => {
                    let request_data = &buf[..read_bytes];
                    let mut headers = [httparse::EMPTY_HEADER; 16];
                    let mut req = httparse::Request::new(&mut headers);

                    if let Ok(httparse::Status::Complete(_)) = req.parse(request_data) {
                        if let Some(path) = req.path {
                            if let Some(method) = req.method {
                                if method == "CONNECT" {
                                    // Handle HTTPS requests
                                    if inbound
                                        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                                        .await
                                        .is_ok()
                                    {
                                        handle_https_tunneling(inbound, path).await;
                                    }
                                } else {
                                    // Handle HTTP requests
                                    handle_http_request(client, inbound, method, path).await;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });
    }
}

async fn handle_https_tunneling(mut inbound: TcpStream, host: &str) {
    if let Ok(outbound) = TcpStream::connect("127.0.0.1:4807").await {
        let mut outbound = io::BufStream::new(outbound);

        // Connect the Socks5 proxy to the destination host
        if let Ok(_) = async_socks5::connect(&mut outbound, (host, REQ_PORT), None).await {
            let (mut ri, mut wi) = inbound.split();
            let (mut ro, mut wo) = outbound.get_mut().split();

            let client_to_server = io::copy(&mut ri, &mut wo);
            let server_to_client = io::copy(&mut ro, &mut wi);

            tokio::select! {
                _ = client_to_server => {}
                _ = server_to_client => {}
            }
        }
    }
}

async fn handle_http_request(
    client: reqwest::Client,
    mut inbound: TcpStream,
    method: &str,
    path: &str,
) {
    let req_url = format!("https://{}{}", REQ_HOST, path);

    match client
        .request(method.parse().unwrap_or(Method::GET), req_url)
        .send()
        .await
    {
        Ok(response) => {
            let status = format!(
                "HTTP/1.1 {} {}\r\n",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("")
            );
            let headers = response
                .headers()
                .iter()
                .map(|(key, value)| format!("{}: {}\r\n", key, value.to_str().unwrap_or("")))
                .collect::<String>();

            if inbound
                .write_all(format!("{}{}\r\n", status, headers).as_bytes())
                .await
                .is_ok()
            {
                if let Ok(body_bytes) = response.bytes().await {
                    let _ = inbound.write_all(&body_bytes).await;
                }
            }
        }
        Err(_) => {
            let _ = inbound
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
                .await;
        }
    }

    let _ = inbound.flush().await;
}

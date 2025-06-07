use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use reqwest::Method;

const REQ_HOST: &str = "xjtu.app";
const REQ_PORT: u16 = 443;
async fn socks2http() {
    let listener = TcpListener::bind(format!("127.0.0.1:{:?}", 4802))
            .await
            .unwrap();
    let socks5_url = reqwest::Url::parse(
        &*format!("socks5h://127.0.0.1:{:?}", 4801).to_string(),
    )
            .unwrap();
    let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(socks5_url).unwrap())
            .cookie_store(true)
            .use_rustls_tls()
            .tls_sni(true)
            .tls_info(true)
            .build()
            .unwrap();
    loop {
        let client = client.clone();
        let (mut inbound, addr) = listener.accept().await.unwrap();
        println!("NEW CLIENT: {}", addr);

        tokio::spawn(async move {
            let mut buf = [0; 1024 * 8];
            let Ok(downstream_read_bytes_size) = inbound.read(&mut buf).await else {
                return;
            };
            let bytes_from_downstream = &buf[0..downstream_read_bytes_size];

            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Request::new(&mut headers);
            let Ok(parse_result) = req.parse(bytes_from_downstream) else {
                return;
            };
            if parse_result.is_complete() {
                if let Some(valid_req_path) = req.path {
                    println!("get request: {}", valid_req_path);

                    let outbound = TcpStream::connect(format!("127.0.0.1:{:?}", 4801))
                            .await
                            .unwrap();
                    println!("forwarding to socks5 proxy at port {}", 4801);
                    let mut outbound = io::BufStream::new(outbound);
                    async_socks5::connect(&mut outbound, (REQ_HOST, REQ_PORT), None)
                            .await
                            .unwrap();
                    println!("proxy server connected to {}", REQ_HOST);
                    dbg!(req.method.unwrap());
                    if req.method.unwrap() == Method::CONNECT { // HTTPS proxy use CONNECT command
                        inbound
                                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                                .await
                                .unwrap();
                        let (mut ri, mut wi) = inbound.split();
                        let (mut ro, mut wo) = outbound.get_mut().split();

                        let client_to_server = async {
                            io::copy(&mut ri, &mut wo)
                                    .await
                                    .expect("Transport endpoint is not connected");
                            wo.shutdown().await
                        };

                        let server_to_client = async {
                            let _ = io::copy(&mut ro, &mut wi).await;
                            wi.shutdown().await
                        };
                        println!("try join");
                        let _ = futures::future::try_join(client_to_server, server_to_client).await;
                    } else {

                        let req_url = format!("https://{}{}", REQ_HOST, valid_req_path);
                        println!("reqwest client built with SOCKS5 to {}", req_url);
                        // let response = client.get(req_url).send().await.unwrap();
                        let response = client.request(Method::GET, req_url).send().await.unwrap();
                        // let response = client.request(Method::GET,"https://myip.xjtu.app").send().await.unwrap();
                        // dbg!(response.version());
                        // dbg!(response.text().await.unwrap());

                        // let headers = response.headers();
                        // let body_text =response.text().await.unwrap();

                        inbound
                                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                                .await
                                .unwrap();
                        let response_bytes = response.bytes().await.unwrap();
                        let _ = inbound.write(&response_bytes).await;
                        inbound.flush().await.unwrap();

                        // Ok(hyper::Response::new(hyper::Body::from(body_text)))

                        // // Method = GET ...
                        // let upstream_write_bytes_size =
                        //     outbound.write(bytes_from_downstream).await.unwrap();
                        // assert_eq!(upstream_write_bytes_size, downstream_read_bytes_size);
                        //
                        // let (mut ri, mut wi) = inbound.split();
                        // let (mut ro, mut wo) = outbound.get_mut().split();
                        //
                        // io::copy(&mut ro, &mut wi)
                        //     .await.expect("Transport endpoint is not connected");
                        // wi.shutdown().await;
                        // wo.shutdown().await;
                    }
                }
            }
        });
    }
}

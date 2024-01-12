use std::env;
use std::io::Read;
use std::str::FromStr;

use axum::body::Bytes;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::routing::any;
use axum::Router;
use flate2::read::GzDecoder;
use reqwest::Client;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let log_requests = env::var("REED_PROXY_REQUESTS")
        .map(|v| v == "true")
        .unwrap_or(false);

    let client = Client::new();

    let app = Router::new().route(
        "/kobo-storeapi/*path",
        any(
            move |method: Method, Path(path): Path<String>, headers: HeaderMap, body: Bytes| async move {
                println!("{method} {path}");

                let method = match method {
                    Method::GET => reqwest::Method::GET,
                    Method::POST => reqwest::Method::POST,
                    Method::PATCH => reqwest::Method::PATCH,
                    Method::PUT => reqwest::Method::PUT,
                    Method::DELETE => reqwest::Method::DELETE,
                    Method::HEAD => reqwest::Method::HEAD,
                    Method::OPTIONS => reqwest::Method::OPTIONS,
                    _ => panic!("unsupported request method {method}"),
                };

                let mut proxy_headers = reqwest::header::HeaderMap::new();
                for (name, value) in &headers {
                    if name != "host"
                        && name != "connection"
                        && name != "content-length"
                        && name != "transfer-encoding"
                    {
                        proxy_headers.append(
                            reqwest::header::HeaderName::from_str(name.as_str()).unwrap(),
                            reqwest::header::HeaderValue::from_bytes(value.as_bytes()).unwrap(),
                        );
                    }
                }

                let proxy_url = format!("https://storeapi.kobo.com/{path}");
                if log_requests {
                    println!("proxying to {proxy_url}");
                    println!("{proxy_headers:?}");
                }

                let res = client
                    .request(method, proxy_url)
                    .headers(proxy_headers)
                    .body(body)
                    .send()
                    .await
                    .unwrap();

                let status = StatusCode::from_u16(res.status().as_u16()).unwrap();

                let mut res_headers = HeaderMap::new();
                for (name, value) in res.headers() {
                    if name != "connection"
                        && name != "content-length"
                        && name != "transfer-encoding"
                    {
                        res_headers.append(
                            HeaderName::from_str(name.as_str()).unwrap(),
                            HeaderValue::from_bytes(value.as_bytes()).unwrap(),
                        );
                    }
                }

                let res_body = res.bytes().await.unwrap();

                let mut decoder = GzDecoder::new(&*res_body);
                let mut res_text = String::new();
                decoder.read_to_string(&mut res_text).unwrap();

                if log_requests {
                    println!("got status {status:?}");
                    println!("{res_headers:?}");
                    println!("{res_text}");
                    println!();
                }

                (status, res_headers, res_body)
            },
        ),
    );

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

use hyper::{Request, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use http_body_util::Empty;
use futures_util::{TryFutureExt, TryStreamExt};
use hyper_proxy::{Proxy, ProxyConnector, Intercept};
use headers::Authorization;
use std::error::Error;
use tokio::io::{stdout, AsyncWriteExt as _};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let proxy = {
        let proxy_uri = "http://my-proxy:8080".parse().unwrap();
        let mut proxy = Proxy::new(Intercept::All, proxy_uri);
        proxy.set_authorization(Authorization::basic("John Doe", "Agent1234"));
        let connector = HttpConnector::new();
        #[cfg(not(any(feature = "tls", feature = "rustls-base", feature = "openssl-tls")))]
        let proxy_connector = ProxyConnector::from_proxy_unsecured(connector, proxy);
        #[cfg(any(feature = "tls", feature = "rustls-base", feature = "openssl"))]
        let proxy_connector = ProxyConnector::from_proxy(connector, proxy).unwrap();
        proxy_connector
    };

    // Connecting to http will trigger regular GETs and POSTs.
    // We need to manually append the relevant headers to the request
    let uri: Uri = "http://my-remote-website.com".parse().unwrap();
    let mut req = Request::get(uri.clone()).body(Empty::default()).unwrap();

    if let Some(headers) = proxy.http_headers(&uri) {
        req.headers_mut().extend(headers.clone().into_iter());
    }

    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(proxy);
    let mut resp = client.request(req).await?;
    println!("Response: {}", resp.status());
    while let Some(chunk) = resp.body_mut().data().await {
        stdout().write_all(&chunk?).await?;
    }

    // Connecting to an https uri is straightforward (uses 'CONNECT' method underneath)
    let uri = "https://my-remote-websitei-secured.com".parse().unwrap();
    let mut resp = client.get(uri).await?;
    println!("Response: {}", resp.status());
    while let Some(chunk) = resp.body_mut().data().await {
        stdout().write_all(&chunk?).await?;
    }

    Ok(())
}

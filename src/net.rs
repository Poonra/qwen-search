use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn is_online() -> bool {
    matches!(
        timeout(
            Duration::from_secs(2),
            TcpStream::connect("html.duckduckgo.com:443")
        )
        .await,
        Ok(Ok(_))
    )
}

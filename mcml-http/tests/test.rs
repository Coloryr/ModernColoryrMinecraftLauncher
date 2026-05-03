#[cfg(test)]
mod tests {
    use mcml_net::{Client, NetResult};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestResponse {
        url: String,
        #[serde(rename = "origin")]
        _origin: String,
    }

    #[derive(Debug, Serialize)]
    struct PostBody {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_get() {
        let client = Client::new();
        let result: NetResult<TestResponse> =
            client.get("https://httpbin.org/get").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post() {
        let client = Client::new();
        let body = PostBody {
            name: "test".into(),
            value: 42,
        };
        let result: NetResult<serde_json::Value> =
            client.post("https://httpbin.org/post", &body).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_text() {
        let client = Client::new();
        let result = client.get_text("https://httpbin.org/get").await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_bytes() {
        let client = Client::new();
        let result = client.get_bytes("https://httpbin.org/get").await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_post_form() {
        let client = Client::new();
        let result: NetResult<serde_json::Value> = client
            .post_form("https://httpbin.org/post", &[("key", "value")])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_custom_client() {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("mcml-test/1.0")
            .build();
        let result: NetResult<serde_json::Value> =
            client.get("https://httpbin.org/get").await;
        assert!(result.is_ok());
    }
}

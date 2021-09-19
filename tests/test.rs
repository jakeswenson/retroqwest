use reqwest::ClientBuilder;
use retroqwest::RetroqwestError;

use serde::{Deserialize, Serialize};
use wiremock::matchers::{body_json, query_param};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[derive(Deserialize, Serialize, Clone)]
pub struct HttpBinResponse {
    pub url: String,
}

#[retroqwest::retroqwest]
pub trait HttpBin {
    #[http::get("/anything")]
    async fn get_anything(&self) -> Result<HttpBinResponse, RetroqwestError>;

    #[http::get("/anything/{name}")]
    async fn get_by_name(&self, name: String) -> Result<HttpBinResponse, RetroqwestError>;

    #[http::post("/anything/{name}")]
    async fn post_to_name(
        &self,
        name: String,
        #[query] q: bool,
        #[json] body: &HttpBinResponse,
    ) -> Result<HttpBinResponse, RetroqwestError>;

    #[http::get("/anything")]
    async fn get_multiple_queries(
        &self,
        #[query] q: bool,
        #[query] q2: i32,
    ) -> Result<HttpBinResponse, RetroqwestError>;
}

impl HttpBinClient {
    pub fn new(base_uri: String) -> Result<Self, RetroqwestError> {
        Self::from_builder(base_uri, ClientBuilder::default())
    }
}

// This method allows for better code completion
// since `impl HttpBin` is better than the generated struct...
fn build_client(uri: String) -> Result<impl HttpBin, retroqwest::RetroqwestError> {
    Ok(HttpBinClient::new(uri)?)
}

#[tokio::test]
async fn test_simple_gets() -> Result<(), Box<dyn std::error::Error>> {
    let server = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/anything"))
        .respond_with(ResponseTemplate::new(200).set_body_json(HttpBinResponse {
            url: "test".to_string(),
        }))
        .mount(&server)
        .await;

    let client = build_client(server.uri())?;
    let result: HttpBinResponse = client.get_anything().await?;

    assert_eq!(result.url, "test".to_string());
    Ok(())
}

#[tokio::test]
async fn test_gets_with_vars_in_path() -> Result<(), Box<dyn std::error::Error>> {
    let server = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/anything/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(HttpBinResponse {
            url: "test".to_string(),
        }))
        .mount(&server)
        .await;

    let client = build_client(server.uri())?;

    let result: HttpBinResponse = client.get_by_name("test".to_string()).await?;

    assert_eq!(result.url, "test".to_string());

    Ok(())
}

#[tokio::test]
async fn test_complex_post() -> Result<(), Box<dyn std::error::Error>> {
    let server = wiremock::MockServer::start().await;

    let body = HttpBinResponse {
        url: "test".to_string(),
    };

    Mock::given(method("POST"))
        .and(path("/anything/test"))
        .and(query_param("q", "true"))
        .and(body_json(body.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(HttpBinResponse {
            url: "posted".to_string(),
        }))
        .mount(&server)
        .await;

    let client = build_client(server.uri())?;

    let result: HttpBinResponse = client.post_to_name("test".to_string(), true, &body).await?;

    assert_eq!(result.url, "posted".to_string());

    Ok(())
}

#[tokio::test]
async fn test_multiple_query_params() -> Result<(), Box<dyn std::error::Error>> {
  let server = wiremock::MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/anything"))
    .and(query_param("q", "true"))
    .and(query_param("q2", "3"))
    .respond_with(ResponseTemplate::new(200).set_body_json(HttpBinResponse {
      url: "multi-queries".to_string(),
    }))
    .mount(&server)
    .await;


  let client = build_client(server.uri())?;

  let r = client.get_multiple_queries(true, 3).await;

  if r.is_err() {
    dbg!(server.received_requests().await);
  }

  let result: HttpBinResponse = r?;

  assert_eq!(result.url, "multi-queries".to_string());

  Ok(())
}

#[tokio::test]
async fn test_get_errors() -> Result<(), Box<dyn std::error::Error>> {
    let server = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/anything/test"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    let client = build_client(server.uri())?;

    let result = client
        .get_by_name("test".to_string())
        .await
        .err()
        .unwrap();

    assert!(matches!(result, RetroqwestError::ResponseError {..}));

    Ok(())
}

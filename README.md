# Retroqwest

[Retrofit]: https://square.github.io/retrofit/
[Refit]: https://github.com/reactiveui/refit

> This project is still a work in progress!!

A Rust proc-macro attribute HTTP Client generator from a trait.
Inspired by [Retrofit] and [Refit] to bring something like them to rust.

## Example

```rust
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
```

> See [tests](tests/test.rs) for a full example

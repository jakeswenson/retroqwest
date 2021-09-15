# Retroqwest

[Retrofit]: https://square.github.io/retrofit/
[Refit]: https://github.com/reactiveui/refit

> This project is still a work in progress!!

A Rust proc-macro attribute HTTP Client generator from a trait.
Inspired by [Retrofit] and [Refit] to bring something like them to rust.

## Example

```rust
#[retroqwest::retroqwest]
pub trait HttpBin {
  #[get::json("/anything")]
  async fn get_anything(&self) -> Result<HttpBinResponse, RetroqwestError>;

  #[get::json("/anything/{name}")]
  async fn get_by_name(&self, name: String) -> Result<HttpBinResponse, RetroqwestError>;
}
```

> See [tests](tests/test.rs) for a full example

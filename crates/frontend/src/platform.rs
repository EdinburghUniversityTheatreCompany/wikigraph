#![cfg(target_arch = "wasm32")]

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, Request, Response, window};

#[derive(Debug)]
pub struct Error(JsValue);
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn from_str(s: &str) -> Error {
        Error(JsValue::from_str(s))
    }
}

impl From<Error> for JsValue {
    fn from(value: Error) -> Self {
        value.0
    }
}

pub async fn fetch_body(url: &str) -> Result<String> {
    let window = window().ok_or_else(|| Error::from_str("no window"))?;

    let request =
        Request::new_with_str(url).map_err(|_| Error::from_str("request creation failed"))?;
    let resp: Response = window
        .fetch_with_request(&request)
        .await
        .map_err(|_| Error::from_str("request waiting failed"))?
        .dyn_into()
        .map_err(|_| Error::from_str("failed to cast to Response"))?;

    let text: String = resp
        .text()
        .map_err(|_| Error::from_str("text() failed"))?
        .await
        .map_err(|_| Error::from_str("text waiting failed"))?
        .as_string()
        .ok_or_else(|| Error::from_str("failed to cast to string"))?;

    Ok(text)
}

pub fn open_link(url: &str) -> Result<()> {
    let window = window().ok_or_else(|| Error::from_str("no window"))?;

    window
        .open_with_url(url)
        .map_err(|_| Error::from_str("could not open link"))?;

    Ok(())
}

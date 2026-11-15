#![cfg(feature = "net")]
pub mod http;

mod http_request;
mod https;
mod web_socket;

pub use http::{Client, ClientConfig, ClientRequest, ClientResponse, Url};
pub use http::{Method, Request, Response, Server, ServerConfig, StaticFiles, Status};

// The old API is kept for now so existing code keeps building.
#[deprecated(note = "use http::Request")]
pub use http_request::HTTPRequest;
#[deprecated(note = "use http::Method")]
pub use http_request::RequestType;
#[deprecated(note = "use http::Response and http::StaticFiles")]
pub use https::HTTPS;

pub use web_socket::MessageDataType;
pub use web_socket::WebSocket;
pub use web_socket::WebSocketInterface;

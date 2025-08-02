mod http_request;
mod https;
mod web_socket;

pub use http_request::HTTPRequest;
pub use http_request::RequestType;
pub use https::HTTPS;
pub use web_socket::MessageDataType;
pub use web_socket::WebSocket;
pub use web_socket::WebSocketInterface;

mod tests {

    #[test]
    fn it_works() {
        println!("lol")
    }
}

mod web_socket;
mod https;
mod http_request;

pub use web_socket::MessageDataType;
pub use web_socket::WebSocketInterface;
pub use web_socket::WebSocket;
pub use https::HTTPS;
pub use http_request::HTTPRequest;
pub use http_request::RequestType;

mod tests {

    #[test]
    fn it_works() {
        println!("lol")
    }
}
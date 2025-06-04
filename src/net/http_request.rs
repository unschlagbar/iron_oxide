

#[allow(unused)]
pub struct HTTPRequest {
    pub request: RequestType,
    pub host: Option<String>,
}

impl HTTPRequest {
    pub fn parse(buf: &[u8]) -> Option<Self> {

        let text_form = String::from_utf8_lossy(buf);

        let mut request = RequestType::GET(String::with_capacity(0), None);

        for line in text_form.lines() {
            if line.starts_with("GET ") {
                let req = line[4..].split_whitespace().find(|x| x.starts_with("/")).unwrap_or("").replace("%20", " ");
                let req = req.split_once("?").unwrap_or((&req, ""));
                let path = req.0.to_string();
                let query;

                if req.1 == "" {
                    query = None;
                } else {
                    query = Some(req.1.to_string());
                }

                request = RequestType::GET(path, query);

            } else if line.starts_with("Sec-WebSocket-Key") {
                request = RequestType::UpgradeWs(line[19..].to_string());
                break;
            }
        }

        
        Some(Self { request, host: None})
    }
}

#[allow(unused)]
pub enum RequestType {
    GET(String, Option<String>),
    POST,
    UpgradeWs(String)
}
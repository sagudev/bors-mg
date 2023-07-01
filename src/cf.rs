use worker::*;

pub trait Req {
    fn get_header(&self, header: &str) -> Option<String>;
}

impl Req for Request {
    fn get_header(&self, header: &str) -> Option<String> {
        match self.headers().get(header) {
            Ok(Some(val)) => Some(val),
            _ => None,
        }
    }
}

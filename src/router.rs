pub const BASE_URL_1: &str = "192.168.1.3";
pub const BASE_URL_9: &str = "192.168.9.1";

#[derive(Debug)]
pub struct Router {
    pub base: String,
    initial_path: String,
    pub path: String,
}

impl Default for Router {
    fn default() -> Self {
        Router {
            base: String::from(BASE_URL_9),
            initial_path: String::from("anime"),
            path: String::from("anime"),
        }
    }
}

impl Router {
    pub fn route(&mut self, new_path: &str) {
        self.path = new_path.replace("/?raw=true", "")[1..].to_string();
    }
    pub fn route_replace_top(&mut self, new_path: &str) {
        let new_path = &new_path.replace("/?raw=true", "")[1..];
        self.path = new_path.to_string();
        self.initial_path = new_path.to_string();
    }
    pub fn up(&mut self) {
        let new_path = &self.path[0..self.path.rfind("/").unwrap_or(0)];
        if self.path != self.initial_path { self.path = new_path.to_string(); }
    }
}

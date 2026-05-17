//! Nimsuggest-compatible query server and LSP bridge.

#[cfg(test)]
use rnim_allocator as _;
pub struct SuggestServer {}

impl SuggestServer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_query(&mut self, _query: &str) -> String {
        String::new()
    }
}

impl Default for SuggestServer {
    fn default() -> Self {
        Self::new()
    }
}

use serde::{Serialize, Deserialize};
use aragog::query::Query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageLimit {
    pub page: u32,
    pub limit: u32,
}

impl Default for PageLimit {
    fn default() -> Self {
        PageLimit { page: 1, limit: 50 }
    }
}

impl PageLimit {
    pub fn offset(&self) -> u32 {
        self.page.saturating_sub(1).saturating_mul(self.limit)
    }

    pub fn to_aql(&self) -> String {
        format!("LIMIT {offset}, {limit}", offset = self.offset(), limit = self.limit)
    }
}

pub trait ApplyPageLimit {
    fn apply_page_limit(self, page_limit: PageLimit) -> Self;
}

impl ApplyPageLimit for Query {
    fn apply_page_limit(self, page_limit: PageLimit) -> Self {
        self.limit(page_limit.limit, Some(page_limit.offset()))
    }
}

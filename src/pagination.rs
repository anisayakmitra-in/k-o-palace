//! Pagination helpers and bounds.

use crate::error::{PalaceError, PalaceErrorCode};

pub const DEFAULT_LIMIT: usize = 50;
pub const MAX_LIMIT: usize = 250;
pub const MAX_OFFSET: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    pub limit: usize,
    pub offset: usize,
}

impl Pagination {
    pub fn new(limit: usize, offset: usize) -> Result<Self, crate::error::PalaceError> {
        if limit == 0 {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                "limit must be greater than 0",
            ));
        }
        if limit > MAX_LIMIT {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                format!("limit must be <= {MAX_LIMIT}"),
            ));
        }
        if offset > MAX_OFFSET {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                format!("offset must be <= {MAX_OFFSET}"),
            ));
        }
        Ok(Self { limit, offset })
    }

    pub fn bounds(&self, total: usize) -> (usize, usize) {
        let start = self.offset.min(total);
        let end = self.offset.saturating_add(self.limit).min(total);
        (start, end)
    }
}

#![expect(dead_code, reason = "The full API is not in use yet.")]

/// Tracks an allocated resource budget (cycles or compute) and how much has been consumed.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    allocated: u64,
    consumed: u64,
}

impl Budget {
    #[must_use]
    pub(crate) const fn new(allocated: u64) -> Self {
        Self { allocated, consumed: 0 }
    }

    pub(crate) const fn consume(&mut self, amount: u64) {
        self.consumed += amount;
    }

    #[must_use]
    /// Returns the total allocated budget.
    pub const fn allocated(&self) -> u64 {
        self.allocated
    }

    #[must_use]
    /// Returns the amount consumed so far.
    pub const fn consumed(&self) -> u64 {
        self.consumed
    }

    #[must_use]
    /// Returns the remaining budget (may be negative if over-consumed).
    pub const fn remaining(&self) -> i64 {
        self.allocated.cast_signed() - self.consumed.cast_signed()
    }

    #[must_use]
    /// Returns `true` if the budget is fully consumed.
    pub const fn exhausted(&self) -> bool {
        self.consumed >= self.allocated
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    /// Returns the usage ratio as a value between 0.0 and 1.0+.
    pub fn usage(&self) -> f64 {
        if self.allocated == 0 {
            return 0.0;
        }

        self.consumed as f64 / self.allocated as f64
    }
}

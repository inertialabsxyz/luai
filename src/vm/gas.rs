use crate::types::value::LuaValue;

/// Gas costs for each VM operation category.
pub mod gas_cost {
    pub const BASE_INSTRUCTION: u64 = 1;
    pub const FUNCTION_CALL: u64 = 10;
    pub const FUNCTION_RETURN: u64 = 5;
    pub const PCALL_SETUP: u64 = 15;
    pub const PCALL_UNWIND: u64 = 10;
    pub const TABLE_ALLOC: u64 = 20;
    pub const TABLE_GET: u64 = 2;
    pub const TABLE_SET_EXISTING: u64 = 2;
    pub const TABLE_SET_NEW_KEY: u64 = 5;
    pub const ITER_SORTED_STEP: u64 = 2;
    pub const ITER_ARRAY_STEP: u64 = 1;
    pub const LEN: u64 = 1;
    pub const TOOL_CALL_BASE: u64 = 100;
    pub const LOG_BASE: u64 = 10;
    pub const ERROR_RAISE: u64 = 10;
    // Variable costs computed at call site:
    //   TABLE_GROW: entries_after_rehash (new_capacity)
    //   CONCAT(n values): len(result)
    //   STRING_COPY: bytes_copied
    //   PAIRS_SORTED_SETUP: n * ceil(log2(n + 1))
    //   TOOL_CALL: 100 + args_bytes + response_bytes
    //   LOG: 10 + len(message)
}

/// Errors that the VM can raise.
#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
    /// ERR_GAS — unrecoverable
    GasExhausted,
    /// ERR_MEM — unrecoverable
    MemoryExhausted,
    /// ERR_DEPTH — recoverable via pcall
    CallDepthExceeded,
    /// ERR_TYPE — recoverable via pcall
    TypeError(String),
    /// ERR_RUNTIME — recoverable via pcall; also raised by error() builtin
    RuntimeError(LuaValue),
    /// ERR_TOOL — recoverable via pcall
    ToolError(String),
    /// ERR_OUTPUT — unrecoverable
    OutputExceeded,
}

impl VmError {
    /// True for errors that cannot be caught by pcall.
    pub fn is_unrecoverable(&self) -> bool {
        matches!(
            self,
            VmError::GasExhausted | VmError::MemoryExhausted | VmError::OutputExceeded
        )
    }
}

/// Gas metering state for one VM run.
#[derive(Debug)]
pub struct GasMeter {
    remaining: u64,
    limit: u64,
}

impl GasMeter {
    /// Create a new meter with the given budget.
    pub fn new(limit: u64) -> Self {
        GasMeter {
            remaining: limit,
            limit,
        }
    }

    /// Deduct `amount` gas. Returns `Err(VmError::GasExhausted)` if the budget
    /// would be exceeded.
    pub fn charge(&mut self, amount: u64) -> Result<(), VmError> {
        if amount > self.remaining {
            self.remaining = 0;
            Err(VmError::GasExhausted)
        } else {
            self.remaining -= amount;
            Ok(())
        }
    }

    /// Gas remaining (informational).
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Gas consumed so far.
    pub fn used(&self) -> u64 {
        self.limit - self.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_charge_within_budget() {
        let mut g = GasMeter::new(200);
        g.charge(100).unwrap();
        assert_eq!(g.remaining(), 100);
    }

    #[test]
    fn gas_charge_exact_budget() {
        let mut g = GasMeter::new(200);
        g.charge(200).unwrap();
        assert_eq!(g.remaining(), 0);
    }

    #[test]
    fn gas_charge_exceeds_budget() {
        let mut g = GasMeter::new(100);
        assert_eq!(g.charge(101), Err(VmError::GasExhausted));
    }

    #[test]
    fn gas_used_tracks_charges() {
        let mut g = GasMeter::new(500);
        g.charge(100).unwrap();
        g.charge(50).unwrap();
        assert_eq!(g.used(), 150);
        assert_eq!(g.remaining(), 350);
    }

    #[test]
    fn gas_zero_limit() {
        let mut g = GasMeter::new(0);
        assert_eq!(g.charge(1), Err(VmError::GasExhausted));
    }
}

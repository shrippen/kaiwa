//! What two processes on one profile file must not lose.

use rusqlite::{Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::{Result, Store};

/// How long a writer waits for another process's write (a loaded disk, many bookings).
const BUSY_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

impl Store {
    /// Reads a state value (its default when missing), changes it and writes it back as one
    /// transaction that takes the write lock first: two writers on one file lose no change.
    pub fn update_state_value<T, R>(&self, key: &str, f: impl FnOnce(&mut T) -> R) -> Result<(T, R)>
    where
        T: Serialize + for<'de> Deserialize<'de> + Default,
    {
        self.db.busy_timeout(BUSY_WAIT)?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)?;
        let mut v: T = self.state_value(key)?.unwrap_or_default();
        let out = f(&mut v);
        self.set_state_value(key, &v)?;
        tx.commit()?;
        Ok((v, out))
    }
}

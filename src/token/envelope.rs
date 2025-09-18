use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::errors::Error;

/// Serializable snapshot used by contract tests and quickstart flows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub value: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scoped: bool,
}

impl TokenSnapshot {
    pub fn issued_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.issued_at)
    }

    pub fn expires_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.expires_at)
    }
}

/// Represents an authentication token plus metadata required for refresh decisions.
#[derive(Clone, Debug)]
pub struct TokenEnvelope {
    value: String,
    issued_at: SystemTime,
    expires_at: SystemTime,
    scoped: bool,
    refresh_in_progress: bool,
}

impl TokenEnvelope {
    /// Create a token envelope validating TTL expectations (minimum 60 seconds).
    pub fn try_new(
        value: String,
        issued_at: SystemTime,
        expires_at: SystemTime,
        scoped: bool,
    ) -> Result<Self, Error> {
        if expires_at <= issued_at {
            return Err(Error::Config("Token expires before or at issuance".into()));
        }
        let ttl = expires_at
            .duration_since(issued_at)
            .map_err(|_| Error::Config("Token TTL underflow".into()))?;
        if ttl < Duration::from_secs(60) {
            return Err(Error::Config(
                "Token TTL must be at least 60 seconds to support proactive refresh".into(),
            ));
        }
        Ok(Self {
            value,
            issued_at,
            expires_at,
            scoped,
            refresh_in_progress: false,
        })
    }

    pub fn from_snapshot(snapshot: TokenSnapshot) -> Result<Self, Error> {
        let TokenSnapshot {
            value,
            issued_at,
            expires_at,
            scoped,
        } = snapshot;
        Self::try_new(
            value,
            UNIX_EPOCH + Duration::from_secs(issued_at),
            UNIX_EPOCH + Duration::from_secs(expires_at),
            scoped,
        )
    }

    pub fn to_snapshot(&self) -> TokenSnapshot {
        TokenSnapshot {
            value: self.value.clone(),
            issued_at: secs_since_epoch(self.issued_at),
            expires_at: secs_since_epoch(self.expires_at),
            scoped: self.scoped,
        }
    }

    /// Returns the raw token value suitable for Authorization headers.
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn issued_at(&self) -> SystemTime {
        self.issued_at
    }

    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    pub fn scoped(&self) -> bool {
        self.scoped
    }

    pub fn refresh_in_progress(&self) -> bool {
        self.refresh_in_progress
    }

    pub fn set_refresh_in_progress(&mut self, refreshing: bool) {
        self.refresh_in_progress = refreshing;
    }

    /// Returns how long until the token expires relative to the provided time.
    pub fn remaining(&self, now: SystemTime) -> Option<Duration> {
        self.expires_at.duration_since(now).ok()
    }

    /// Returns the lifetime of the token from issuance to expiry.
    pub fn lifetime(&self) -> Option<Duration> {
        self.expires_at.duration_since(self.issued_at).ok()
    }

    /// Replaces the token value and timestamps when a refresh succeeds.
    pub fn update_from_snapshot(&mut self, snapshot: TokenSnapshot) -> Result<(), Error> {
        let updated = TokenEnvelope::from_snapshot(snapshot)?;
        self.value = updated.value;
        self.issued_at = updated.issued_at;
        self.expires_at = updated.expires_at;
        self.scoped = updated.scoped;
        self.refresh_in_progress = false;
        Ok(())
    }
}

fn secs_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

use base64::Engine as _;
use pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};
use rsa::pkcs1::{DecodeRsaPrivateKey as _, EncodeRsaPrivateKey as _};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::{Config, Error};

const MIN_EXP_SECS: u64 = 30;
const MAX_EXP_SECS: u64 = 3600;

fn now_secs() -> Result<u64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Config(format!("Time error: {e}")))?
        .as_secs())
}

fn clamp_exp_secs(desired: Option<u64>) -> (u64, Option<u64>) {
    let original = desired.unwrap_or(MAX_EXP_SECS);
    let effective = original.clamp(MIN_EXP_SECS, MAX_EXP_SECS);
    let clamped_from = if effective != original {
        Some(original)
    } else {
        None
    };
    (effective, clamped_from)
}

/// Returns base64 encoded fingerprint of a public key derived from the RSA key.
pub(super) fn compute_fingerprint(key: &rsa::RsaPublicKey) -> Result<String, Error> {
    let spki = key
        .to_public_key_der()
        .map_err(|e| Error::Key(format!("SubjectPublicKeyInfo DER encode failed: {e}")))?;
    let fingerprint = {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(spki.as_bytes());
        let b64 = base64::engine::general_purpose::STANDARD.encode(hash);
        format!("SHA256:{}", b64)
    };
    Ok(fingerprint)
}

fn load_rsa_private_key_from_pem(
    pem_str: &str,
    passphrase: Option<&str>,
) -> Result<rsa::RsaPrivateKey, Error> {
    if let Ok(blocks) = pem::parse_many(pem_str.as_bytes()) {
        for block in &blocks {
            match block.tag() {
                "ENCRYPTED PRIVATE KEY" => {
                    let pass = passphrase.ok_or_else(|| {
                        Error::Key("Encrypted private key provided but no passphrase set".into())
                    })?;
                    return rsa::RsaPrivateKey::from_pkcs8_encrypted_der(block.contents(), pass)
                        .map_err(|e| Error::Key(format!("PKCS#8 decryption failed: {e}")));
                }
                "PRIVATE KEY" => {
                    return rsa::RsaPrivateKey::from_pkcs8_der(block.contents())
                        .map_err(|e| Error::Key(format!("PKCS#8 parse failed: {e}")));
                }
                "RSA PRIVATE KEY" => {
                    return rsa::RsaPrivateKey::from_pkcs1_der(block.contents())
                        .map_err(|e| Error::Key(format!("PKCS#1 parse failed: {e}")));
                }
                _ => continue,
            }
        }
    }

    if let Some(pass) = passphrase
        && let Ok(key) = rsa::RsaPrivateKey::from_pkcs8_encrypted_pem(pem_str, pass)
    {
        return Ok(key);
    }
    if let Ok(key) = rsa::RsaPrivateKey::from_pkcs8_pem(pem_str) {
        return Ok(key);
    }
    if let Ok(key) = rsa::RsaPrivateKey::from_pkcs1_pem(pem_str) {
        return Ok(key);
    }

    Err(Error::Key(
        "Invalid RSA private key: unsupported format or incorrect passphrase".into(),
    ))
}

struct AssertionBundle {
    token: String,
    issued_at: u64,
    expires_at: u64,
    lifetime_secs: u64,
    clamped_from: Option<u64>,
}

fn build_assertion(cfg: &Config, log_clamp: bool) -> Result<AssertionBundle, Error> {
    let private_key = cfg.private_key()?;
    let prefix = "TEST://assertion:";
    let now = now_secs()?;
    if let Some(rest) = private_key.strip_prefix(prefix) {
        return Ok(AssertionBundle {
            token: rest.to_string(),
            issued_at: now,
            expires_at: now + MIN_EXP_SECS,
            lifetime_secs: MIN_EXP_SECS,
            clamped_from: None,
        });
    }

    let (lifetime_secs, clamped_from) = clamp_exp_secs(cfg.jwt_exp_secs);
    if let Some(original) = clamped_from.filter(|_| log_clamp) {
        warn!(
            original_seconds = original,
            effective_seconds = lifetime_secs,
            "jwt_exp_secs outside [{MIN_EXP_SECS}, {MAX_EXP_SECS}] - clamped for safety"
        );
    }

    let name = cfg.login.as_deref().unwrap_or(&cfg.user);
    let rsa_key =
        load_rsa_private_key_from_pem(&private_key, cfg.private_key_passphrase.as_deref())?;
    let fingerprint = match cfg.public_key_fp.as_ref() {
        Some(fp) => fp.clone(),
        None => compute_fingerprint(&rsa_key.to_public_key())?,
    };
    let account_norm = cfg.account.to_uppercase().replace('.', "-");
    let user_norm = name.to_uppercase();
    let sub = format!("{}.{}", account_norm, user_norm);
    let iss = format!("{}.{}", sub, fingerprint);
    let exp = now + lifetime_secs;

    #[derive(serde::Serialize)]
    struct Claims {
        iss: String,
        sub: String,
        iat: u64,
        exp: u64,
    }
    let claims = Claims {
        iss,
        sub,
        iat: now,
        exp,
    };

    let pkcs1 = rsa_key
        .to_pkcs1_der()
        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
    let enc_key = jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes());
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &enc_key,
    )
    .map_err(|e| Error::JwtSign(format!("JWT signing failed: {e}")))?;

    Ok(AssertionBundle {
        token,
        issued_at: now,
        expires_at: exp,
        lifetime_secs,
        clamped_from,
    })
}

pub(super) fn generate_assertion(cfg: &Config) -> Result<String, Error> {
    Ok(build_assertion(cfg, true)?.token)
}

pub(crate) struct JwtContext {
    token: Option<String>,
    issued_at: u64,
    expires_at: u64,
    lifetime_secs: u64,
    refresh_margin_secs: u64,
    clamp_logged: bool,
    last_refresh_warning: Option<Instant>,
}

impl JwtContext {
    pub(crate) fn new(cfg: &Config, refresh_margin_secs: u64) -> Result<Self, Error> {
        if refresh_margin_secs < MIN_EXP_SECS {
            return Err(Error::Config(format!(
                "jwt_refresh_margin_secs must be at least {MIN_EXP_SECS} seconds (got {refresh_margin_secs})"
            )));
        }
        let (lifetime_secs, _clamped_from) = clamp_exp_secs(cfg.jwt_exp_secs);
        if refresh_margin_secs >= lifetime_secs {
            return Err(Error::Config(format!(
                "refresh margin ({refresh_margin_secs}) must be less than effective JWT lifetime ({lifetime_secs})"
            )));
        }

        Ok(Self {
            token: None,
            issued_at: 0,
            expires_at: 0,
            lifetime_secs,
            refresh_margin_secs,
            clamp_logged: false,
            last_refresh_warning: None,
        })
    }

    pub(crate) fn ensure_valid(&mut self, cfg: &Config) -> Result<String, Error> {
        let now = now_secs()?;
        let needs_refresh = match self.token {
            None => true,
            Some(_) => {
                let remaining = self.expires_at.saturating_sub(now);
                if remaining <= self.refresh_margin_secs {
                    let should_log = match &self.last_refresh_warning {
                        Some(ts) => ts.elapsed() >= Duration::from_secs(5),
                        None => true,
                    };
                    if should_log {
                        info!(
                            remaining_seconds = remaining,
                            margin_seconds = self.refresh_margin_secs,
                            "refreshing JWT because remaining lifetime is within safety margin"
                        );
                        self.last_refresh_warning = Some(Instant::now());
                    }
                    true
                } else {
                    false
                }
            }
        };

        if needs_refresh {
            let bundle = build_assertion(cfg, !self.clamp_logged)?;
            if bundle.clamped_from.is_some() {
                self.clamp_logged = true;
            }
            self.token = Some(bundle.token);
            self.issued_at = bundle.issued_at;
            self.expires_at = bundle.expires_at;
            self.lifetime_secs = bundle.lifetime_secs;
        } else {
            debug!(
                remaining_seconds = self.expires_at.saturating_sub(now),
                "reusing cached JWT"
            );
        }

        Ok(self
            .token
            .as_ref()
            .expect("token should exist after ensure_valid")
            .clone())
    }

    #[cfg(test)]
    pub(crate) fn force_issued_at(&mut self, issued_at: u64) {
        self.issued_at = issued_at;
        self.expires_at = issued_at + self.lifetime_secs;
    }

    pub(crate) fn invalidate(&mut self) {
        self.token = None;
        self.last_refresh_warning = None;
    }
}

#[cfg(test)]
mod tests;

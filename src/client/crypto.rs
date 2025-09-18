use base64::Engine as _;
use pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};
use rsa::pkcs1::{DecodeRsaPrivateKey as _, EncodeRsaPrivateKey as _};

use crate::{Config, Error};

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

pub(super) fn generate_assertion(cfg: &Config) -> Result<String, Error> {
    // Test helper: allow bypass via special prefix
    let private_key = cfg.private_key()?;
    let prefix = "TEST://assertion:";
    if let Some(rest) = private_key.strip_prefix(prefix) {
        // No iat/exp context in bypass; pick small window forcing refresh paths in tests if needed
        return Ok(rest.to_string());
    }
    let name = match cfg.login.as_ref() {
        Some(login) => login,
        None => &cfg.user,
    };
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
    let iat = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Config(format!("Time error: {e}")))?
        .as_secs();
    let exp_secs = cfg.jwt_exp_secs.unwrap_or(3600);
    if exp_secs == 0 || exp_secs > 3600 {
        return Err(Error::Config(format!(
            "jwt_exp_secs must be between 1 and 3600 seconds (got {})",
            exp_secs
        )));
    }
    let exp = iat + exp_secs;

    #[derive(serde::Serialize)]
    struct Claims {
        iss: String,
        sub: String,
        iat: u64,
        exp: u64,
    }
    let claims = Claims { iss, sub, iat, exp };

    let pkcs1 = rsa_key
        .to_pkcs1_der()
        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
    let enc_key = jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes());
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let assertion = jsonwebtoken::encode(&header, &claims, &enc_key)
        .map_err(|e| Error::JwtSign(format!("JWT signing failed: {e}")))?;
    Ok(assertion)
}

#[cfg(test)]
mod tests;

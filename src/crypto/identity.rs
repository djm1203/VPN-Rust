//! A node's persistent keypair and self-signed certificate.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use zeroize::Zeroizing;

use super::certificate_fingerprint;

/// A node's DER-encoded certificate and private key.
///
/// Identities are self-signed. A peer authenticates this node by pinning
/// [`NodeIdentity::certificate`] (shared out-of-band; compare
/// [`NodeIdentity::fingerprint`] to verify it). The PKCS#8 private key bytes are
/// held in a [`Zeroizing`] buffer so they are wiped from memory on drop.
pub struct NodeIdentity {
    cert: CertificateDer<'static>,
    key_pkcs8: Zeroizing<Vec<u8>>,
}

impl NodeIdentity {
    /// Generate a fresh self-signed identity with `subject` as the certificate's
    /// subject alternative name.
    pub fn generate(subject: &str) -> Result<Self> {
        let cert = rcgen::generate_simple_self_signed(vec![subject.to_string()])
            .context("failed to generate self-signed certificate")?;
        let cert_der = cert.cert.der().clone();
        let key_pkcs8 = Zeroizing::new(cert.key_pair.serialize_der());
        Ok(Self {
            cert: cert_der,
            key_pkcs8,
        })
    }

    /// Load an identity from DER-encoded certificate and key files.
    pub fn load(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let cert = fs::read(cert_path)
            .with_context(|| format!("failed to read certificate '{}'", cert_path.display()))?;
        let key = fs::read(key_path)
            .with_context(|| format!("failed to read private key '{}'", key_path.display()))?;
        Ok(Self {
            cert: CertificateDer::from(cert),
            key_pkcs8: Zeroizing::new(key),
        })
    }

    /// Load the identity from disk, or generate and persist a new one if either
    /// file is missing (the `keygen`-style bootstrap).
    pub fn load_or_generate(cert_path: &Path, key_path: &Path, subject: &str) -> Result<Self> {
        if cert_path.exists() && key_path.exists() {
            Self::load(cert_path, key_path)
        } else {
            let identity = Self::generate(subject)?;
            identity.save(cert_path, key_path)?;
            Ok(identity)
        }
    }

    /// Persist the identity as DER files, creating parent directories as needed.
    /// On Unix the private key file is restricted to `0600`.
    pub fn save(&self, cert_path: &Path, key_path: &Path) -> Result<()> {
        if let Some(parent) = cert_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(cert_path, self.cert.as_ref())
            .with_context(|| format!("failed to write certificate '{}'", cert_path.display()))?;
        fs::write(key_path, self.key_pkcs8.as_slice())
            .with_context(|| format!("failed to write private key '{}'", key_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(key_path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// The DER-encoded certificate (safe to share for pinning).
    pub fn certificate(&self) -> CertificateDer<'static> {
        self.cert.clone()
    }

    /// The DER-encoded private key.
    pub fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key_pkcs8.to_vec()))
    }

    /// A human-readable SHA-256 fingerprint of the certificate, for out-of-band
    /// verification (e.g. `sha256:ab:cd:...`).
    pub fn fingerprint(&self) -> String {
        certificate_fingerprint(&self.cert)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_then_round_trip_through_files() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("vpn-rust-id-{}", std::process::id()));
        fs::create_dir_all(&dir).ok();
        let cert_path = dir.join("node.crt.der");
        let key_path = dir.join("node.key.der");

        let generated = NodeIdentity::generate("test-node")?;
        generated.save(&cert_path, &key_path)?;

        let loaded = NodeIdentity::load(&cert_path, &key_path)?;
        assert_eq!(generated.certificate(), loaded.certificate());
        assert_eq!(
            generated.private_key().secret_der(),
            loaded.private_key().secret_der()
        );
        // Fingerprint is stable across a save/load round trip.
        assert_eq!(generated.fingerprint(), loaded.fingerprint());

        fs::remove_dir_all(&dir).ok();
        Ok(())
    }
}

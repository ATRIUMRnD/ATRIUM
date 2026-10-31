//! ML-DSA-65 (FIPS 204) signatures on the memory chain.
//!
//! Same primitives and key handling as ductei-mcp's receipt signing and LIMEN's license
//! issuer: pqcrypto-mldsa for the algorithm, the secret key DPAPI-wrapped (user scope) through
//! QFLTL's keystore, the public key next to it in hex, and a short key id (kid) so a verifier
//! knows which key an entry claims. Signatures are domain-separated (DOMAIN || entry hash) so a
//! memory signature can never be replayed as a DUCTEI receipt or a LIMEN license.
//!
//! Windows only for now: the macOS keystore stores UTF-8 strings in the Keychain, not raw key
//! bytes, so the Mac build keeps the chain unsigned (scheme absent) until that is wired.
//! Off under `cargo test` so a test run never creates key material in the real config dir.

pub const SCHEME: &str = "ml-dsa-65";
pub const DOMAIN: &[u8] = b"xing-pc-mcp/memory/v1\0";

fn disabled() -> bool {
    cfg!(test)
}

pub fn kid_of(pk: &[u8]) -> String {
    blake3::hash(pk).to_hex()[..16].to_string()
}

#[cfg(windows)]
mod imp {
    use super::{kid_of, DOMAIN};
    use crate::config;
    use pqcrypto_mldsa::mldsa65 as alg;
    use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _, SecretKey as _};
    use std::{fs, path::PathBuf, sync::OnceLock};

    pub struct Signer {
        sk: alg::SecretKey,
        pk: alg::PublicKey,
        pub kid: String,
    }

    fn sk_file() -> PathBuf {
        config::dir().join("memory-signing.dpapi")
    }
    fn pk_file() -> PathBuf {
        config::dir().join("memory-signing.pub")
    }

    impl Signer {
        /// Fresh in-memory key pair (tests, or a one-off).
        pub fn generate() -> Self {
            let (pk, sk) = alg::keypair();
            let kid = kid_of(pk.as_bytes());
            Signer { sk, pk, kid }
        }

        fn load_or_create() -> anyhow::Result<Self> {
            fs::create_dir_all(config::dir())?;
            if sk_file().exists() {
                let raw = config::keystore_unprotect(&fs::read(sk_file())?)?;
                let sk = alg::SecretKey::from_bytes(&raw).map_err(|e| anyhow::anyhow!("secret key: {e:?}"))?;
                let pk_hex = fs::read_to_string(pk_file())?;
                let pk = alg::PublicKey::from_bytes(&config::unhex(pk_hex.trim())?)
                    .map_err(|e| anyhow::anyhow!("public key: {e:?}"))?;
                let kid = kid_of(pk.as_bytes());
                return Ok(Signer { sk, pk, kid });
            }
            let s = Self::generate();
            // Secret first, then public: a crash between the two leaves a key that
            // load_or_create refuses (missing .pub) instead of a half-usable one.
            fs::write(sk_file(), config::keystore_protect(s.sk.as_bytes())?)?;
            fs::write(pk_file(), config::hex(s.pk.as_bytes()))?;
            eprintln!("xing-pc-mcp: new memory signing key, kid {} ({})", s.kid, pk_file().display());
            Ok(s)
        }

        pub fn public_hex(&self) -> String {
            config::hex(self.pk.as_bytes())
        }

        /// Detached signature (hex) over DOMAIN || entry hash.
        pub fn sign(&self, hash_hex: &str) -> String {
            let mut m = DOMAIN.to_vec();
            m.extend_from_slice(hash_hex.as_bytes());
            config::hex(alg::detached_sign(&m, &self.sk).as_bytes())
        }

        pub fn verify(&self, hash_hex: &str, sig_hex: &str) -> bool {
            let Ok(sig_b) = config::unhex(sig_hex) else { return false };
            let Ok(sig) = alg::DetachedSignature::from_bytes(&sig_b) else { return false };
            let mut m = DOMAIN.to_vec();
            m.extend_from_slice(hash_hex.as_bytes());
            alg::verify_detached_signature(&sig, &m, &self.pk).is_ok()
        }
    }

    static SIGNER: OnceLock<Option<Signer>> = OnceLock::new();

    /// The machine's signer, created on first use. None if the keystore refused (the chain
    /// then stays hash-only, and the failure is printed once).
    pub fn signer() -> Option<&'static Signer> {
        if super::disabled() {
            return None;
        }
        SIGNER
            .get_or_init(|| match Signer::load_or_create() {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("xing-pc-mcp: memory signing off: {e:#}");
                    None
                }
            })
            .as_ref()
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Signer {
        pub kid: String,
    }
    impl Signer {
        pub fn sign(&self, _h: &str) -> String {
            String::new()
        }
        pub fn verify(&self, _h: &str, _s: &str) -> bool {
            false
        }
        pub fn public_hex(&self) -> String {
            String::new()
        }
    }
    pub fn signer() -> Option<&'static Signer> {
        None
    }
}

pub use imp::signer;

#[cfg(all(test, windows))]
mod tests {
    use super::imp::Signer;
    use super::*;

    #[test]
    fn sign_verify_roundtrip_and_domain_separation() {
        let s = Signer::generate();
        let h = blake3::hash(b"entry").to_hex().to_string();
        let sig = s.sign(&h);
        assert!(s.verify(&h, &sig));
        assert!(!s.verify(&blake3::hash(b"other").to_hex().to_string(), &sig));
        assert!(!s.verify(&h, "00ff"));
        let other = Signer::generate();
        assert!(!other.verify(&h, &sig));
        assert_ne!(s.kid, other.kid);
        assert_eq!(s.kid.len(), 16);
        assert_eq!(s.public_hex().len(), 1952 * 2);
    }

    #[test]
    fn signer_is_off_under_test() {
        assert!(signer().is_none());
    }
}

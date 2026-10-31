//! Clé API chiffrée avec Argon2id + XChaCha20Poly1305 dans SQLite.
//! Le mot de passe maître ne doit jamais être enregistré dans la base.
use anyhow::{bail, Result};
use argon2::Argon2;
use chacha20poly1305::{XChaCha20Poly1305, KeyInit, XNonce, aead::Aead};

pub fn seal(secret:&str,passphrase:&str)->Result<(Vec<u8>,Vec<u8>,Vec<u8>)> {
    if secret.trim().is_empty() || secret.len()>4096 || passphrase.chars().count()<12 {
        bail!("Clé vide/trop longue ou phrase maîtresse inférieure à 12 caractères");
    }
    let mut salt=[0u8;16];
    let mut nonce=[0u8;24];
    getrandom::fill(&mut salt).map_err(|e|anyhow::anyhow!("Aléa indisponible : {e}"))?;
    getrandom::fill(&mut nonce).map_err(|e|anyhow::anyhow!("Aléa indisponible : {e}"))?;
    let mut key=[0u8;32];
    Argon2::default().hash_password_into(passphrase.as_bytes(),&salt,&mut key)
        .map_err(|e|anyhow::anyhow!("Dérivation de clé échouée : {e}"))?;
    let cipher=XChaCha20Poly1305::new_from_slice(&key).map_err(|_|anyhow::anyhow!("Clé invalide"))?;
    let ciphertext=cipher.encrypt(XNonce::from_slice(&nonce),secret.as_bytes())
        .map_err(|_|anyhow::anyhow!("Chiffrement impossible"))?;
    key.fill(0);
    Ok((salt.to_vec(),nonce.to_vec(),ciphertext))
}
pub fn unseal(salt:&[u8],nonce:&[u8],ciphertext:&[u8],passphrase:&str)->Result<String> {
    if salt.len()!=16 || nonce.len()!=24 || ciphertext.is_empty(){bail!("Clé chiffrée corrompue");}
    let mut key=[0u8;32];
    Argon2::default().hash_password_into(passphrase.as_bytes(),salt,&mut key)
        .map_err(|e|anyhow::anyhow!("Dérivation de clé échouée : {e}"))?;
    let cipher=XChaCha20Poly1305::new_from_slice(&key).map_err(|_|anyhow::anyhow!("Clé invalide"))?;
    let plain=cipher.decrypt(XNonce::from_slice(nonce),ciphertext)
        .map_err(|_|anyhow::anyhow!("Mot de passe incorrect ou données altérées"))?;
    key.fill(0);
    Ok(String::from_utf8(plain)?)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn authenticated_encryption()->Result<()>{
        let (s,n,c)=seal("secret-de-test","une phrase maitresse longue")?;
        assert_eq!(unseal(&s,&n,&c,"une phrase maitresse longue")?,"secret-de-test");
        assert!(unseal(&s,&n,&c,"mot de passe different").is_err());
        Ok(())
    }
}

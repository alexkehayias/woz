use std::io::{Read, Write};
use std::fs::File;
use std::fs;
use std::path::PathBuf;

use ring::aead::*;
use ring::rand::*;
use ring::pbkdf2::*;
use ring::error::Unspecified;

use failure::Error;
use failure::ResultExt;


// TODO replace this with something useful. All it does is wrap a
// Nonce, but since sealing/opening keys require you implement a
// NonceSequence this is needed. Maybe generate the Nonce in the `new`
// method?
struct OneNonceSequence(Option<Nonce>);

impl OneNonceSequence {
    fn new(nonce: Nonce) -> Self {
        Self(Some(nonce))
    }
}

impl NonceSequence for OneNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        self.0.take().ok_or(Unspecified)
    }
}

#[derive(Clone)]
pub struct FileCache {
    key: [u8; 32],
    path: PathBuf,
}

impl FileCache {
    pub fn new(key: [u8; 32], path: PathBuf) -> Self {
        FileCache { key, path }
    }

    pub fn make_key(password: &str, salt: &str) -> [u8; 32] {
        let mut key = [0; 32];
        derive(
            PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(100).unwrap(),
            salt.as_bytes(),
            &password.as_bytes()[..],
            &mut key
        );
        key
    }

    fn encrypt(&self, content: Vec<u8>) -> Vec<u8> {
        // Ring uses the same input variable as output
        let mut in_out = content.clone();

        // Fill nonce with random data. Random data must be used only
        // once per encryption
        let mut nonce_value = vec![0; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce_value)
            .expect("Failed to fill random bytes");
        let nonce = Nonce::try_assume_unique_for_key(&nonce_value)
            .expect("Failed to create nonce");

        // Sealing key used to encrypt data
        let s_key = UnboundKey::new(&CHACHA20_POLY1305, &self.key).unwrap();
        let mut sealing_key = SealingKey::new(
            s_key,
            OneNonceSequence::new(nonce)
        );

        // Additional data that you would like to send and it would
        // not be encrypted but it would be signed
        let additional_data = Aad::empty();

        // Encrypt data into in_out variable
        sealing_key.seal_in_place_append_tag(
            additional_data,
            &mut in_out
        ).expect("Failed to seal");

        // Add in the nonce to the end so we can extract it later for
        // decrypting
        for i in nonce_value {
            in_out.push(i);
        }
        in_out
    }

    fn decrypt(&self, nonce: Nonce, content: Vec<u8>) -> Vec<u8> {
        let mut in_out = content.clone();
        // Opening key used to decrypt data
        let o_key = UnboundKey::new(&CHACHA20_POLY1305, &self.key)
            .expect("Failed to init decryption key");
        let mut opening_key = OpeningKey::new(
            o_key,
            OneNonceSequence::new(nonce)
        );

        // Additional data that you would like to send and it would
        // not be encrypted but it would be signed
        let additional_data = Aad::empty();
        // Encrypt data into in_out variable
        opening_key.open_in_place(
            additional_data,
            &mut in_out
        ).expect("Failed to decrypt");
        // Remove the extra padding from suffix
        in_out[..in_out.len() - CHACHA20_POLY1305.tag_len()].to_vec()
    }

    fn extract_nonce(encrypted_content: Vec<u8>) -> (Nonce, Vec<u8>) {
        let len = encrypted_content.len();
        let content = encrypted_content[..len - 12].to_vec();
        let nonce_value = encrypted_content[len - 12..len].to_vec();
        let nonce = Nonce::try_assume_unique_for_key(&nonce_value)
            .expect("Failed to initialize nonce");
        (nonce, content)
    }

    fn write(home_path: &PathBuf, file_name: String, value: Vec<u8>) -> Result<(), Error> {
        let mut file_path = home_path.clone();
        file_path.push(file_name);
        let mut f = File::create(&file_path).context("Failed to create file")?;
        Ok(f.write_all(&value).context("Failed to write file")?)
    }

    fn read(home_path: &PathBuf, file_name: String) -> Result<String, Error> {
        let mut path = home_path.clone();
        path.push(file_name);
        Ok(fs::read_to_string(path)?)
    }

    pub fn get(&self, key: &str) -> Result<String, Error> {
        Ok(Self::read(&self.path, format!(".{}", key))?)
    }

    pub fn set(&self, key: &str, value: Vec<u8>) -> Result<(), Error> {
        Ok(Self::write(&self.path, format!(".{}", key), value)?)
    }

    pub fn get_encrypted(&self, key: &str) -> Result<String, Error> {
        let mut path = self.path.clone();
        path.push(format!(".{}", key));

        let mut f = fs::File::open(path).context("Unable to find encrypted file")?;
        let mut buffer = vec![];
        f.read_to_end(&mut buffer).context("Failed to read encrypted file")?;

        let (nonce, content) = Self::extract_nonce(buffer);
        let out = String::from_utf8(self.decrypt(nonce, content))?;
        Ok(out)
    }

    pub fn set_encrypted(&self, key: &str, value: Vec<u8>) -> Result<(), Error> {
        let encrypted_value = self.encrypt(value);
        Ok(self.set(key, encrypted_value)?)
    }
}

/// Note: files written during tests end up in the system temp
/// directory which should get cleaned up periodically so we don't
/// have to here.
#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::env;


    fn make_key() -> [u8; 32] {
        FileCache::make_key("test password", "test salt")
    }

    #[test]
    fn make_key_works() {
        let result = FileCache::make_key("test-password", "test-salt");
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn encrypt_works() {
        let key = make_key();
        let home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path);
        let out = cache.encrypt(b"secret message".to_vec());
        assert!(out.len() > 0);
    }

    #[test]
    fn extract_nonce_works() {
        let key = make_key();
        let home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path);
        let out = cache.encrypt(b"secret message".to_vec());
        FileCache::extract_nonce(out);
    }

    #[test]
    fn decrypt_works() {
        let key = make_key();
        let home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path);
        let value = b"secret message".to_vec();
        let encrypted = cache.clone().encrypt(value.clone());
        let (nonce, content) = FileCache::extract_nonce(encrypted);
        let actual = cache.decrypt(nonce, content);
        assert_eq!(value, actual);
    }

    #[test]
    fn get_works() {
        let key = make_key();
        let home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path);
        cache.set("test-get-key", "test value".as_bytes().to_vec()).unwrap();
        let result = cache.get("test-get-key").unwrap();
        assert_eq!("test value", result);
    }

    #[test]
    fn get_encrypted_works() {
        let key = make_key();
        let home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path.clone());
        cache.set_encrypted(
            "test-get-encrypted-key",
            "test value".as_bytes().to_vec()
        ).unwrap();

        let result = cache.get_encrypted("test-get-encrypted-key").unwrap();
        assert_eq!("test value", result);
    }

    #[test]
    fn set_works() {
        let key = make_key();
        let mut home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path.clone());
        cache.set(
            "test-set-key",
            "test value".as_bytes().to_vec()
        ).expect("Failed to set cache");
        home_path.push(".test-set-key");
        assert_eq!("test value", fs::read_to_string(home_path).expect("Failed to read file"));
    }

    #[test]
    fn set_encrypted_works() {
        let key = make_key();
        let mut home_path = env::temp_dir();
        let cache = FileCache::new(key, home_path.clone());
        cache.set_encrypted(
            "test-set-encrypted-key",
            "test value".as_bytes().to_vec()
        ).unwrap();

        home_path.push(".test-set-encrypted-key");
        let mut f = fs::File::open(home_path).unwrap();
        let mut buffer = vec![];
        f.read_to_end(&mut buffer).unwrap();
        assert_ne!("test value".as_bytes().to_vec(), buffer);
    }
}

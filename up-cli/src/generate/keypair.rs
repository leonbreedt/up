use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use openssl::{rsa::Rsa, symm::Cipher};

use crate::CliError;

/// Generate keypair.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "keypair")]
pub struct GenerateKeypairCommand {
    #[argh(positional)]
    public_key_file_name: Utf8PathBuf,

    #[argh(positional)]
    private_key_file_name: Utf8PathBuf,

    /// key size in bits (default: 2048)
    #[argh(option, default = "2048")]
    size: u32,

    /// do not protect key with password (default: false)
    #[argh(switch)]
    no_passphrase: bool,
}

impl GenerateKeypairCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!("generating RSA keypair ({} bits)", self.size);

        let key = Rsa::generate(self.size)?;

        let private_key_pem = if self.no_passphrase {
            key.private_key_to_pem()?
        } else {
            let passphrase = rpassword::prompt_password("passphrase: ")?;
            key.private_key_to_pem_passphrase(Cipher::aes_128_cbc(), passphrase.as_bytes())?
        };
        let public_key_pem = key.public_key_to_pem()?;

        tracing::info!("saving private key to {}", self.private_key_file_name);
        fs::write(&self.private_key_file_name, private_key_pem)?;
        tracing::info!("saving public key to {}", self.public_key_file_name);
        fs::write(&self.public_key_file_name, public_key_pem)?;

        Ok(())
    }
}

use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    rsa::Rsa,
    symm::Cipher,
    x509::{
        extension::{AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier},
        X509Name, X509,
    },
};

use crate::CliError;

pub const DEFAULT_KEY_SIZE: u32 = 2048;
pub const DEFAULT_EXPIRY_DAYS: u32 = 1825;

/// Generate keypair.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "ca-certificate")]
pub struct GenerateCACertificateCommand {
    /// certificate common name (example: ca.domain.com)
    #[argh(positional)]
    common_name: String,

    #[argh(positional)]
    file_name: Utf8PathBuf,

    /// key size in bits (default: 2048)
    #[argh(option, default = "DEFAULT_KEY_SIZE")]
    size: u32,

    /// do not protect key with password (default: false)
    #[argh(switch)]
    no_passphrase: bool,

    /// certificate expiry in days from today (default: 1825)
    #[argh(option, default = "DEFAULT_EXPIRY_DAYS")]
    expiry_days: u32,
}

impl GenerateCACertificateCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!("generating CA certificate for {}", self.common_name);

        let passphrase = if self.no_passphrase {
            None
        } else {
            Some(rpassword::prompt_password("passphrase: ")?)
        };

        let bundle = generate_ca_certificate_bundle_with_key(
            self.size,
            &self.common_name,
            self.expiry_days,
            passphrase.as_deref(),
        )?;

        tracing::info!("saving CA certificate to {}", self.file_name);
        fs::write(&self.file_name, &bundle)?;

        Ok(())
    }
}

pub fn generate_ca_certificate_bundle_with_key(
    key_size: u32,
    common_name: &str,
    expiry_days: u32,
    passphrase: Option<&str>,
) -> Result<Vec<u8>, CliError> {
    let keypair = Rsa::generate(key_size)?;
    let pkey = PKey::from_rsa(keypair.clone())?;

    let mut issuer_name = X509Name::builder()?;
    issuer_name.append_entry_by_nid(Nid::COMMONNAME, common_name)?;
    let issuer_name = issuer_name.build();

    let mut builder = X509::builder()?;

    builder.set_version(2)?;
    builder.set_subject_name(&issuer_name)?;
    builder.set_issuer_name(&issuer_name)?;
    builder.set_not_before(&Asn1Time::days_from_now(0).unwrap())?;
    builder.set_not_after(&Asn1Time::days_from_now(expiry_days).unwrap())?;
    builder.set_pubkey(&pkey)?;

    let mut serial = BigNum::new()?;
    serial.rand(128, MsbOption::MAYBE_ZERO, false)?;
    builder.set_serial_number(&serial.to_asn1_integer().unwrap())?;

    // From https://superuser.com/questions/738612/openssl-ca-keyusage-extension

    let basic_constraints = BasicConstraints::new().critical().ca().build()?;
    builder.append_extension(basic_constraints)?;
    let key_usage = KeyUsage::new()
        .critical()
        .crl_sign()
        .digital_signature()
        .key_cert_sign()
        .build()?;
    builder.append_extension(key_usage)?;
    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&builder.x509v3_context(None, None))?;
    builder.append_extension(subject_key_identifier).unwrap();
    let authority_key_identifier = AuthorityKeyIdentifier::new()
        .keyid(true)
        .issuer(true)
        .build(&builder.x509v3_context(None, None))?;
    builder.append_extension(authority_key_identifier)?;

    builder.sign(&pkey, MessageDigest::sha256())?;

    let certificate: X509 = builder.build();

    let mut pem_bundle = Vec::new();

    pem_bundle.extend_from_slice(&certificate.to_pem()?);
    if let Some(passphrase) = passphrase {
        pem_bundle.extend_from_slice(
            &keypair.private_key_to_pem_passphrase(Cipher::aes_128_cbc(), passphrase.as_bytes())?,
        );
    } else {
        pem_bundle.extend_from_slice(&keypair.private_key_to_pem()?);
    }
    pem_bundle.extend_from_slice(&keypair.public_key_to_pem()?);

    Ok(pem_bundle)
}

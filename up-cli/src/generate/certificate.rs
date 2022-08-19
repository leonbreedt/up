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
        extension::{
            AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage,
            SubjectAlternativeName, SubjectKeyIdentifier,
        },
        X509Name, X509,
    },
};

use crate::CliError;

/// Generate keypair.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "certificate")]
pub struct GenerateCertificateCommand {
    // path to CA certificate file
    #[argh(positional)]
    ca_file_name: Utf8PathBuf,

    // where to save the certificate file
    #[argh(positional)]
    file_name: Utf8PathBuf,

    /// certificate common name (example: domain.com)
    #[argh(positional)]
    common_name: String,

    /// key size in bits (default: 2048)
    #[argh(option, default = "2048")]
    size: u32,

    /// do not protect key with password (default: false)
    #[argh(switch)]
    no_passphrase: bool,

    /// certificate expiry in days from today (default: 365)
    #[argh(option, default = "365")]
    expiry_days: u32,

    /// an additional name to include as a Subject Alternative Name. may be repeated.
    #[argh(option)]
    alt_name: Vec<String>,
}

impl GenerateCertificateCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!(
            "issuing certificate for {} using {}",
            self.common_name,
            self.ca_file_name
        );

        let ca_pem = fs::read(&self.ca_file_name)?;
        let ca_keypair = Rsa::private_key_from_pem(&ca_pem)?;
        let ca_pkey = PKey::from_rsa(ca_keypair)?;
        let ca_x509 = X509::from_pem(&ca_pem)?;

        let keypair = Rsa::generate(self.size)?;
        let pkey = PKey::from_rsa(keypair.clone())?;

        let mut issuer_name = X509Name::builder()?;
        issuer_name.append_entry_by_nid(Nid::COMMONNAME, &subject_common_name(&ca_x509))?;
        let issuer_name = issuer_name.build();

        let mut subject_name = X509Name::builder()?;
        subject_name.append_entry_by_nid(Nid::COMMONNAME, &self.common_name)?;
        let subject_name = subject_name.build();

        let mut builder = X509::builder()?;

        builder.set_version(2)?;
        builder.set_subject_name(&subject_name)?;
        builder.set_issuer_name(&issuer_name)?;
        builder.set_not_before(&Asn1Time::days_from_now(0).unwrap())?;
        builder.set_not_after(&Asn1Time::days_from_now(self.expiry_days).unwrap())?;
        builder.set_pubkey(&pkey)?;

        let mut serial = BigNum::new()?;
        serial.rand(128, MsbOption::MAYBE_ZERO, false)?;
        builder.set_serial_number(&serial.to_asn1_integer().unwrap())?;

        let basic_constraints = BasicConstraints::new().critical().build()?;
        builder.append_extension(basic_constraints)?;
        let key_usage = KeyUsage::new()
            .critical()
            .non_repudiation()
            .digital_signature()
            .key_encipherment()
            .key_agreement()
            .build()?;
        builder.append_extension(key_usage)?;
        let ext_key_usage = ExtendedKeyUsage::new()
            .client_auth()
            .server_auth()
            .build()?;
        builder.append_extension(ext_key_usage)?;

        let subject_key_identifier =
            SubjectKeyIdentifier::new().build(&builder.x509v3_context(Some(&ca_x509), None))?;
        builder.append_extension(subject_key_identifier).unwrap();

        // authority key identifier must always be the key ID of CA certificate.
        let authority_key_identifier = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&builder.x509v3_context(Some(&ca_x509), None))?;

        builder.append_extension(authority_key_identifier)?;

        if !self.alt_name.is_empty() {
            let mut subject_alt_name = SubjectAlternativeName::new();
            for alt_name in self.alt_name.iter() {
                subject_alt_name.dns(alt_name);
            }
            let subject_alt_name =
                subject_alt_name.build(&builder.x509v3_context(Some(&ca_x509), None))?;
            builder.append_extension(subject_alt_name)?;
        }

        builder.sign(&ca_pkey, MessageDigest::sha256())?;

        let certificate: X509 = builder.build();

        let mut pem_bundle = Vec::new();

        pem_bundle.extend_from_slice(&certificate.to_pem()?);
        if self.no_passphrase {
            pem_bundle.extend_from_slice(&keypair.private_key_to_pem()?);
        } else {
            let passphrase = rpassword::prompt_password("passphrase: ")?;
            pem_bundle.extend_from_slice(
                &keypair
                    .private_key_to_pem_passphrase(Cipher::aes_128_cbc(), passphrase.as_bytes())?,
            );
        }
        pem_bundle.extend_from_slice(&keypair.public_key_to_pem()?);

        tracing::info!("saving certificate to {}", self.file_name);
        fs::write(&self.file_name, pem_bundle)?;

        Ok(())
    }
}

fn subject_common_name(cert: &X509) -> String {
    cert.subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .and_then(|v| v.data().as_utf8().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(String::new)
}

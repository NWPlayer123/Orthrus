use core::fmt;
use core::fmt::Write;

use x509_parser::der_parser::Oid;
use x509_parser::prelude::*;
use x509_parser::public_key::PublicKey;
use x509_parser::signature_algorithm::SignatureAlgorithm;
use ouroboros::self_referencing;

#[self_referencing]
pub struct Certificate {
    der_buf: Box<[u8]>,
    #[borrows(der_buf)]
    #[covariant]
    cert: X509Certificate<'this>,
}

impl Certificate {
    pub fn from_der(der: &[u8]) -> crate::Result<Self> {

        let (rest, _cert) = X509Certificate::from_der(der)?;
        let der_buf = der[..(der.len() - rest.len())].into();

        CertificateTryBuilder {
            der_buf,
            cert_builder: |buf| match X509Certificate::from_der(buf) {
                Ok((_rest, cert)) => Ok(cert),
                Err(err) => Err(err.into())
            }
        }.try_build()
    }

    pub fn cert(&self) -> &X509Certificate<'_> {
        self.borrow_cert()
    }

    pub fn len(&self) -> usize {
        self.borrow_der_buf().len()
    }
}

impl fmt::Debug for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.borrow_cert().fmt(f)
    }
}

fn make_indent(indent: usize) -> String {
    "    ".repeat(indent)
}

fn octet_string(data: &[u8]) -> String {
    data.iter()
        .fold(String::new(), |mut output, b| {
            write!(&mut output, "{b:02X}:").unwrap();
            output
        })
        .trim_end_matches(':')
        .to_string()
}

fn hex_string(data: &[u8]) -> Vec<String> {
    data.chunks(16)
        .map(|chunk| {
            chunk.iter().fold(String::new(), |mut output, b| {
                write!(&mut output, "{b:02X}:").unwrap();
                output
            })
        })
        .map(|line| format!("{:16}", line.trim_end_matches(':')))
        .collect()
}

fn format_oid(oid: &Oid) -> String {
    x509_parser::oid_registry::format_oid(oid, oid_registry())
}

/// This function prints all relevant information from an [X.509 Certificate](X509Certificate) to
/// [`log::debug`] using the specified indentation.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if unable to parse the [To Be Signed
/// Certificate](print_x509_tbs) or [Signature Algorithm](print_x509_signature_algorithm).
pub fn print_x509_info(cert: &X509Certificate) -> crate::Result<()> {
    let indent = 1;
    print_x509_tbs(&cert.tbs_certificate, indent)?;

    print_x509_signature_algorithm(&cert.signature_algorithm, indent)?;

    log::debug!("{}Signature Value:", make_indent(indent));
    for line in hex_string(&cert.signature_value.data) {
        log::debug!("{}{}", make_indent(indent + 1), line);
    }

    Ok(())
}

/// This function prints all relevant information from a [To Be Signed Certificate](TbsCertificate)
/// to [`log::debug`] using the specified indentation.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if unable to parse the [Signature
/// Algorithm](print_x509_signature_algorithm), [Validity](print_x509_validity), or [Public Key
/// Info](print_x509_public_key_info).
pub fn print_x509_tbs(cert: &TbsCertificate, indent: usize) -> crate::Result<()> {
    log::debug!("{}To Be Signed:", make_indent(indent));
    let version = cert.version;

    log::debug!("{}Version: {}", make_indent(indent + 1), version);

    log::debug!(
        "{}Serial Number: {}",
        make_indent(indent + 1),
        octet_string(cert.raw_serial())
    );

    print_x509_signature_algorithm(&cert.signature, indent + 1)?;

    log::debug!("{}Subject: {}", make_indent(indent + 1), cert.subject);

    log::debug!("{}Issuer:  {}", make_indent(indent + 1), cert.issuer);

    print_x509_validity(&cert.validity, indent + 1)?;

    print_x509_public_key_info(&cert.subject_pki, indent + 1)?;

    if version == X509Version::V2 || version == X509Version::V3 {
        if let Some(unique_id) = &cert.issuer_uid {
            log::debug!(
                "{}Issuer Unique ID: {}",
                make_indent(indent + 1),
                octet_string(&unique_id.0.data)
            );
        }
        if let Some(unique_id) = &cert.subject_uid {
            log::debug!(
                "{}Subject Unique ID: {}",
                make_indent(indent + 1),
                octet_string(&unique_id.0.data)
            );
        }
        if version == X509Version::V3 {
            log::debug!("{}Extensions:", make_indent(indent + 1));

            let mut extensions = cert.extensions().to_vec();
            extensions.sort_by(|a, b| {
                get_extension_order(a.parsed_extension())
                    .cmp(&get_extension_order(b.parsed_extension()))
            });
            for extension in extensions {
                print_x509_extension(&extension, indent + 2);
            }
        }
    }
    Ok(())
}

/// This function converts an [`AlgorithmIdentifier`] to a [`SignatureAlgorithm`] and prints all
/// relevant information to [`log::debug`] using the specified indentation.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if it fails to convert to a
/// [`SignatureAlgorithm`], or if the algorithm encoding is invalid in the case of
/// [RSASSA-PSS](SignatureAlgorithm::RSASSA_PSS) and [RSASSA-OAEP](SignatureAlgorithm::RSAAES_OAEP).
pub fn print_x509_signature_algorithm(
    algo: &AlgorithmIdentifier,
    indent: usize,
) -> crate::Result<()> {
    match SignatureAlgorithm::try_from(algo)? {
        SignatureAlgorithm::RSA => {
            log::debug!("{}Signature Algorithm: RSA", make_indent(indent));
        }
        SignatureAlgorithm::DSA => {
            log::debug!("{}Signature Algorithm: DSA", make_indent(indent));
        }
        SignatureAlgorithm::ECDSA => {
            log::debug!("{}Signature Algorithm: ECDSA", make_indent(indent));
        }
        SignatureAlgorithm::ED25519 => {
            log::debug!("{}Signature Algorithm: ED25519", make_indent(indent));
        }
        SignatureAlgorithm::RSASSA_PSS(params) => {
            log::debug!("{}Signature Algorithm: RSASSA-PSS", make_indent(indent));
            log::debug!(
                "{}Hash Algorithm: {}",
                make_indent(indent + 1),
                format_oid(params.hash_algorithm_oid())
            );
            let mask_gen = params.mask_gen_algorithm()?;
            log::debug!(
                "{}Mask Generation Function: {}/{}",
                make_indent(indent + 1),
                format_oid(&mask_gen.mgf),
                format_oid(&mask_gen.hash)
            );
            log::debug!(
                "{}Salt Length: {}",
                make_indent(indent + 1),
                params.salt_length()
            );
        }
        SignatureAlgorithm::RSAAES_OAEP(params) => {
            log::debug!("{}Signature Algorithm: RSASSA-OAEP", make_indent(indent));
            log::debug!(
                "{}Hash Algorithm: {}",
                make_indent(indent + 1),
                format_oid(params.hash_algorithm_oid())
            );
            let mask_gen = params.mask_gen_algorithm()?;
            log::debug!(
                "{}Mask Generation Function: {}/{}",
                make_indent(indent + 1),
                format_oid(&mask_gen.mgf),
                format_oid(&mask_gen.hash)
            );
            log::debug!(
                "{}P Source Function: {}",
                make_indent(indent + 1),
                format_oid(&params.p_source_alg().algorithm)
            );
        }
    }
    Ok(())
}

/// This function and prints all [Validity] information to [`log::debug`] using the specified
/// indentation.
///
/// # Errors
/// Returns [`TimeInvalidRange`](crate::Error::TimeInvalidRange) if unable to convert the timestamp
/// to a valid date.
pub fn print_x509_validity(valid: &Validity, indent: usize) -> crate::Result<()> {
    log::debug!("{}Validity:", make_indent(indent));
    log::debug!(
        "{}Not Before: {}",
        make_indent(indent + 1),
        crate::time::format_timestamp(valid.not_before.timestamp())?
    );
    log::debug!(
        "{}Not After:  {}",
        make_indent(indent + 1),
        crate::time::format_timestamp(valid.not_after.timestamp())?
    );
    log::debug!(
        "{}[Still Valid: {}]",
        make_indent(indent + 1),
        valid.is_valid()
    );
    Ok(())
}

/// This function parses an [`AlgorithmIdentifier`] and prints the Object Identifier and Parameters
/// to [`log::debug`] using the specified indentation.
pub fn print_x509_digest_algorithm(algo: &AlgorithmIdentifier, indent: usize) {
    let temp = format_oid(&algo.algorithm);
    log::debug!("{}Object Identifier: {}", make_indent(indent), temp);
    if let Some(params) = &algo.parameters {
        if params.data.is_empty() {
            log::debug!(
                "{}Algorithm Parameter: {}",
                make_indent(indent),
                params.tag()
            );
        } else {
            log::debug!(
                "{}Algorithm Parameter: {} {:?}",
                make_indent(indent),
                params.tag(),
                params.data
            );
        }
    }
}

/// This function parses a [`SubjectPublicKeyInfo`] and prints all relevant information to
/// [`log::debug`] using the specified indentation.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if it fails to parse the public key info.
pub fn print_x509_public_key_info(info: &SubjectPublicKeyInfo, indent: usize) -> crate::Result<()> {
    log::debug!("{}Subject Public Key Info:", make_indent(indent));
    print_x509_digest_algorithm(&info.algorithm, indent + 1);
    match info.parsed()? {
        PublicKey::RSA(rsa) => {
            log::debug!(
                "{}RSA Public Key: ({} bit, {:#X} exponent)",
                make_indent(indent + 1),
                rsa.key_size(),
                rsa.try_exponent()?
            );
            for line in hex_string(rsa.modulus) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
        PublicKey::EC(ec) => {
            log::debug!(
                "{}EC Public Key: ({} bit)",
                make_indent(indent + 1),
                ec.key_size()
            );
            for line in hex_string(ec.data()) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
        PublicKey::DSA(y) => {
            log::debug!(
                "{}DSA Public Key: ({} bit)",
                make_indent(indent + 1),
                8 * y.len()
            );
            for line in hex_string(y) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
        PublicKey::GostR3410(y) => {
            log::debug!(
                "{}GOST R 34.10-94 Public Key: ({} bit)",
                make_indent(indent + 1),
                8 * y.len()
            );
            for line in hex_string(y) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
        PublicKey::GostR3410_2012(y) => {
            log::debug!(
                "{}GOST R 34.10-2012 Public Key: ({} bit)",
                make_indent(indent + 1),
                8 * y.len()
            );
            for line in hex_string(y) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
        PublicKey::Unknown(b) => {
            log::debug!(
                "{}Unknown Key Type! ({} bit)",
                make_indent(indent + 1),
                8 * b.len()
            );
            for line in hex_string(b) {
                log::debug!("{}{}", make_indent(indent + 2), line);
            }
        }
    }
    Ok(())
}

#[must_use]
/// This returns the raw enum names for [`ParsedExtension`], instead of using camelCase for debug.
pub const fn get_extension_name(extension: &ParsedExtension) -> &'static str {
    match extension {
        ParsedExtension::UnsupportedExtension { .. } => "UnsupportedExtension",
        ParsedExtension::ParseError { .. } => "ParseError",
        ParsedExtension::AuthorityKeyIdentifier(_) => "AuthorityKeyIdentifier",
        ParsedExtension::SubjectKeyIdentifier(_) => "SubjectKeyIdentifier",
        ParsedExtension::KeyUsage(_) => "KeyUsage",
        ParsedExtension::CertificatePolicies(_) => "CertificatePolicies",
        ParsedExtension::PolicyMappings(_) => "PolicyMappings",
        ParsedExtension::SubjectAlternativeName(_) => "SubjectAlternativeName",
        ParsedExtension::IssuerAlternativeName(_) => "IssuerAlternativeName",
        ParsedExtension::BasicConstraints(_) => "BasicConstraints",
        ParsedExtension::NameConstraints(_) => "NameConstraints",
        ParsedExtension::PolicyConstraints(_) => "PolicyConstraints",
        ParsedExtension::ExtendedKeyUsage(_) => "ExtendedKeyUsage",
        ParsedExtension::CRLDistributionPoints(_) => "CRLDistributionPoints",
        ParsedExtension::InhibitAnyPolicy(_) => "InhibitAnyPolicy",
        ParsedExtension::AuthorityInfoAccess(_) => "AuthorityInfoAccess",
        ParsedExtension::NSCertType(_) => "NSCertType",
        ParsedExtension::NsCertComment(_) => "NsCertComment",
        ParsedExtension::CRLNumber(_) => "CRLNumber",
        ParsedExtension::ReasonCode(_) => "ReasonCode",
        ParsedExtension::InvalidityDate(_) => "InvalidityDate",
        ParsedExtension::SCT(_) => "SCT",
        ParsedExtension::Unparsed => "Unparsed",
    }
}

#[must_use]
/// This returns the preferred sorting order for [X509 Extensions](X509Extension).
pub const fn get_extension_order(ext: &ParsedExtension) -> usize {
    match ext {
        ParsedExtension::AuthorityKeyIdentifier(_) => 0,
        ParsedExtension::SubjectKeyIdentifier(_) => 1,
        ParsedExtension::KeyUsage(_) => 2,
        ParsedExtension::CertificatePolicies(_) => 3,
        ParsedExtension::PolicyMappings(_) => 4,
        ParsedExtension::SubjectAlternativeName(_) => 5,
        ParsedExtension::IssuerAlternativeName(_) => 6,
        ParsedExtension::BasicConstraints(_) => 7,
        ParsedExtension::NameConstraints(_) => 8,
        ParsedExtension::PolicyConstraints(_) => 9,
        ParsedExtension::ExtendedKeyUsage(_) => 10,
        ParsedExtension::CRLDistributionPoints(_) => 11,
        ParsedExtension::InhibitAnyPolicy(_) => 12,
        ParsedExtension::AuthorityInfoAccess(_) => 13,
        ParsedExtension::CRLNumber(_) => 14,
        ParsedExtension::ReasonCode(_) => 15,
        ParsedExtension::InvalidityDate(_) => 16,
        ParsedExtension::SCT(_) => 17,
        ParsedExtension::NSCertType(_) => 18,
        ParsedExtension::NsCertComment(_) => 19,
        ParsedExtension::UnsupportedExtension { .. } => 20,
        ParsedExtension::ParseError { .. } => 21,
        ParsedExtension::Unparsed => 22,
    }
}

/// Parses an [`X509Extension`] and prints all relevant information to [`log::debug`] using the
/// specified indentation.
///
/// Currently, it only supports [`AuthorityKeyIdentifier`],
/// [`SubjectKeyIdentifier`](ParsedExtension::SubjectKeyIdentifier), [`KeyUsage`],
/// [`BasicConstraints`], and [`ExtendedKeyUsage`], it will print the debug output of any other
/// extension.
pub fn print_x509_extension(extension: &X509Extension, indent: usize) {
    let parsed = extension.parsed_extension();
    log::debug!(
        "{}[critical:{}] {}",
        make_indent(indent),
        extension.critical,
        get_extension_name(parsed)
    );

    match parsed {
        ParsedExtension::AuthorityKeyIdentifier(identifier) => {
            print_x509_authority_key_identifier(identifier, indent + 1);
        }
        ParsedExtension::SubjectKeyIdentifier(identifier) => {
            print_x509_subject_key_identifier(identifier, indent + 1);
        }
        ParsedExtension::KeyUsage(usage) => {
            print_x509_key_usage(usage, indent + 1);
        }
        ParsedExtension::BasicConstraints(constraints) => {
            print_x509_basic_constraints(constraints, indent + 1);
        }
        ParsedExtension::ExtendedKeyUsage(usage) => {
            print_x509_extended_key_usage(usage, indent + 1);
        }
        x => {
            log::debug!("{}{:?}", make_indent(indent + 1), x);
        }
    }
}

fn print_x509_authority_key_identifier(identifier: &AuthorityKeyIdentifier, indent: usize) {
    if let Some(key_id) = &identifier.key_identifier {
        log::debug!(
            "{}Identifier: {}",
            make_indent(indent),
            octet_string(key_id.0)
        );
    }
    if let Some(issuer) = &identifier.authority_cert_issuer {
        for name in issuer {
            log::debug!("{}Issuer: {}", make_indent(indent), name);
        }
    }
    if let Some(serial) = identifier.authority_cert_serial {
        log::debug!("{}Serial: {}", make_indent(indent), octet_string(serial));
    }
}

fn print_x509_subject_key_identifier(identifier: &KeyIdentifier, indent: usize) {
    log::debug!(
        "{}Identifier: {}",
        make_indent(indent),
        octet_string(identifier.0)
    );
}

fn print_x509_key_usage(usage: &KeyUsage, indent: usize) {
    log::debug!("{}Key Usage: {}", make_indent(indent), usage);
}

fn print_x509_basic_constraints(constraints: &BasicConstraints, indent: usize) {
    log::debug!(
        "{}Certificate Authority: {}",
        make_indent(indent),
        constraints.ca
    );
    if constraints.ca {
        if let Some(value) = constraints.path_len_constraint {
            log::debug!("{}Certification Path Limit: {}", make_indent(indent), value);
        }
    }
}

fn print_x509_extended_key_usage(usage: &ExtendedKeyUsage, indent: usize) {
    if usage.any {
        log::debug!("{}Any: true", make_indent(indent));
    }
    if usage.server_auth {
        log::debug!("{}Server Authentication: true", make_indent(indent));
    }
    if usage.client_auth {
        log::debug!("{}Client Authentication: true", make_indent(indent));
    }
    if usage.code_signing {
        log::debug!("{}Code Signing: true", make_indent(indent));
    }
    if usage.email_protection {
        log::debug!("{}Email Protection: true", make_indent(indent));
    }
    if usage.time_stamping {
        log::debug!("{}Time Stamping: true", make_indent(indent));
    }
    if usage.ocsp_signing {
        log::debug!("{}Certificate Status Signing: true", make_indent(indent));
    }
    for oid in &usage.other {
        log::debug!("{}Other: {}", make_indent(indent), oid);
    }
}

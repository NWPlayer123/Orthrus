//TODO: need to re-evaluate x509-parser to allow for no_std

/*use core::fmt;
use core::fmt::Write;

use ouroboros::self_referencing;
use x509_parser::der_parser::Oid;
use x509_parser::prelude::*;
use x509_parser::public_key::PublicKey;
use x509_parser::signature_algorithm::SignatureAlgorithm;

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
                Err(err) => Err(err.into()),
            },
        }
        .try_build()
    }

    #[must_use]
    pub fn cert(&self) -> &X509Certificate<'_> {
        self.borrow_cert()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.borrow_der_buf().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Debug for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.borrow_cert().fmt(f)
    }
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

/// This function parses  all relevant information from an [X.509 Certificate](X509Certificate) and
/// returns a [`String`] with the formatted data.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if unable to parse the [To Be Signed
/// Certificate](print_x509_tbs) or [Signature Algorithm](print_x509_signature_algorithm).
pub fn print_x509_info(cert: &X509Certificate) -> crate::Result<String> {
    let indent = 1;
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    output.push_str(&print_x509_tbs(&cert.tbs_certificate, indent)?);

    output.push_str(&print_x509_signature_algorithm(
        &cert.signature_algorithm,
        indent,
    )?);

    output.push_str(&format!("{indentation}Signature Value:\n"));
    indentation += "    ";
    for line in hex_string(&cert.signature_value.data) {
        output.push_str(&format!("{indentation}{line}\n"));
    }

    Ok(output)
}

/// This function parses all relevant information from a [To Be Signed Certificate](TbsCertificate)
/// and returns a [`String`] with the formatted data.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if unable to parse the [Signature
/// Algorithm](print_x509_signature_algorithm), [Validity](print_x509_validity), or [Public Key
/// Info](print_x509_public_key_info).
pub fn print_x509_tbs(cert: &TbsCertificate, indent: usize) -> crate::Result<String> {
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    output.push_str(&format!("{indentation}To Be Signed:\n"));
    indentation += "    ";

    let version = cert.version;
    output.push_str(&format!("{indentation}Version: {version}\n"));

    output.push_str(&format!(
        "{indentation}Serial Number: {}\n",
        octet_string(cert.raw_serial())
    ));

    output.push_str(&print_x509_signature_algorithm(
        &cert.signature,
        indent + 1,
    )?);

    output.push_str(&format!("{indentation}Subject: {}\n", cert.subject));

    output.push_str(&format!("{indentation}Issuer:  {}\n", cert.issuer));

    output.push_str(&print_x509_validity(&cert.validity, indent + 1)?);

    output.push_str(&print_x509_public_key_info(&cert.subject_pki, indent + 1)?);

    if version == X509Version::V2 || version == X509Version::V3 {
        if let Some(unique_id) = &cert.issuer_uid {
            output.push_str(&format!(
                "{indentation}Issuer Unique ID: {}\n",
                octet_string(&unique_id.0.data)
            ));
        }
        if let Some(unique_id) = &cert.subject_uid {
            output.push_str(&format!(
                "{indentation}Subject Unique ID: {}\n",
                octet_string(&unique_id.0.data)
            ));
        }
        if version == X509Version::V3 {
            output.push_str(&format!("{indentation}Extensions:\n"));

            let mut extensions = cert.extensions().to_vec();
            extensions.sort_by(|a, b| {
                get_extension_order(a.parsed_extension())
                    .cmp(&get_extension_order(b.parsed_extension()))
            });
            for extension in extensions {
                output.push_str(&print_x509_extension(&extension, indent + 2));
            }
        }
    }

    Ok(output)
}

/// This function converts an [`AlgorithmIdentifier`] to a [`SignatureAlgorithm`] and returns a
/// [`String`] with the formatted data.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if it fails to convert to a
/// [`SignatureAlgorithm`], or if the algorithm encoding is invalid in the case of
/// [RSASSA-PSS](SignatureAlgorithm::RSASSA_PSS) and [RSASSA-OAEP](SignatureAlgorithm::RSAAES_OAEP).
pub fn print_x509_signature_algorithm(
    algo: &AlgorithmIdentifier,
    indent: usize,
) -> crate::Result<String> {
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    match SignatureAlgorithm::try_from(algo)? {
        SignatureAlgorithm::RSA => {
            output.push_str(&format!("{indentation}Signature Algorithm: RSA\n"));
        }
        SignatureAlgorithm::DSA => {
            output.push_str(&format!("{indentation}Signature Algorithm: DSA\n"));
        }
        SignatureAlgorithm::ECDSA => {
            output.push_str(&format!("{indentation}Signature Algorithm: ECDSA\n"));
        }
        SignatureAlgorithm::ED25519 => {
            output.push_str(&format!("{indentation}Signature Algorithm: ED25519\n"));
        }
        SignatureAlgorithm::RSASSA_PSS(params) => {
            output.push_str(&format!("{indentation}Signature Algorithm: RSASSA-PSS\n"));
            indentation += "    ";

            output.push_str(&format!(
                "{indentation}Hash Algorithm: {}\n",
                format_oid(params.hash_algorithm_oid())
            ));

            let mask_gen = params.mask_gen_algorithm()?;
            output.push_str(&format!(
                "{indentation}Mask Generation Function: {}/{}\n",
                format_oid(&mask_gen.mgf),
                format_oid(&mask_gen.hash)
            ));

            output.push_str(&format!(
                "{indentation}Salt Length: {}\n",
                params.salt_length()
            ));
        }
        SignatureAlgorithm::RSAAES_OAEP(params) => {
            output.push_str(&format!("{indentation}Signature Algorithm: RSASSA-OAEP\n"));
            indentation += "    ";

            output.push_str(&format!(
                "{indentation}Hash Algorithm: {}\n",
                format_oid(params.hash_algorithm_oid())
            ));

            let mask_gen = params.mask_gen_algorithm()?;
            output.push_str(&format!(
                "{indentation}Mask Generation Function: {}/{}\n",
                format_oid(&mask_gen.mgf),
                format_oid(&mask_gen.hash)
            ));

            output.push_str(&format!(
                "{indentation}P Source Function: {}\n",
                format_oid(&params.p_source_alg().algorithm)
            ));
        }
    }

    Ok(output)
}

/// This function parses all [Validity] information and returns a [`String`] with the formatted
/// data.
///
/// # Errors
/// Returns [`TimeInvalidRange`](crate::Error::TimeInvalidRange) if unable to convert the timestamp
/// to a valid date.
pub fn print_x509_validity(valid: &Validity, indent: usize) -> crate::Result<String> {
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    output.push_str(&format!("{indentation}Validity:\n"));
    indentation += "    ";
    output.push_str(&format!(
        "{indentation}Not Before: {}\n",
        crate::time::format_timestamp(valid.not_before.timestamp())?
    ));
    output.push_str(&format!(
        "{indentation}Not After:  {}\n",
        crate::time::format_timestamp(valid.not_after.timestamp())?
    ));
    output.push_str(&format!(
        "{indentation}[Still Valid: {}]\n",
        valid.is_valid()
    ));

    Ok(output)
}

/// This function parses an [`AlgorithmIdentifier`] and returns a [`String`] with the formatted
/// data.
pub fn print_x509_digest_algorithm(algo: &AlgorithmIdentifier, indent: usize) -> String {
    let mut output = String::new();
    let indentation = "    ".repeat(indent);

    let temp = format_oid(&algo.algorithm);
    output.push_str(&format!("{indentation}Object Identifier: {temp}\n"));
    if let Some(params) = &algo.parameters {
        if params.data.is_empty() {
            output.push_str(&format!(
                "{indentation}Algorithm Parameter: {}\n",
                params.tag()
            ));
        } else {
            output.push_str(&format!(
                "{indentation}Algorithm Parameter: {} {:?}\n",
                params.tag(),
                params.data
            ));
        }
    }

    output
}

/// This function parses a [`SubjectPublicKeyInfo`] and returns a [`String`] with the formatted
/// data.
///
/// # Errors
/// Will return an [`X509Error`](crate::Error::X509Error) if it fails to parse the public key info.
pub fn print_x509_public_key_info(
    info: &SubjectPublicKeyInfo,
    indent: usize,
) -> crate::Result<String> {
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    output.push_str(&format!("{indentation}Subject Public Key Info:\n"));
    output.push_str(&print_x509_digest_algorithm(&info.algorithm, indent + 1));
    match info.parsed()? {
        PublicKey::RSA(rsa) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}RSA Public Key: ({} bit, {:#X} exponent)\n",
                rsa.key_size(),
                rsa.try_exponent()?
            ));
            indentation += "    ";
            for line in hex_string(rsa.modulus) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
        PublicKey::EC(ec) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}EC Public Key: ({} bit)\n",
                ec.key_size()
            ));
            indentation += "    ";
            for line in hex_string(ec.data()) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
        PublicKey::DSA(y) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}DSA Public Key: ({} bit)\n",
                8 * y.len()
            ));
            indentation += "    ";
            for line in hex_string(y) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
        PublicKey::GostR3410(y) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}GOST R 34.10-94 Public Key: ({} bit)\n",
                8 * y.len()
            ));
            indentation += "    ";
            for line in hex_string(y) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
        PublicKey::GostR3410_2012(y) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}GOST R 34.10-2012 Public Key: ({} bit)\n",
                8 * y.len()
            ));
            indentation += "    ";
            for line in hex_string(y) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
        PublicKey::Unknown(b) => {
            indentation += "    ";
            output.push_str(&format!(
                "{indentation}Unknown Key Type! ({} bit)\n",
                8 * b.len()
            ));
            indentation += "    ";
            for line in hex_string(b) {
                output.push_str(&format!("{indentation}{line}\n"));
            }
        }
    }

    Ok(output)
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

/// Parses an [`X509Extension`] and returns a [`String`] with the formatted data.
///
/// Currently, it only supports [`AuthorityKeyIdentifier`],
/// [`SubjectKeyIdentifier`](ParsedExtension::SubjectKeyIdentifier), [`KeyUsage`],
/// [`BasicConstraints`], and [`ExtendedKeyUsage`], it will print the debug output of any other
/// extension.
pub fn print_x509_extension(extension: &X509Extension, indent: usize) -> String {
    let parsed = extension.parsed_extension();
    let mut output = String::new();
    let mut indentation = "    ".repeat(indent);

    output.push_str(&format!(
        "{indentation}[critical:{}] {}\n",
        extension.critical,
        get_extension_name(parsed)
    ));

    match parsed {
        ParsedExtension::AuthorityKeyIdentifier(identifier) => {
            output.push_str(&print_x509_authority_key_identifier(identifier, indent + 1));
        }
        ParsedExtension::SubjectKeyIdentifier(identifier) => {
            output.push_str(&print_x509_subject_key_identifier(identifier, indent + 1));
        }
        ParsedExtension::KeyUsage(usage) => {
            output.push_str(&print_x509_key_usage(usage, indent + 1));
        }
        ParsedExtension::BasicConstraints(constraints) => {
            output.push_str(&print_x509_basic_constraints(constraints, indent + 1));
        }
        ParsedExtension::ExtendedKeyUsage(usage) => {
            output.push_str(&print_x509_extended_key_usage(usage, indent + 1));
        }
        x => {
            indentation += "    ";
            output.push_str(&format!("{indentation}{x:?}\n"));
        }
    }

    output
}

fn print_x509_authority_key_identifier(
    identifier: &AuthorityKeyIdentifier,
    indent: usize,
) -> String {
    let mut output = String::new();
    let indentation = "    ".repeat(indent);

    if let Some(key_id) = &identifier.key_identifier {
        output.push_str(&format!(
            "{indentation}Identifier: {}\n",
            octet_string(key_id.0)
        ));
    }
    if let Some(issuer) = &identifier.authority_cert_issuer {
        for name in issuer {
            output.push_str(&format!("{indentation}Issuer: {name}\n"));
        }
    }
    if let Some(serial) = identifier.authority_cert_serial {
        output.push_str(&format!("{indentation}Serial: {}\n", octet_string(serial)));
    }

    output
}

fn print_x509_subject_key_identifier(identifier: &KeyIdentifier, indent: usize) -> String {
    format!(
        "{}Identifier: {}\n",
        "    ".repeat(indent),
        octet_string(identifier.0)
    )
}

fn print_x509_key_usage(usage: &KeyUsage, indent: usize) -> String {
    format!("{}Key Usage: {usage}\n", "    ".repeat(indent))
}

fn print_x509_basic_constraints(constraints: &BasicConstraints, indent: usize) -> String {
    let mut output = String::new();
    let indentation = "    ".repeat(indent);

    output.push_str(&format!(
        "{indentation}Certificate Authority: {}\n",
        constraints.ca
    ));
    if constraints.ca {
        if let Some(value) = constraints.path_len_constraint {
            output.push_str(&format!("{indentation}Certification Path Limit: {value}\n"));
        }
    }

    output
}

fn print_x509_extended_key_usage(usage: &ExtendedKeyUsage, indent: usize) -> String {
    let mut output = String::new();
    let indentation = "    ".repeat(indent);

    if usage.any {
        output.push_str(&format!("{indentation}Any: true\n"));
    }
    if usage.server_auth {
        output.push_str(&format!("{indentation}Server Authentication: true\n"));
    }
    if usage.client_auth {
        output.push_str(&format!("{indentation}Client Authentication: true\n"));
    }
    if usage.code_signing {
        output.push_str(&format!("{indentation}Code Signing: true\n"));
    }
    if usage.email_protection {
        output.push_str(&format!("{indentation}Email Protection: true\n"));
    }
    if usage.time_stamping {
        output.push_str(&format!("{indentation}Time Stamping: true\n"));
    }
    if usage.ocsp_signing {
        output.push_str(&format!("{indentation}Certificate Status Signing: true\n"));
    }
    for oid in &usage.other {
        output.push_str(&format!("{indentation}Other: {oid}\n"));
    }

    output
}
*/

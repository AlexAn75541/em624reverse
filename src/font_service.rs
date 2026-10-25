use std::io::Read;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSourceOption {
    Auto,
    GoogleFonts,
    Fontshare,
}

impl FontSourceOption {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Auto => "Auto (Google + Fontshare)",
            Self::GoogleFonts => "Google Fonts (1500+ fonts)",
            Self::Fontshare => "Fontshare (Free Quality Fonts)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedFont {
    pub display_name: String,
    pub family_name: String,
    pub source_name: &'static str,
    pub download_url: String,
    pub font_bytes: Vec<u8>,
    pub file_size_kb: usize,
}

#[derive(Clone, Debug)]
pub enum FontCheckState {
    Idle,
    Checking(String),
    Available(VerifiedFont),
    NotFound(String),
    Error(String),
}

/// Validates that binary data begins with valid TrueType or OpenType header
pub fn validate_font_data(data: &[u8]) -> Result<(), String> {
    if data.len() < 12 {
        return Err("Payload too small to be a valid font (< 12 bytes)".to_string());
    }

    let magic = &data[0..4];
    let is_valid = magic == [0x00, 0x01, 0x00, 0x00] // TrueType 1.0
        || magic == b"OTTO"                           // OpenType CFF
        || magic == b"true"                           // Apple TrueType
        || magic == b"ttcf"; // TrueType Collection

    if !is_valid {
        return Err(format!(
            "Invalid font header magic: {:02x?} (expected TrueType or OpenType)",
            magic
        ));
    }

    let num_tables = u16::from_be_bytes([data[4], data[5]]);
    if num_tables == 0 || num_tables > 100 {
        return Err(format!("Unreasonable table count in font: {}", num_tables));
    }

    Ok(())
}

fn to_title_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c.is_whitespace() || c == '-' || c == '_' {
            result.push(c);
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.extend(c.to_lowercase());
        }
    }
    result
}

fn to_kebab_case(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

fn extract_ttf_url(css: &str) -> Option<String> {
    let mut idx = 0;
    while let Some(pos) = css[idx..].find("url(") {
        let actual_pos = idx + pos;
        if let Some(end) = css[actual_pos..].find(')') {
            let actual_end = actual_pos + end;
            let raw = css[actual_pos + 4..actual_end]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');
            if raw.contains(".ttf") || raw.contains(".otf") {
                return Some(raw.to_string());
            }
            idx = actual_end + 1;
        } else {
            break;
        }
    }
    None
}

fn fetch_google_fonts(name: &str) -> Result<(String, String, Vec<u8>), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent("curl/7.68.0")
        .build();

    let mut candidates = vec![name.trim().to_string()];
    let title_cased = to_title_case(name.trim());
    if title_cased != name.trim() {
        candidates.push(title_cased);
    }

    for cand in candidates {
        let enc = url_encode(&cand);
        let urls = [
            format!("https://fonts.googleapis.com/css?family={enc}:400,700"),
            format!("https://fonts.googleapis.com/css?family={enc}"),
        ];

        for css_url in urls {
            let resp = match agent.get(&css_url).call() {
                Ok(r) => r,
                Err(_) => continue,
            };

            if resp.status() == 200 {
                if let Ok(css_text) = resp.into_string() {
                    if let Some(ttf_url) = extract_ttf_url(&css_text) {
                        if let Ok(font_resp) = agent.get(&ttf_url).call() {
                            if font_resp.status() == 200 {
                                let mut bytes = Vec::new();
                                if font_resp
                                    .into_reader()
                                    .take(10 * 1024 * 1024)
                                    .read_to_end(&mut bytes)
                                    .is_ok()
                                    && validate_font_data(&bytes).is_ok()
                                {
                                    return Ok((cand, ttf_url, bytes));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "Font '{}' not found on Google Fonts. Verify spelling or case.",
        name
    ))
}

fn fetch_fontshare(name: &str) -> Result<(String, String, Vec<u8>), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent("curl/7.68.0")
        .build();

    let slug = to_kebab_case(name.trim());
    let css_url = format!("https://api.fontshare.com/v2/css?f[]={slug}@400,700");

    let resp = agent
        .get(&css_url)
        .call()
        .map_err(|e| format!("Fontshare connection error: {e}"))?;

    if resp.status() == 200 {
        if let Ok(css_text) = resp.into_string() {
            if let Some(mut ttf_url) = extract_ttf_url(&css_text) {
                if ttf_url.starts_with("//") {
                    ttf_url = format!("https:{ttf_url}");
                }
                if let Ok(font_resp) = agent.get(&ttf_url).call() {
                    if font_resp.status() == 200 {
                        let mut bytes = Vec::new();
                        if font_resp
                            .into_reader()
                            .take(10 * 1024 * 1024)
                            .read_to_end(&mut bytes)
                            .is_ok()
                            && validate_font_data(&bytes).is_ok()
                        {
                            return Ok((name.trim().to_string(), ttf_url, bytes));
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "Font '{}' not found on Fontshare repository.",
        name
    ))
}

fn fetch_direct_url(url: &str) -> Result<(String, String, Vec<u8>), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .user_agent("curl/7.68.0")
        .build();

    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("Direct URL fetch error: {e}"))?;

    if resp.status() == 200 {
        let mut bytes = Vec::new();
        resp.into_reader()
            .take(15 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;

        validate_font_data(&bytes)?;

        let filename = url.split('?').next().unwrap_or(url);
        let name = filename
            .rsplit('/')
            .next()
            .unwrap_or("CustomFont")
            .trim_end_matches(".ttf")
            .trim_end_matches(".otf");

        return Ok((name.to_string(), url.to_string(), bytes));
    }

    Err(format!("HTTP error {}", resp.status()))
}

/// Verifies and downloads font binary from remote public repositories without server-side setups
pub fn verify_and_fetch_font(
    input: &str,
    source_opt: FontSourceOption,
) -> Result<VerifiedFont, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Font name cannot be blank. Please enter a valid font name.".to_string());
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let (name, url, bytes) = fetch_direct_url(trimmed)?;
        let size_kb = bytes.len() / 1024;
        return Ok(VerifiedFont {
            display_name: name.clone(),
            family_name: name,
            source_name: "Direct URL",
            download_url: url,
            font_bytes: bytes,
            file_size_kb: size_kb,
        });
    }

    match source_opt {
        FontSourceOption::GoogleFonts => {
            let (name, url, bytes) = fetch_google_fonts(trimmed)?;
            let size_kb = bytes.len() / 1024;
            Ok(VerifiedFont {
                display_name: name.clone(),
                family_name: name,
                source_name: "Google Fonts",
                download_url: url,
                font_bytes: bytes,
                file_size_kb: size_kb,
            })
        }
        FontSourceOption::Fontshare => {
            let (name, url, bytes) = fetch_fontshare(trimmed)?;
            let size_kb = bytes.len() / 1024;
            Ok(VerifiedFont {
                display_name: name.clone(),
                family_name: name,
                source_name: "Fontshare",
                download_url: url,
                font_bytes: bytes,
                file_size_kb: size_kb,
            })
        }
        FontSourceOption::Auto => {
            if let Ok((name, url, bytes)) = fetch_google_fonts(trimmed) {
                let size_kb = bytes.len() / 1024;
                return Ok(VerifiedFont {
                    display_name: name.clone(),
                    family_name: name,
                    source_name: "Google Fonts",
                    download_url: url,
                    font_bytes: bytes,
                    file_size_kb: size_kb,
                });
            }

            if let Ok((name, url, bytes)) = fetch_fontshare(trimmed) {
                let size_kb = bytes.len() / 1024;
                return Ok(VerifiedFont {
                    display_name: name.clone(),
                    family_name: name,
                    source_name: "Fontshare",
                    download_url: url,
                    font_bytes: bytes,
                    file_size_kb: size_kb,
                });
            }

            Err(format!(
                "Font '{}' was not found on Google Fonts or Fontshare. Verify spelling, try standard Title Case (e.g., 'Inter', 'General Sans'), or paste a direct TTF link.",
                trimmed
            ))
        }
    }
}

/// Spawns background thread for non-blocking font verification
pub fn start_verify_font(
    input: String,
    source_opt: FontSourceOption,
) -> Receiver<Result<VerifiedFont, String>> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let res = verify_and_fetch_font(&input, source_opt);
        let _ = tx.send(res);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("General Sans"), "general-sans");
        assert_eq!(to_kebab_case("Fira Code"), "fira-code");
        assert_eq!(to_kebab_case("Clash Display"), "clash-display");
        assert_eq!(to_kebab_case("Inter"), "inter");
    }

    #[test]
    fn test_validate_font_data() {
        let invalid = [0u8; 10];
        assert!(validate_font_data(&invalid).is_err());

        // Valid TTF magic: 0x00, 0x01, 0x00, 0x00, with num_tables = 5
        let mut ttf = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x05];
        ttf.extend_from_slice(&[0u8; 100]);
        assert!(validate_font_data(&ttf).is_ok());

        // Valid OTF magic: 'OTTO', with num_tables = 5
        let mut otf = b"OTTO".to_vec();
        otf.extend_from_slice(&[0x00, 0x05]);
        otf.extend_from_slice(&[0u8; 100]);
        assert!(validate_font_data(&otf).is_ok());
    }

    #[test]
    fn test_font_source_display() {
        assert_eq!(
            FontSourceOption::Auto.display_name(),
            "Auto (Google + Fontshare)"
        );
        assert_eq!(
            FontSourceOption::GoogleFonts.display_name(),
            "Google Fonts (1500+ fonts)"
        );
        assert_eq!(
            FontSourceOption::Fontshare.display_name(),
            "Fontshare (Free Quality Fonts)"
        );
    }
}

use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(super) fn public_https_host(url: &str) -> Option<&str> {
    public_https_authority_host(public_https_authority_text(url)?)
}

fn public_https_authority(url: &str) -> Option<(&str, Option<u16>)> {
    public_https_authority_parts(public_https_authority_text(url)?)
}

fn public_https_url_rest(url: &str) -> Option<&str> {
    if url
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return None;
    }
    url.strip_prefix("https://")
}

fn public_https_authority_text(url: &str) -> Option<&str> {
    let rest = public_https_url_rest(url)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some(&rest[..authority_end])
}

fn public_https_authority_host(authority: &str) -> Option<&str> {
    public_https_authority_parts(authority).map(|(host, _port)| host)
}

fn public_https_authority_parts(authority: &str) -> Option<(&str, Option<u16>)> {
    if authority.is_empty()
        || authority.contains('@')
        || authority.contains(['/', '?', '#', '\\'])
        || authority
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return None;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        let host = &bracketed[..end];
        let suffix = &bracketed[end + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            Some(parse_public_https_port(suffix.strip_prefix(':')?)?)
        };
        if host.is_empty() || host.parse::<Ipv6Addr>().is_err() {
            return None;
        }
        Some((host, port))
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(host, port)| (host, Some(port)));
        if host.is_empty()
            || host.contains(['[', ']', ':'])
            || port.is_some_and(|port| port.contains(':'))
        {
            return None;
        }
        let port = match port {
            Some(port) => Some(parse_public_https_port(port)?),
            None => None,
        };
        if host.parse::<Ipv4Addr>().is_err() && !public_dns_host_is_well_formed(host) {
            return None;
        }
        Some((host, port))
    }
}

fn parse_public_https_port(port: &str) -> Option<u16> {
    if port.is_empty() || port.len() > 5 || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    port.parse::<u16>().ok().filter(|parsed| *parsed != 0)
}

pub(super) fn public_https_authorities_match(left: &str, right: &str) -> bool {
    let Some((left_host, left_port)) = public_https_authority(left) else {
        return false;
    };
    let Some((right_host, right_port)) = public_https_authority(right) else {
        return false;
    };
    public_authority_host_key(left_host) == public_authority_host_key(right_host)
        && left_port.unwrap_or(443) == right_port.unwrap_or(443)
}

pub(super) fn public_https_url_key(url: &str) -> Option<String> {
    let (host, port) = public_https_authority(url)?;
    let path = public_https_path(url)?;
    Some(format!(
        "{}:{}{}",
        public_authority_host_key(host),
        port.unwrap_or(443),
        path
    ))
}

fn public_authority_host_key(host: &str) -> String {
    match host.parse::<IpAddr>() {
        Ok(ip) => ip.to_string(),
        Err(_) => host.trim_end_matches('.').to_ascii_lowercase(),
    }
}

pub(super) fn public_https_path(url: &str) -> Option<&str> {
    let rest = public_https_url_rest(url)?;
    let path_start = rest.find('/')?;
    let path = &rest[path_start..];
    if path.contains(['?', '#']) {
        return None;
    }
    (!path.is_empty()).then_some(path)
}

pub(super) fn public_host_is_external(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let lowercase_host = host.to_ascii_lowercase();
    if lowercase_host == "localhost"
        || lowercase_host.ends_with(".local")
        || special_use_dns_name(&lowercase_host)
    {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => public_ipv4_is_external(ip),
        Ok(IpAddr::V6(ip)) => public_ipv6_is_external(ip),
        Err(_) => public_dns_host_is_well_formed(host),
    }
}

fn special_use_dns_name(host: &str) -> bool {
    host == "local"
        || host == "test"
        || host == "example"
        || host == "invalid"
        || host == "example.com"
        || host == "example.net"
        || host == "example.org"
        || host.ends_with(".example.com")
        || host.ends_with(".example.net")
        || host.ends_with(".example.org")
        || host.ends_with(".localhost")
        || host.ends_with(".test")
        || host.ends_with(".example")
        || host.ends_with(".invalid")
}

fn public_dns_host_is_well_formed(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let mut label_count = 0;
    let mut labels = host.split('.').peekable();
    while let Some(label) = labels.next() {
        label_count += 1;
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let bytes = label.as_bytes();
        if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
            return false;
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return false;
        }
        if labels.peek().is_none() && !bytes.iter().any(|byte| byte.is_ascii_alphabetic()) {
            return false;
        }
    }
    label_count >= 2
}

fn public_ipv4_is_external(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();
    let is_shared_address_space = a == 100 && (64..=127).contains(&b);
    let is_protocol_assignment = a == 192 && b == 0 && c == 0;
    let is_documentation = (a == 192 && b == 0 && c == 2)
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113);
    let is_benchmarking = a == 198 && (b == 18 || b == 19);
    let is_multicast = (224..=239).contains(&a);
    let is_reserved_or_broadcast = (240..=255).contains(&a);
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || is_shared_address_space
        || is_protocol_assignment
        || is_documentation
        || is_benchmarking
        || is_multicast
        || is_reserved_or_broadcast)
}

fn public_ipv6_is_external(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.is_multicast()
        || is_documentation)
}

pub(super) fn public_evidence_uri_is_external(uri: &str) -> bool {
    if let Some(host) = public_https_host(uri) {
        return public_host_is_external(host)
            && public_https_path(uri).is_some_and(|path| path.len() > 1);
    }
    content_addressed_uri_has_identifier(uri, "ipfs://")
        || content_addressed_uri_has_identifier(uri, "ar://")
}

fn content_addressed_uri_has_identifier(uri: &str, scheme: &str) -> bool {
    if uri
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        || uri.contains(['?', '#', '\\'])
    {
        return false;
    }
    let Some(rest) = uri.strip_prefix(scheme) else {
        return false;
    };
    match rest.split_once('/') {
        Some((identifier, path)) => {
            content_addressed_identifier_is_well_formed(identifier)
                && !path.is_empty()
                && path
                    .split('/')
                    .all(content_addressed_path_segment_is_well_formed)
        }
        None => content_addressed_identifier_is_well_formed(rest),
    }
}

fn content_addressed_identifier_is_well_formed(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier != "."
        && identifier != ".."
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn content_addressed_path_segment_is_well_formed(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
}

pub(crate) fn public_network_runtime_multiaddr_is_external(address: &Multiaddr) -> bool {
    let mut saw_public_address = false;
    let mut saw_tcp_listen_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(ip) => {
                if !public_host_is_external(&ip.to_string()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Ip6(ip) => {
                if !public_host_is_external(&ip.to_string()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Dns(host) | Protocol::Dns4(host) | Protocol::Dns6(host) => {
                if !public_host_is_external(host.as_ref()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Tcp(port) if port != 0 => saw_tcp_listen_port = true,
            Protocol::Tcp(_) => return false,
            _ => {}
        }
    }
    saw_public_address && saw_tcp_listen_port
}

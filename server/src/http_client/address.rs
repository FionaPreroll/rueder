//! Which addresses rueder fetches from.
//!
//! Users enter addresses, and feeds and web pages redirect to others. None of
//! them may lead into the network rueder itself runs in: to the machine itself,
//! to private networks, or to the metadata service of a cloud provider. This
//! module decides that for one IP address; the client applies it to every address
//! it is about to connect to.
//!
//! The lists follow the IANA registries of special-purpose addresses. What is not
//! known to be an address of the public internet is refused.

use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

/// Says why rueder does not fetch from the address, or `None` for an address of
/// the public internet.
pub fn internal(ip: IpAddr) -> Option<&'static str> {
    match ip {
        IpAddr::V4(ip) => internal_v4(ip),
        IpAddr::V6(ip) => internal_v6(ip),
    }
}

fn internal_v4(ip: Ipv4Addr) -> Option<&'static str> {
    let [a, b, c, _] = ip.octets();
    Some(match (a, b, c) {
        (0, ..) => "this network",
        (10, ..) | (172, 16..=31, _) | (192, 168, _) => "private",
        (100, 64..=127, _) => "shared (carrier NAT)",
        (127, ..) => "loopback",
        // Among them 169.254.169.254, the metadata service of cloud providers.
        (169, 254, _) => "link-local",
        (192, 0, 0) | (192, 88, 99) => "reserved",
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113) => "documentation",
        (198, 18..=19, _) => "benchmarking",
        (224..=239, ..) => "multicast",
        // The broadcast address is the last of these.
        (240..=255, ..) => "reserved",
        _ => return None,
    })
}

fn internal_v6(ip: Ipv6Addr) -> Option<&'static str> {
    let segments = ip.segments();
    // An IPv4 address written as IPv6 is that IPv4 address: the mapped form the
    // system itself connects through, and the well-known prefix of NAT64.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return internal_v4(v4);
    }
    if segments[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
        let [a, b] = segments[6].to_be_bytes();
        let [c, d] = segments[7].to_be_bytes();
        return internal_v4(Ipv4Addr::new(a, b, c, d));
    }
    // The public internet is 2000::/3, minus what is set aside in there.
    if segments[0] & 0xe000 != 0x2000 {
        return Some(match segments[0] {
            0 if ip.is_loopback() => "loopback",
            0 if ip.is_unspecified() => "this network",
            0xfc00..=0xfdff => "private",
            0xfe80..=0xfebf => "link-local",
            0xff00..=0xffff => "multicast",
            _ => "reserved",
        });
    }
    match (segments[0], segments[1]) {
        // Protocol assignments, among them Teredo.
        (0x2001, 0..=0x01ff) => Some("reserved"),
        (0x2001, 0x0db8) | (0x3fff, 0..=0x0fff) => Some("documentation"),
        // 6to4 carries an IPv4 address; nothing worth reading lives there.
        (0x2002, _) => Some("reserved"),
        _ => None,
    }
}

/// A network in the usual notation, `10.0.0.0/8` or `fd00::/8`, or one address.
/// For the networks an operator opens on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Net {
    addr: IpAddr,
    prefix: u8,
}

impl Net {
    /// Whether the address lies in the network. An IPv4 address never lies in an
    /// IPv6 network and the other way round.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let shift = 32 - u32::from(self.prefix);
                shift == 32 || net.to_bits() >> shift == ip.to_bits() >> shift
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let shift = 128 - u32::from(self.prefix);
                shift == 128 || net.to_bits() >> shift == ip.to_bits() >> shift
            }
            _ => false,
        }
    }
}

impl FromStr for Net {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (addr, prefix) = match text.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix)),
            None => (text, None),
        };
        let addr: IpAddr = addr
            .trim()
            .parse()
            .map_err(|_| format!("{text:?} is not a network like 10.0.0.0/8"))?;
        let longest = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            None => longest,
            Some(prefix) => prefix
                .trim()
                .parse()
                .ok()
                .filter(|&prefix| prefix <= longest)
                .ok_or_else(|| format!("{text:?} has no prefix length up to {longest}"))?,
        };
        Ok(Self { addr, prefix })
    }
}

impl fmt::Display for Net {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

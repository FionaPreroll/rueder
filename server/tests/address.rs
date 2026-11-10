//! The rules for addresses of the hardened HTTP client: which addresses count as
//! internal, and that no way of writing one gets it past the check.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};

use proptest::prelude::*;
use rueder::http_client::{Config, Error, HttpClient, Net, address::internal};

static CLIENT: LazyLock<HttpClient> = LazyLock::new(|| HttpClient::new(Config::default()).unwrap());

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn every_internal_range_of_ipv4_is_refused_from_its_first_to_its_last_address() {
    let ranges = [
        ("0.0.0.0", "0.255.255.255", "this network"),
        ("10.0.0.0", "10.255.255.255", "private"),
        ("100.64.0.0", "100.127.255.255", "shared (carrier NAT)"),
        ("127.0.0.0", "127.255.255.255", "loopback"),
        ("169.254.0.0", "169.254.255.255", "link-local"),
        ("172.16.0.0", "172.31.255.255", "private"),
        ("192.0.0.0", "192.0.0.255", "reserved"),
        ("192.0.2.0", "192.0.2.255", "documentation"),
        ("192.88.99.0", "192.88.99.255", "reserved"),
        ("192.168.0.0", "192.168.255.255", "private"),
        ("198.18.0.0", "198.19.255.255", "benchmarking"),
        ("198.51.100.0", "198.51.100.255", "documentation"),
        ("203.0.113.0", "203.0.113.255", "documentation"),
        ("224.0.0.0", "239.255.255.255", "multicast"),
        ("240.0.0.0", "255.255.255.255", "reserved"),
    ];
    for (first, last, why) in ranges {
        assert_eq!(internal(ip(first)), Some(why), "{first}");
        assert_eq!(internal(ip(last)), Some(why), "{last}");
    }
    // The metadata service of cloud providers.
    assert_eq!(internal(ip("169.254.169.254")), Some("link-local"));
}

#[test]
fn the_addresses_next_to_the_internal_ranges_of_ipv4_are_public() {
    let public = [
        "1.0.0.0",
        "9.255.255.255",
        "11.0.0.0",
        "100.63.255.255",
        "100.128.0.0",
        "126.255.255.255",
        "128.0.0.0",
        "169.253.255.255",
        "169.255.0.0",
        "172.15.255.255",
        "172.32.0.0",
        "192.0.1.0",
        "192.0.3.0",
        "192.88.98.255",
        "192.88.100.0",
        "192.167.255.255",
        "192.169.0.0",
        "198.17.255.255",
        "198.20.0.0",
        "198.51.99.255",
        "198.51.101.0",
        "203.0.112.255",
        "203.0.114.0",
        "223.255.255.255",
        "8.8.8.8",
    ];
    for addr in public {
        assert_eq!(internal(ip(addr)), None, "{addr}");
    }
}

#[test]
fn every_internal_range_of_ipv6_is_refused() {
    let cases = [
        ("::", "this network"),
        ("::1", "loopback"),
        ("::2", "reserved"),
        // IPv4 written as IPv6 is judged as that IPv4 address.
        ("::ffff:127.0.0.1", "loopback"),
        ("::ffff:10.0.0.1", "private"),
        ("::ffff:169.254.169.254", "link-local"),
        ("64:ff9b::10.0.0.1", "private"),
        ("64:ff9b::7f00:1", "loopback"),
        // The old way of writing IPv4 in IPv6, and translation prefixes for local use.
        ("::127.0.0.1", "reserved"),
        ("::ffff:0:127.0.0.1", "reserved"),
        ("64:ff9b:1::1", "reserved"),
        ("100::1", "reserved"),
        ("fc00::1", "private"),
        ("fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff", "private"),
        ("fe80::1", "link-local"),
        ("febf::1", "link-local"),
        ("fec0::1", "reserved"),
        ("ff02::1", "multicast"),
        ("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff", "multicast"),
        ("4000::1", "reserved"),
        // Inside 2000::/3: Teredo, other protocol assignments, documentation, 6to4.
        ("2001::1", "reserved"),
        ("2001:1ff:ffff::1", "reserved"),
        ("2001:db8::1", "documentation"),
        ("3fff:fff::1", "documentation"),
        ("2002:7f00:1::1", "reserved"),
        ("2002:808:808::1", "reserved"),
    ];
    for (addr, why) in cases {
        assert_eq!(internal(ip(addr)), Some(why), "{addr}");
    }
}

#[test]
fn public_addresses_of_ipv6_are_let_through() {
    let public = [
        "2606:4700:4700::1111",
        "2a00:1450:4001:81b::200e",
        "2001:200::1",
        "2001:db7:ffff::1",
        "2001:db9::1",
        "2003::1",
        "3fff:1000::1",
        "3ffe::1",
        // IPv4 written as IPv6, where the IPv4 address is public.
        "::ffff:8.8.8.8",
        "64:ff9b::8.8.8.8",
    ];
    for addr in public {
        assert_eq!(internal(ip(addr)), None, "{addr}");
    }
}

#[test]
fn addresses_as_people_and_attackers_write_them() {
    let refused = [
        // Other schemes, and no scheme.
        "file:///etc/passwd",
        "ftp://example.org/feed.xml",
        "gopher://example.org/",
        "javascript:alert(1)",
        "data:text/plain,hello",
        "example.org/feed.xml",
        "//example.org/feed.xml",
        "",
        "http://",
        // The own machine and network, plainly.
        "http://127.0.0.1/",
        "http://127.0.0.1:5432/",
        "http://[::1]/",
        "http://0.0.0.0/",
        "http://10.0.0.1/admin",
        "http://192.168.1.1/",
        "http://169.254.169.254/latest/meta-data/",
        // The same in other notations.
        "http://2130706433/",
        "http://0x7f000001/",
        "http://0x7f.0.0.1/",
        "http://017700000001/",
        "http://0177.0.0.01/",
        "http://127.1/",
        "http://127.0.1/",
        "http://0/",
        "http://127.0.0.1./",
        "http://[::ffff:127.0.0.1]/",
        "http://[::ffff:7f00:1]/",
        "http://[0:0:0:0:0:ffff:a00:1]/",
        "http://[64:ff9b::a00:1]/",
        // Digits and dots that only look different.
        "http://１２７.０.０.１/",
        "http://127。0。0。1/",
        "http://①②⑦.⓪.⓪.①/",
        // The host is what comes after the last @.
        "http://example.org@127.0.0.1/",
        "http://example.org:80@10.0.0.1/",
        "http://8.8.8.8@[::1]/",
        // A backslash ends the host like a slash; the host is the internal one.
        "http://127.0.0.1\\@example.org/",
        // A zone only makes sense for link-local addresses.
        "http://[fe80::1%25eth0]/",
    ];
    for address in refused {
        assert!(
            CLIENT.parse(address).is_err(),
            "{address:?} must be refused"
        );
    }

    let fine = [
        ("http://example.org/feed.xml", "example.org"),
        ("https://example.org:8443/feed.xml", "example.org"),
        ("  https://example.org/feed.xml\n", "example.org"),
        ("HTTP://EXAMPLE.ORG/", "example.org"),
        ("http://8.8.8.8/", "8.8.8.8"),
        ("http://134744072/", "8.8.8.8"),
        ("http://[2606:4700:4700::1111]/", "[2606:4700:4700::1111]"),
        ("http://user:secret@example.org/feed.xml", "example.org"),
        ("http://127.0.0.1@example.org/", "example.org"),
        ("http://example.org\\@127.0.0.1/", "example.org"),
        ("http://example.org#@127.0.0.1/", "example.org"),
        // Slashes too many are skipped, as browsers do.
        ("http:///example.org/feed.xml", "example.org"),
        // A name is judged when it is looked up, whatever it is called.
        ("http://localhost/", "localhost"),
    ];
    for (address, host) in fine {
        let url = CLIENT
            .parse(address)
            .unwrap_or_else(|err| panic!("{address:?}: {err}"));
        assert_eq!(url.host_str(), Some(host), "{address:?}");
        assert_eq!(url.fragment(), None, "{address:?}");
    }
}

#[test]
fn a_refusal_names_the_host_and_never_the_password() {
    let err = CLIENT.parse("http://user:secret@10.0.0.1/").unwrap_err();
    assert_eq!(
        err,
        Error::Internal {
            host: "10.0.0.1".to_owned(),
            addr: ip("10.0.0.1"),
            why: "private"
        }
    );
    assert!(!err.to_string().contains("secret"), "{err}");
    assert!(err.to_string().contains("10.0.0.1"), "{err}");
}

#[test]
fn the_operator_can_open_a_network_and_nothing_next_to_it() {
    let config = Config {
        allow: vec![
            "10.1.0.0/16".parse().unwrap(),
            "fd12:3456::/32".parse().unwrap(),
        ],
        ..Config::default()
    };
    let client = HttpClient::new(config).unwrap();
    for open in [
        "http://10.1.0.0/",
        "http://10.1.255.255/",
        "http://[::ffff:10.1.2.3]/",
        "http://[fd12:3456::1]/",
    ] {
        assert!(client.parse(open).is_ok(), "{open}");
    }
    for closed in [
        "http://10.0.255.255/",
        "http://10.2.0.0/",
        "http://127.0.0.1/",
        "http://[fd12:3457::1]/",
    ] {
        assert!(client.parse(closed).is_err(), "{closed}");
    }
}

#[test]
fn networks_are_read_and_written_the_usual_way() {
    let net: Net = "192.168.1.0/24".parse().unwrap();
    assert!(net.contains(ip("192.168.1.0")));
    assert!(net.contains(ip("192.168.1.255")));
    assert!(!net.contains(ip("192.168.2.0")));
    assert!(!net.contains(ip("::ffff:192.168.1.1")));
    assert_eq!(net.to_string(), "192.168.1.0/24");

    let one: Net = "127.0.0.1".parse().unwrap();
    assert!(one.contains(ip("127.0.0.1")));
    assert!(!one.contains(ip("127.0.0.2")));
    let one: Net = "::1".parse().unwrap();
    assert!(one.contains(ip("::1")));
    assert!(!one.contains(ip("::2")));

    let all: Net = " 0.0.0.0/0 ".parse().unwrap();
    assert!(all.contains(ip("8.8.8.8")));
    assert!(!all.contains(ip("::1")));
    let all: Net = "::/0".parse().unwrap();
    assert!(all.contains(ip("2606:4700::1")));
    assert!(!all.contains(ip("8.8.8.8")));

    for wrong in [
        "",
        "10.0.0.0/33",
        "::/129",
        "10.0.0/8",
        "example.org/8",
        "10.0.0.0/-1",
        "10.0.0.0/",
    ] {
        assert!(wrong.parse::<Net>().is_err(), "{wrong:?}");
    }
}

// What follows compares the check with a second, plain way of deciding: tables of
// networks as numbers, for random addresses in every way of writing them.

const INTERNAL_V4: [(&str, u32); 15] = [
    ("0.0.0.0", 8),
    ("10.0.0.0", 8),
    ("100.64.0.0", 10),
    ("127.0.0.0", 8),
    ("169.254.0.0", 16),
    ("172.16.0.0", 12),
    ("192.0.0.0", 24),
    ("192.0.2.0", 24),
    ("192.88.99.0", 24),
    ("192.168.0.0", 16),
    ("198.18.0.0", 15),
    ("198.51.100.0", 24),
    ("203.0.113.0", 24),
    ("224.0.0.0", 4),
    ("240.0.0.0", 4),
];

fn public_v4(addr: u32) -> bool {
    !INTERNAL_V4.iter().any(|&(net, prefix)| {
        let net: Ipv4Addr = net.parse().unwrap();
        net.to_bits() >> (32 - prefix) == addr >> (32 - prefix)
    })
}

fn within_v6(addr: u128, net: &str, prefix: u32) -> bool {
    let net: Ipv6Addr = net.parse().unwrap();
    net.to_bits() >> (128 - prefix) == addr >> (128 - prefix)
}

fn public_v6(addr: u128) -> bool {
    if within_v6(addr, "::ffff:0:0", 96) || within_v6(addr, "64:ff9b::", 96) {
        return public_v4(addr as u32);
    }
    within_v6(addr, "2000::", 3)
        && !within_v6(addr, "2001::", 23)
        && !within_v6(addr, "2001:db8::", 32)
        && !within_v6(addr, "2002::", 16)
        && !within_v6(addr, "3fff::", 20)
}

/// Any IPv4 address, with the small internal networks drawn often enough to
/// be met.
fn any_v4() -> impl Strategy<Value = u32> {
    prop_oneof![
        any::<u32>(),
        (0..INTERNAL_V4.len(), any::<u32>()).prop_map(|(range, rest)| {
            let (net, prefix) = INTERNAL_V4[range];
            let net: Ipv4Addr = net.parse().unwrap();
            net.to_bits() | (rest >> prefix)
        }),
    ]
}

/// Any IPv6 address, with the networks the rules name drawn often.
fn any_v6() -> impl Strategy<Value = u128> {
    const NETS: [(&str, u32); 14] = [
        ("::", 96),
        ("::ffff:0:0", 96),
        ("64:ff9b::", 96),
        ("64:ff9b:1::", 48),
        ("100::", 64),
        ("2000::", 3),
        ("2001::", 23),
        ("2001:db8::", 32),
        ("2002::", 16),
        ("3fff::", 20),
        ("fc00::", 7),
        ("fe80::", 10),
        ("fec0::", 10),
        ("ff00::", 8),
    ];
    prop_oneof![
        any::<u128>(),
        (0..NETS.len(), any::<u128>(), any_v4()).prop_map(|(range, rest, v4)| {
            let (net, prefix) = NETS[range];
            let net: Ipv6Addr = net.parse().unwrap();
            let addr = net.to_bits() | (rest >> prefix);
            // Where an IPv4 address is carried, carry an interesting one.
            if prefix == 96 {
                addr & !0xffff_ffff | u128::from(v4)
            } else {
                addr
            }
        }),
    ]
}

/// The ways to write the IPv4 address as a host that end up at that address.
fn host_v4(addr: u32, notation: usize) -> String {
    let [a, b, c, d] = addr.to_be_bytes();
    match notation {
        0 => format!("{a}.{b}.{c}.{d}"),
        1 => format!("{addr}"),
        2 => format!("0x{addr:x}"),
        3 => format!("0X{addr:X}"),
        4 => format!("0{addr:o}"),
        5 => format!("0{a:o}.0{b:o}.0{c:o}.0{d:o}"),
        6 => format!("0x{a:x}.0x{b:x}.0x{c:x}.0x{d:x}"),
        7 => format!("{a}.0x{b:x}.0{c:o}.{d}"),
        8 => format!("{a}.{}", addr & 0x00ff_ffff),
        9 => format!("{a}.{b}.{}", addr & 0xffff),
        10 => format!("{a}.{b}.{c}.{d}."),
        11 => format!("[::ffff:{a}.{b}.{c}.{d}]"),
        12 => format!("[::ffff:{:x}:{:x}]", addr >> 16, addr & 0xffff),
        13 => format!("[0:0:0:0:0:FFFF:{:X}:{:X}]", addr >> 16, addr & 0xffff),
        14 => format!("[64:ff9b::{a}.{b}.{c}.{d}]"),
        _ => format!("[64:ff9b::{:x}:{:x}]", addr >> 16, addr & 0xffff),
    }
}

/// What may stand around the host without changing where the address leads.
fn address(host: &str, decoration: usize, port: Option<u16>) -> String {
    let scheme = if decoration & 1 == 0 { "http" } else { "https" };
    let userinfo = match (decoration >> 1) % 5 {
        0 => "",
        1 => "user@",
        2 => "user:secret@",
        3 => "8.8.8.8@",
        _ => "example.org:80@",
    };
    let port = port.map(|port| format!(":{port}")).unwrap_or_default();
    let rest = match (decoration >> 4) % 4 {
        0 => "",
        1 => "/feed.xml",
        2 => "/?next=http://8.8.8.8/",
        _ => "#@8.8.8.8/",
    };
    format!("{scheme}://{userinfo}{host}{port}{rest}")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    #[test]
    fn no_way_of_writing_an_ipv4_address_changes_the_verdict(
        addr in any_v4(),
        notation in 0usize..16,
        decoration in any::<usize>(),
        port in proptest::option::of(any::<u16>()),
    ) {
        let address = address(&host_v4(addr, notation), decoration, port);
        let verdict = CLIENT.parse(&address);
        prop_assert_eq!(verdict.is_ok(), public_v4(addr), "{}: {:?}", address, verdict);
        if let Ok(url) = verdict {
            // And the host that would be connected to is the one that was judged.
            let host = url.host_str().unwrap().trim_matches(['[', ']']).parse::<IpAddr>().unwrap();
            let judged = match host {
                IpAddr::V4(host) => host.to_bits(),
                // Written as IPv6, the IPv4 address is the last 32 bits.
                IpAddr::V6(host) => host.to_bits() as u32,
            };
            prop_assert_eq!(judged, addr, "{}", address);
        }
    }

    #[test]
    fn no_ipv6_address_gets_past_the_rules(
        addr in any_v6(),
        decoration in any::<usize>(),
        port in proptest::option::of(any::<u16>()),
    ) {
        let host = format!("[{}]", Ipv6Addr::from(addr));
        let address = address(&host, decoration, port);
        let verdict = CLIENT.parse(&address);
        prop_assert_eq!(verdict.is_ok(), public_v6(addr), "{}: {:?}", address, verdict);
        prop_assert_eq!(internal(Ipv6Addr::from(addr).into()).is_none(), public_v6(addr));
    }

    #[test]
    fn the_rules_agree_with_the_tables_for_every_ipv4_address(addr in any_v4()) {
        prop_assert_eq!(internal(Ipv4Addr::from(addr).into()).is_none(), public_v4(addr));
    }

    #[test]
    fn only_http_and_https_are_fetched(scheme in "[a-z][a-z0-9+.-]{0,8}", host in "[a-z]{1,10}\\.example") {
        let verdict = CLIENT.parse(&format!("{scheme}://{host}/"));
        prop_assert_eq!(verdict.is_ok(), scheme == "http" || scheme == "https", "{}", scheme);
    }

    #[test]
    fn nothing_somebody_types_brings_the_check_down(address in "\\PC{0,60}") {
        if let Ok(url) = CLIENT.parse(&address) {
            prop_assert!(matches!(url.scheme(), "http" | "https"));
            prop_assert!(url.host().is_some());
        }
    }
}

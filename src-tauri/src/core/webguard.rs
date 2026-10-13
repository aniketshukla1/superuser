//! Web guard — SSRF protection + untrusted-content framing for the agent's web
//! tools (`web_fetch`/`web_search`). The concept: an agent must not be able to reach
//! the machine's own private network, and web content is data, never instructions.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(test)]
mod tests;

mod parse_ipv4;
mod normalize_for_matching;

pub use parse_ipv4::*;
pub use normalize_for_matching::*;

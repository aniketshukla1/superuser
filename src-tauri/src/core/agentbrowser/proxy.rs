//! The assistant browser's only way out — a proxy this app owns.
//!
//! The catch: a real Chrome does its
//! own networking. Everything else in this app reaches the network through
//! `core::egress`, which is what makes local-only mode a hard switch and the Privacy page
//! a complete record. A browser launched without this would step around both on every
//! page it loaded — and every image, script and redirect on that page.
//!
//! So each assistant's Chrome is started with `--proxy-server` pointed here and
//! `--proxy-bypass-list=<-loopback>` (which removes Chrome's implicit "never proxy
//! localhost"), and this decides every connection:
//!
//! 1. **The name.** Loopback only when the user allowed it. Private and cloud-metadata
//!    names never (`webguard`'s table). Anything else goes to `egress::allow`, which
//!    refuses it under local-only and puts it on the Network page otherwise.
//! 2. **The address it resolves to.** Stronger than a floor that only reads the name and
//!    never resolves: this proxy is the thing that connects, so it
//!    can check what a name really points at, and then connect to exactly that address —
//!    a public name that resolves to `127.0.0.1` or `169.254.169.254` is refused, and a
//!    second lookup cannot swap the answer between the check and the connect.
//!
//! HTTPS is tunnelled, never inspected: the proxy sees a host and a port, which is what a
//! record of "who heard from this browser" needs and all it should hold.

use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[cfg(test)]
mod tests;

mod decide_name;
mod serve;

pub use decide_name::*;
pub use serve::*;

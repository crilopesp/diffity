//! `diffity-server [--port 4318] [--host 127.0.0.1] [--web <dir>]`: the app for a browser, run on this machine.
//!
//! Build it on its own (`cargo build --no-default-features --features server --bin diffity-server`): with the
//! `app` feature the crate is the Tauri app and has no server.

#[cfg(feature = "app")]
fn main() {
    eprintln!("diffity-server is built without the app: cargo build --no-default-features --features server --bin diffity-server");
    std::process::exit(1);
}

#[cfg(not(feature = "app"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .try_init();
    // Agents and git need the login shell's PATH, also when started by a service manager.
    diffity_desktop_lib::env_fix::load_login_shell_env();
    diffity_desktop_lib::server::serve(options::parse(std::env::args().skip(1))?).await
}

#[cfg(not(feature = "app"))]
mod options {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;

    use diffity_desktop_lib::server::ServerOptions;

    const USAGE: &str = "usage: diffity-server [--port 4318] [--host 127.0.0.1] [--web <dir>]";

    pub fn parse(mut args: impl Iterator<Item = String>) -> anyhow::Result<ServerOptions> {
        let mut host = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let mut port = 4318;
        let mut web_dir = PathBuf::from("dist-web");
        while let Some(flag) = args.next() {
            let mut value = || args.next().ok_or_else(|| anyhow::anyhow!("{flag} needs a value\n{USAGE}"));
            match flag.as_str() {
                "--host" => host = value()?.parse()?,
                "--port" => port = value()?.parse()?,
                "--web" => web_dir = PathBuf::from(value()?),
                _ => anyhow::bail!("unknown option {flag}\n{USAGE}"),
            }
        }
        Ok(ServerOptions { address: SocketAddr::new(host, port), web_dir })
    }
}

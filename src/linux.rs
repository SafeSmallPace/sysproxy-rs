use crate::{Error, Result, Sysproxy};
use std::{env, process::Command, str::from_utf8};
use xdg;

const CMD_KEY: &str = "org.gnome.system.proxy";

impl Sysproxy {
    pub fn get_system_proxy() -> Result<Sysproxy> {
        let enable = Sysproxy::get_enable()?;

        let mut socks = get_proxy("socks")?;
        let https = get_proxy("https")?;
        let http = get_proxy("http")?;

        if socks.host.len() == 0 {
            if http.host.len() > 0 {
                socks.host = http.host;
                socks.port = http.port;
            }
            if https.host.len() > 0 {
                socks.host = https.host;
                socks.port = https.port;
            }
        }

        socks.enable = enable;
        socks.bypass = Sysproxy::get_bypass().unwrap_or("".into());

        Ok(socks)
    }

    pub fn set_system_proxy(&self) -> Result<()> {
        self.set_enable()?;

        if self.enable {
            self.set_socks()?;
            self.set_https()?;
            self.set_http()?;
            self.set_bypass()?;
        }

        Ok(())
    }

    pub fn get_enable() -> Result<bool> {
        match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
            "GNOME" => {
                let mode = gsettings().args(["get", CMD_KEY, "mode"]).output()?;
                let mode = from_utf8(&mode.stdout).or(Err(Error::ParseStr("mode".into())))?.trim();
                Ok(mode == "'manual'")
            }
            "KDE" => {
                let xdg_dir = xdg::BaseDirectories::new()?;
                let config = xdg_dir.get_config_file("kioslaverc");
                let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

                let mode = kreadconfig()
                    .args([
                        "--file",
                        config,
                        "--group",
                        "Proxy Settings",
                        "--key",
                        "ProxyType",
                    ])
                    .output()?;
                let mode = from_utf8(&mode.stdout).or(Err(Error::ParseStr("mode".into())))?.trim();
                Ok(mode == "1")
            }
            _ => Err(Error::NotSupport),
        }
    }

    pub fn get_bypass() -> Result<String> {
        match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
            "GNOME" => {
                let bypass = gsettings()
                    .args(["get", CMD_KEY, "ignore-hosts"])
                    .output()?;
                let bypass = from_utf8(&bypass.stdout)
                    .or(Err(Error::ParseStr("bypass".into())))?
                    .trim();

                let bypass = bypass.strip_prefix('[').unwrap_or(bypass);
                let bypass = bypass.strip_suffix(']').unwrap_or(bypass);

                let bypass = bypass
                    .split(',')
                    .map(|h| strip_str(h.trim()))
                    .collect::<Vec<&str>>()
                    .join(",");

                Ok(bypass)
            }
            "KDE" => {
                let xdg_dir = xdg::BaseDirectories::new()?;
                let config = xdg_dir.get_config_file("kioslaverc");
                let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

                let bypass = kreadconfig()
                    .args([
                        "--file",
                        config,
                        "--group",
                        "Proxy Settings",
                        "--key",
                        "NoProxyFor",
                    ])
                    .output()?;
                let bypass = from_utf8(&bypass.stdout).or(Err(Error::ParseStr("bypass".into())))?.trim();

                let bypass = bypass
                    .split(',')
                    .map(|h| strip_str(h.trim()))
                    .collect::<Vec<&str>>()
                    .join(",");

                Ok(bypass)
            }
            _ => Err(Error::NotSupport),
        }
    }

    pub fn get_http() -> Result<Sysproxy> {
        get_proxy("http")
    }

    pub fn get_https() -> Result<Sysproxy> {
        get_proxy("https")
    }

    pub fn get_socks() -> Result<Sysproxy> {
        get_proxy("socks")
    }

    pub fn set_enable(&self) -> Result<()> {
        match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
            "GNOME" => {
                let mode = if self.enable { "'manual'" } else { "'none'" };
                gsettings().args(["set", CMD_KEY, "mode", mode]).status()?;
                Ok(())
            }
            "KDE" => {
                let xdg_dir = xdg::BaseDirectories::new()?;
                let config = xdg_dir.get_config_file("kioslaverc");
                let config = config.to_str().ok_or(Error::ParseStr)?;
                let mode = if self.enable { "1" } else { "0" };
                kwriteconfig()
                    .args([
                        "--file",
                        config,
                        "--group",
                        "Proxy Settings",
                        "--key",
                        "ProxyType",
                        mode,
                    ])
                    .status()?;
                Ok(())
            }
            _ => Err(Error::NotSupport),
        }
    }

    pub fn set_bypass(&self) -> Result<()> {
        match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
            "GNOME" => {
                let bypass = self
                    .bypass
                    .split(',')
                    .map(|h| {
                        let mut host = String::from(h.trim());
                        if !host.starts_with('\'') && !host.starts_with('"') {
                            host = String::from("'") + &host;
                        }
                        if !host.ends_with('\'') && !host.ends_with('"') {
                            host = host + "'";
                        }
                        host
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                let bypass = format!("[{bypass}]");

                gsettings()
                    .args(["set", CMD_KEY, "ignore-hosts", bypass.as_str()])
                    .status()?;
                Ok(())
            }
            "KDE" => {
                let xdg_dir = xdg::BaseDirectories::new()?;
                let config = xdg_dir.get_config_file("kioslaverc");
                let config = config.to_str().ok_or(Error::ParseStr)?;

                kwriteconfig()
                    .args([
                        "--file",
                        config,
                        "--group",
                        "Proxy Settings",
                        "--key",
                        "NoProxyFor",
                        self.bypass.as_str(),
                    ])
                    .status()?;
                Ok(())
            }
            _ => Err(Error::NotSupport),
        }
    }

    pub fn set_http(&self) -> Result<()> {
        set_proxy(self, "http")
    }

    pub fn set_https(&self) -> Result<()> {
        set_proxy(self, "https")
    }

    pub fn set_socks(&self) -> Result<()> {
        set_proxy(self, "socks")
    }
}

fn gsettings() -> Command {
    Command::new("gsettings")
}

fn kreadconfig() -> Command {
    Command::new("kreadconfig5")
}

fn kwriteconfig() -> Command {
    Command::new("kwriteconfig5")
}

fn set_proxy(proxy: &Sysproxy, service: &str) -> Result<()> {
    match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
        "GNOME" => {
            let schema = format!("{CMD_KEY}.{service}");
            let schema = schema.as_str();

            let host = format!("'{}'", proxy.host);
            let host = host.as_str();
            let port = format!("{}", proxy.port);
            let port = port.as_str();

            gsettings().args(["set", schema, "host", host]).status()?;
            gsettings().args(["set", schema, "port", port]).status()?;

            Ok(())
        }
        "KDE" => {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr)?;

            let key = format!("{service}Proxy");
            let key = key.as_str();

            let service = match service {
                "socks" => "socks",
                _ => "http",
            };

            let host = format!("{}", proxy.host);
            let host = host.as_str();
            let port = format!("{}", proxy.port);
            let port = port.as_str();

            let schema = format!("{service}://{host} {port}");
            let schema = schema.as_str();

            kwriteconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    key,
                    schema,
                ])
                .status()?;

            Ok(())
        }
        _ => Err(Error::NotSupport),
    }
}

fn get_proxy(service: &str) -> Result<Sysproxy> {
    match env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().as_str() {
        "GNOME" => {
            let schema = format!("{CMD_KEY}.{service}");
            let schema = schema.as_str();

            let host = gsettings().args(["get", schema, "host"]).output()?;
            let host = from_utf8(&host.stdout)
                .or(Err(Error::ParseStr("host".into())))?
                .trim();
            let host = strip_str(host);

            let port = gsettings().args(["get", schema, "port"]).output()?;
            let port = from_utf8(&port.stdout)
                .or(Err(Error::ParseStr("port".into())))?
                .trim();
            let port = port.parse().unwrap_or(80u16);

            Ok(Sysproxy {
                enable: false,
                host: String::from(host),
                port,
                bypass: "".into(),
            })
        }
        "KDE" => {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

            let key = format!("{service}Proxy");
            let key = key.as_str();

            let schema = kreadconfig()
                .args(["--file", config, "--group", "Proxy Settings", "--key", key])
                .output()?;
            let schema = from_utf8(&schema.stdout).or(Err(Error::ParseStr("schema".into())))?.trim();
            let schema = schema
                .trim_start_matches("http://")
                .trim_start_matches("socks://");
            let schema = schema.split_once(' ').ok_or(Error::ParseStr("schema".into()))?;

            let host = strip_str(schema.0);
            let port = schema.1.parse().unwrap_or(80u16);

            Ok(Sysproxy {
                enable: false,
                host: String::from(host),
                port,
                bypass: "".into(),
            })
        }
        _ => Err(Error::NotSupport),
    }
}

fn strip_str<'a>(text: &'a str) -> &'a str {
    text.strip_prefix('\'')
        .unwrap_or(text)
        .strip_suffix('\'')
        .unwrap_or(text)
}

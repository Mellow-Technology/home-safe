//! Start at Login, via a per-user LaunchAgent. Unlike `SMAppService` this
//! works for unbundled `cargo run` builds too. The agent passes
//! [`BACKGROUND_FLAG`], so a login launch starts in the menu bar only.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

/// Starts the app without opening its window.
pub const BACKGROUND_FLAG: &str = "--background";

const LABEL: &str = "com.mellow.HomeSafe";

fn plist_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(Path::new(&home)
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

/// Whether Home Safe starts at login.
pub fn is_enabled() -> bool {
    plist_path().is_ok_and(|p| p.exists())
}

/// Turns Start at Login on (for this copy of the app) or off. Takes effect
/// at the next login; nothing is started now.
pub fn set_enabled(enabled: bool) -> Result<()> {
    let path = plist_path()?;
    if enabled {
        let exe = std::env::current_exe().context("finding the app's executable")?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, plist(&exe)).with_context(|| format!("writing {}", path.display()))
    } else {
        match std::fs::remove_file(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                Err(err).with_context(|| format!("removing {}", path.display()))
            }
            _ => Ok(()),
        }
    }
}

fn plist(exe: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>{BACKGROUND_FLAG}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(&exe.to_string_lossy())
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_runs_this_exe_in_the_background() {
        let plist = plist(Path::new("/Apps/Home & Safe.app/Contents/MacOS/home-safe"));
        assert!(
            plist.contains("<string>/Apps/Home &amp; Safe.app/Contents/MacOS/home-safe</string>")
        );
        assert!(plist.contains("<string>--background</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"));
    }
}

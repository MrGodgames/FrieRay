use std::process::Command;

/// Set system proxy on macOS
#[cfg(target_os = "macos")]
pub fn set_system_proxy(http_port: u16, socks_port: u16) -> Result<(), String> {
    let interfaces = get_macos_network_interfaces()?;

    for interface in &interfaces {
        // Set HTTP proxy
        run_cmd(
            "networksetup",
            &[
                "-setwebproxy",
                interface,
                "127.0.0.1",
                &http_port.to_string(),
            ],
        )?;

        // Set HTTPS proxy
        run_cmd(
            "networksetup",
            &[
                "-setsecurewebproxy",
                interface,
                "127.0.0.1",
                &http_port.to_string(),
            ],
        )?;

        // Set SOCKS proxy
        run_cmd(
            "networksetup",
            &[
                "-setsocksfirewallproxy",
                interface,
                "127.0.0.1",
                &socks_port.to_string(),
            ],
        )?;

        log::info!("System proxy set on interface: {}", interface);
    }

    Ok(())
}

/// Unset system proxy on macOS
#[cfg(target_os = "macos")]
pub fn unset_system_proxy() -> Result<(), String> {
    let interfaces = get_macos_network_interfaces()?;

    for interface in &interfaces {
        run_cmd("networksetup", &["-setwebproxystate", interface, "off"])?;
        run_cmd(
            "networksetup",
            &["-setsecurewebproxystate", interface, "off"],
        )?;
        run_cmd(
            "networksetup",
            &["-setsocksfirewallproxystate", interface, "off"],
        )?;
        log::info!("System proxy cleared on interface: {}", interface);
    }

    Ok(())
}

/// Get active network interfaces on macOS
#[cfg(target_os = "macos")]
fn get_macos_network_interfaces() -> Result<Vec<String>, String> {
    let output = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .map_err(|e| format!("Failed to list network services: {}", e))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let interfaces: Vec<String> = text
        .lines()
        .skip(1) // Skip "An asterisk (*)" header
        .filter(|line| !line.starts_with('*') && !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    if interfaces.is_empty() {
        return Err("No network interfaces found".into());
    }

    Ok(interfaces)
}

/// Set system proxy on Windows
#[cfg(target_os = "windows")]
pub fn set_system_proxy(http_port: u16, _socks_port: u16) -> Result<(), String> {
    let proxy_server = format!("127.0.0.1:{}", http_port);

    // Enable proxy via registry
    run_cmd(
        "reg",
        &[
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyEnable",
            "/t",
            "REG_DWORD",
            "/d",
            "1",
            "/f",
        ],
    )?;

    run_cmd(
        "reg",
        &[
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyServer",
            "/t",
            "REG_SZ",
            "/d",
            &proxy_server,
            "/f",
        ],
    )?;

    log::info!("Windows system proxy set to {}", proxy_server);
    Ok(())
}

/// Unset system proxy on Windows
#[cfg(target_os = "windows")]
pub fn unset_system_proxy() -> Result<(), String> {
    run_cmd(
        "reg",
        &[
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyEnable",
            "/t",
            "REG_DWORD",
            "/d",
            "0",
            "/f",
        ],
    )?;

    log::info!("Windows system proxy disabled");
    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Command '{}' failed: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command '{}' returned error: {}", program, stderr));
    }

    Ok(())
}

///tun.rs - Debug Enhanced Version
///
/// create a tun interface and configure it properly for testing
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::Command;
use tokio::io::unix::AsyncFd;
use tun::{Configuration, Device as DeviceTrait};

pub struct TunInterface {
    pub name: String,
    inner: AsyncFd<File>,
}

impl TunInterface {
    pub fn create_server() -> Result<Self> {
        Self::cleanup_existing_interface("rustvpn0")?;

        let mut config = Configuration::default();
        config.name("rustvpn0").mtu(1500).layer(tun::Layer::L3);

        let dev = tun::create(&config).context("Failed to create server TUN interface")?;

        let name = dev.name()?.to_string();
        println!("Created server TUN interface: {}", name);

        let raw_fd = dev.into_raw_fd();
        let file = unsafe { File::from_raw_fd(raw_fd) };
        let async_fd = AsyncFd::new(file)?;

        Ok(Self {
            name,
            inner: async_fd,
        })
    }

    pub fn create_client() -> Result<Self> {
        Self::cleanup_existing_interface("rustvpn1")?;

        let mut config = Configuration::default();
        config.name("rustvpn1").mtu(1500).layer(tun::Layer::L3);

        let dev = tun::create(&config).context("Failed to create client TUN interface")?;

        let name = dev.name()?.to_string();
        println!("Created client TUN interface: {}", name);

        let raw_fd = dev.into_raw_fd();
        let file = unsafe { File::from_raw_fd(raw_fd) };
        let async_fd = AsyncFd::new(file)?;

        Ok(Self {
            name,
            inner: async_fd,
        })
    }

    fn cleanup_existing_interface(interface_name: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(&["link", "show", interface_name])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("Cleaning up existing interface: {}", interface_name);
                let _ = Command::new("ip")
                    .args(&["link", "set", interface_name, "down"])
                    .output();
                let _ = Command::new("ip")
                    .args(&["link", "delete", interface_name])
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        Ok(())
    }

    pub fn configure_server_ip(&self) -> Result<()> {
        println!("Configuring server IP for {}", self.name);

        // Configure IP
        let add_result = Command::new("ip")
            .args(&["addr", "add", "10.8.0.1/30", "dev", &self.name])
            .output()?;

        if !add_result.status.success() {
            let error = String::from_utf8_lossy(&add_result.stderr);
            return Err(anyhow::anyhow!("Failed to add server IP: {}", error));
        }

        // Bring up
        let up_result = Command::new("ip")
            .args(&["link", "set", &self.name, "up"])
            .output()?;

        if !up_result.status.success() {
            let error = String::from_utf8_lossy(&up_result.stderr);
            return Err(anyhow::anyhow!(
                "Failed to bring server interface up: {}",
                error
            ));
        }

        // Add route to reach client
        let _ = Command::new("ip")
            .args(&["route", "add", "10.8.0.2/32", "dev", &self.name])
            .output();

        println!("✅ Server TUN configured: 10.8.0.1/30");

        // Show configuration
        self.show_interface_config();
        Ok(())
    }

    pub fn configure_client_ip(&self) -> Result<()> {
        println!("Configuring client IP for {}", self.name);

        // Configure IP
        let add_result = Command::new("ip")
            .args(&["addr", "add", "10.8.0.2/30", "dev", &self.name])
            .output()?;

        if !add_result.status.success() {
            let error = String::from_utf8_lossy(&add_result.stderr);
            return Err(anyhow::anyhow!("Failed to add client IP: {}", error));
        }

        // Bring up
        let up_result = Command::new("ip")
            .args(&["link", "set", &self.name, "up"])
            .output()?;

        if !up_result.status.success() {
            let error = String::from_utf8_lossy(&up_result.stderr);
            return Err(anyhow::anyhow!(
                "Failed to bring client interface up: {}",
                error
            ));
        }

        // Add route to reach server
        let _ = Command::new("ip")
            .args(&["route", "add", "10.8.0.1/32", "dev", &self.name])
            .output();

        println!("✅ Client TUN configured: 10.8.0.2/30");

        // Show configuration
        self.show_interface_config();
        Ok(())
    }

    fn show_interface_config(&self) {
        if let Ok(output) = Command::new("ip")
            .args(&["addr", "show", &self.name])
            .output()
        {
            if output.status.success() {
                let config = String::from_utf8_lossy(&output.stdout);
                println!("Interface {} configuration:", self.name);
                for line in config.lines() {
                    if line.trim().starts_with("inet") {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
    }

    pub async fn read_packet(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 1504];
        let guard = self.inner.readable().await?;
        let n = guard.get_inner().read(&mut buf)?;
        buf.truncate(n);

        // Debug: Show packet info
        if n > 0 {
            println!(
                "📦 TUN read {} bytes: {:02x?}...",
                n,
                &buf[..std::cmp::min(20, n)]
            );
        }

        Ok(buf)
    }

    pub async fn write_packet(&self, data: &[u8]) -> Result<()> {
        let guard = self.inner.writable().await?;
        guard.get_inner().write_all(data)?;

        // Debug: Show packet info
        println!(
            "📤 TUN write {} bytes: {:02x?}...",
            data.len(),
            &data[..std::cmp::min(20, data.len())]
        );

        Ok(())
    }
}

impl Drop for TunInterface {
    fn drop(&mut self) {
        let _ = Command::new("ip")
            .args(&["link", "set", &self.name, "down"])
            .output();
        let _ = Command::new("ip")
            .args(&["link", "delete", &self.name])
            .output();
        println!("🧹 Cleaned up TUN interface: {}", self.name);
    }
}

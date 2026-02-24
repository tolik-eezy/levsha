//! Risk classification for shell commands.
//!
//! Detects destructive commands and classifies them by risk level.
//! Used by the engine to trigger user confirmation before executing
//! potentially dangerous operations.

use regex::Regex;
use tracing::warn;

/// Risk level for a command that matched a destructive pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Commands that can destroy the entire system or data irrecoverably.
    /// Examples: `rm -rf /`, `mkfs`, `dd if=`, disk partitioning, reboot/shutdown.
    High,
    /// Commands that can disrupt services or remove packages.
    /// Examples: `dnf remove`, `systemctl stop` on core services.
    Medium,
    /// Commands that match a destructive pattern but are lower severity.
    /// Examples: dangerous redirects to block devices.
    Low,
}

/// A single destructive pattern with its associated risk level.
struct PatternEntry {
    regex: Regex,
    level: RiskLevel,
}

/// Destructive command guard -- checks if a command matches known dangerous patterns
/// and classifies the risk level.
pub struct DestructiveGuard {
    patterns: Vec<PatternEntry>,
}

impl DestructiveGuard {
    pub fn new() -> Self {
        let entries: &[(&str, RiskLevel)] = &[
            // High: Recursive / forced file deletion
            (r"rm\s+(-[a-zA-Z]*r|-[a-zA-Z]*f[a-zA-Z]*\s+/|--recursive)", RiskLevel::High),
            // High: Filesystem formatting
            (r"mkfs\.", RiskLevel::High),
            (r"wipefs\s", RiskLevel::High),
            // High: Raw disk writes
            (r"dd\s+.*if=", RiskLevel::High),
            // High: Partition table modification
            (r"fdisk\s", RiskLevel::High),
            (r"parted\s", RiskLevel::High),
            (r"gdisk\s", RiskLevel::High),
            // High: System shutdown / reboot
            (r"(reboot|shutdown|poweroff|halt)\b", RiskLevel::High),
            (r"init\s+[06]", RiskLevel::High),
            // Medium: Stopping/disabling core services
            (r"systemctl\s+(stop|disable|mask)\s+(sshd|systemd-|levsha-|NetworkManager)", RiskLevel::Medium),
            // Medium: Bulk package removal
            (r"dnf\s+(-y\s+)?remove\s", RiskLevel::Medium),
            // Medium: Network configuration changes
            (r"nmcli\s+connection\s+modify\s", RiskLevel::Medium),
            // Low: Dangerous redirects to block devices
            (r">\s*/dev/sd[a-z]", RiskLevel::Low),
            (r">\s*/dev/nvme", RiskLevel::Low),
        ];

        let patterns = entries
            .iter()
            .filter_map(|(pattern, level)| match Regex::new(pattern) {
                Ok(re) => Some(PatternEntry {
                    regex: re,
                    level: *level,
                }),
                Err(e) => {
                    warn!("Failed to compile destructive pattern '{}': {}", pattern, e);
                    None
                }
            })
            .collect();

        Self { patterns }
    }

    /// Classify the risk level of a command. Returns `None` if the command
    /// does not match any destructive pattern.
    pub fn classify_risk(&self, command: &str) -> Option<RiskLevel> {
        let mut highest: Option<RiskLevel> = None;

        for entry in &self.patterns {
            if entry.regex.is_match(command) {
                highest = Some(match highest {
                    None => entry.level,
                    Some(current) => higher_risk(current, entry.level),
                });
            }
        }

        highest
    }

    /// Check if a command matches any destructive pattern (backwards compatibility).
    pub fn is_destructive(&self, command: &str) -> bool {
        self.classify_risk(command).is_some()
    }
}

impl Default for DestructiveGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Return the higher of two risk levels.
fn higher_risk(a: RiskLevel, b: RiskLevel) -> RiskLevel {
    match (a, b) {
        (RiskLevel::High, _) | (_, RiskLevel::High) => RiskLevel::High,
        (RiskLevel::Medium, _) | (_, RiskLevel::Medium) => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> DestructiveGuard {
        DestructiveGuard::new()
    }

    // -----------------------------------------------------------------------
    // High risk commands
    // -----------------------------------------------------------------------

    #[test]
    fn rm_rf_root_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("rm -rf /"), Some(RiskLevel::High));
    }

    #[test]
    fn rm_rf_home_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("rm -rf /home/user"), Some(RiskLevel::High));
    }

    #[test]
    fn rm_single_file_is_not_destructive() {
        let g = guard();
        assert_eq!(g.classify_risk("rm file.txt"), None);
        assert!(!g.is_destructive("rm file.txt"));
    }

    #[test]
    fn mkfs_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("mkfs.ext4 /dev/sda1"), Some(RiskLevel::High));
    }

    #[test]
    fn dd_is_high() {
        let g = guard();
        assert_eq!(
            g.classify_risk("dd if=/dev/zero of=/dev/sda"),
            Some(RiskLevel::High)
        );
    }

    #[test]
    fn fdisk_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("fdisk /dev/sda"), Some(RiskLevel::High));
    }

    #[test]
    fn parted_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("parted /dev/sda"), Some(RiskLevel::High));
    }

    #[test]
    fn gdisk_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("gdisk /dev/sda"), Some(RiskLevel::High));
    }

    #[test]
    fn wipefs_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("wipefs /dev/sda"), Some(RiskLevel::High));
    }

    #[test]
    fn reboot_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("reboot"), Some(RiskLevel::High));
    }

    #[test]
    fn shutdown_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("shutdown -h now"), Some(RiskLevel::High));
    }

    #[test]
    fn poweroff_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("poweroff"), Some(RiskLevel::High));
    }

    #[test]
    fn halt_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("halt"), Some(RiskLevel::High));
    }

    #[test]
    fn init_0_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("init 0"), Some(RiskLevel::High));
    }

    #[test]
    fn init_6_is_high() {
        let g = guard();
        assert_eq!(g.classify_risk("init 6"), Some(RiskLevel::High));
    }

    // -----------------------------------------------------------------------
    // Medium risk commands
    // -----------------------------------------------------------------------

    #[test]
    fn systemctl_stop_sshd_is_medium() {
        let g = guard();
        assert_eq!(
            g.classify_risk("systemctl stop sshd"),
            Some(RiskLevel::Medium)
        );
    }

    #[test]
    fn systemctl_disable_networkmanager_is_medium() {
        let g = guard();
        assert_eq!(
            g.classify_risk("systemctl disable NetworkManager"),
            Some(RiskLevel::Medium)
        );
    }

    #[test]
    fn dnf_remove_is_medium() {
        let g = guard();
        assert_eq!(
            g.classify_risk("dnf remove curl"),
            Some(RiskLevel::Medium)
        );
    }

    #[test]
    fn dnf_y_remove_is_medium() {
        let g = guard();
        assert_eq!(
            g.classify_risk("dnf -y remove curl"),
            Some(RiskLevel::Medium)
        );
    }

    #[test]
    fn nmcli_connection_modify_is_medium() {
        let g = guard();
        assert_eq!(
            g.classify_risk("nmcli connection modify eth0 ipv4.method manual"),
            Some(RiskLevel::Medium)
        );
    }

    // -----------------------------------------------------------------------
    // Low risk commands
    // -----------------------------------------------------------------------

    #[test]
    fn redirect_to_block_device_is_low() {
        let g = guard();
        assert_eq!(
            g.classify_risk("echo hello > /dev/sda"),
            Some(RiskLevel::Low)
        );
    }

    #[test]
    fn redirect_to_nvme_is_low() {
        let g = guard();
        assert_eq!(
            g.classify_risk("echo data > /dev/nvme0n1"),
            Some(RiskLevel::Low)
        );
    }

    // -----------------------------------------------------------------------
    // Safe commands (not destructive)
    // -----------------------------------------------------------------------

    #[test]
    fn ls_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("ls -la"));
        assert_eq!(g.classify_risk("ls -la"), None);
    }

    #[test]
    fn cat_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("cat /etc/passwd"));
    }

    #[test]
    fn dnf_search_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("dnf search curl"));
        assert_eq!(g.classify_risk("dnf search curl"), None);
    }

    #[test]
    fn echo_to_file_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("echo hello > /tmp/test.txt"));
    }

    #[test]
    fn systemctl_start_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("systemctl start nginx"));
    }

    #[test]
    fn dnf_install_is_safe() {
        let g = guard();
        assert!(!g.is_destructive("dnf install vim"));
    }

    // -----------------------------------------------------------------------
    // is_destructive backwards compatibility
    // -----------------------------------------------------------------------

    #[test]
    fn is_destructive_returns_true_for_dangerous() {
        let g = guard();
        assert!(g.is_destructive("rm -rf /"));
        assert!(g.is_destructive("mkfs.ext4 /dev/sda1"));
        assert!(g.is_destructive("dd if=/dev/zero of=/dev/sda"));
        assert!(g.is_destructive("reboot"));
        assert!(g.is_destructive("dnf remove curl"));
    }

    #[test]
    fn is_destructive_returns_false_for_safe() {
        let g = guard();
        assert!(!g.is_destructive("ls -la"));
        assert!(!g.is_destructive("cat /etc/passwd"));
        assert!(!g.is_destructive("echo hello"));
        assert!(!g.is_destructive("pwd"));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_command_is_safe() {
        let g = guard();
        assert!(!g.is_destructive(""));
        assert_eq!(g.classify_risk(""), None);
    }

    #[test]
    fn higher_risk_fn_returns_highest() {
        assert_eq!(higher_risk(RiskLevel::Low, RiskLevel::High), RiskLevel::High);
        assert_eq!(higher_risk(RiskLevel::High, RiskLevel::Low), RiskLevel::High);
        assert_eq!(higher_risk(RiskLevel::Low, RiskLevel::Medium), RiskLevel::Medium);
        assert_eq!(higher_risk(RiskLevel::Medium, RiskLevel::Low), RiskLevel::Medium);
        assert_eq!(higher_risk(RiskLevel::Low, RiskLevel::Low), RiskLevel::Low);
    }
}

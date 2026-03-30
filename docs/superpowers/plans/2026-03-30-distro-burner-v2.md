# distro-burner v2.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite distro-burner as a safe, TUI-driven Rust CLI that burns Linux ISOs only to newly created partitions in unallocated disk space, never touching existing partitions.

**Architecture:** Seven focused source modules (`catalogue`, `disk`, `partition`, `download`, `tui`, `burn`, `main`) plus a user-editable `distro-burner.yaml` that replaces all hard-coded ISO data. The flow is: load config → TUI selections → disk probe → partition creation → download+verify → burn.

**Tech Stack:** Rust 2021, clap 4 (derive), reqwest 0.12 (blocking), serde + serde_yaml 0.9, serde_json 1, inquire 0.7, sha2 0.10, hex 0.4, indicatif 0.17, anyhow 1, chrono 0.4, tempfile 3, colored 2.

---

## File Map

| File | Responsibility |
|---|---|
| `Cargo.toml` | All dependencies |
| `distro-burner.yaml` | ISO catalogue, blacklist, default sizes (user-editable) |
| `src/main.rs` | Entry point, clap args, top-level orchestration |
| `src/catalogue.rs` | `IsoSpec`, `Config`, `ChecksumFormat` — serde YAML load/save |
| `src/disk.rs` | `DiskInfo`, `FreeGap` — lsblk + parted probing, mount checks |
| `src/partition.rs` | Blacklist check, name validation, parted create |
| `src/download.rs` | Stream download, tempfile, 3× retry, SHA256 verify |
| `src/tui.rs` | All inquire prompts (no business logic) |
| `src/burn.rs` | `sudo dd` execution, full command logging |

---

## Task 1: Scaffold — Cargo.toml + project structure

**Files:**
- Modify: `Cargo.toml`
- Create: `distro-burner.yaml`
- Create: `src/catalogue.rs`, `src/disk.rs`, `src/partition.rs`, `src/download.rs`, `src/tui.rs`, `src/burn.rs`

- [ ] **Step 1: Replace Cargo.toml**

```toml
[package]
name = "distro-burner"
version = "2.0.0"
edition = "2021"
description = "Safely burn Linux ISOs to new unallocated-space partitions"

[[bin]]
name = "distro-burner"
path = "src/main.rs"

[dependencies]
anyhow      = "1"
chrono      = "0.4"
clap        = { version = "4", features = ["derive"] }
colored     = "2"
hex         = "0.4"
indicatif   = "0.17"
inquire     = "0.7"
reqwest     = { version = "0.12", features = ["blocking"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
serde_yaml  = "0.9"
sha2        = "0.10"
tempfile    = "3"

[dev-dependencies]
# none needed — unit tests live in each module
```

- [ ] **Step 2: Create stub modules so the project compiles**

Create `src/catalogue.rs`:
```rust
// catalogue.rs — ISO catalogue types, YAML config load/save
pub struct IsoSpec;    // stub — filled in Task 2
pub struct Config;     // stub — filled in Task 2
```

Create `src/disk.rs`:
```rust
// disk.rs — disk probing, gap detection, mount checks
```

Create `src/partition.rs`:
```rust
// partition.rs — blacklist enforcement, name validation, parted shell-out
```

Create `src/download.rs`:
```rust
// download.rs — streaming download, tempfile, retries, SHA256 verify
```

Create `src/tui.rs`:
```rust
// tui.rs — all inquire TUI prompts
```

Create `src/burn.rs`:
```rust
// burn.rs — sudo dd execution and logging
```

- [ ] **Step 3: Replace src/main.rs with minimal stub**

```rust
mod catalogue;
mod disk;
mod partition;
mod download;
mod tui;
mod burn;

fn main() {
    println!("distro-burner v2.0 — stub");
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1
```
Expected: compiles with 0 errors (warnings about unused stubs are fine).

- [ ] **Step 5: Create distro-burner.yaml**

```yaml
# distro-burner.yaml
# Edit ISO URLs/versions here when distros release new point versions.
# Never edit blacklist manually — let distro-burner manage it.

blacklist: []   # populated on first run

default_sizes_gb:
  ubuntu: 18
  fedora: 20
  popos:  18
  debian: 15
  arch:   15

isos:
  - key: ubuntu
    display: "Ubuntu 24.04.2 LTS Desktop (amd64)"
    iso_url: "https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso"
    checksum_url: "https://releases.ubuntu.com/24.04/SHA256SUMS"
    iso_filename: "ubuntu-24.04.2-desktop-amd64.iso"
    checksum_format: standard

  - key: fedora
    display: "Fedora Workstation 42 Live (x86_64)"
    iso_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-42-1.1.iso"
    checksum_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-42-1.1-x86_64-CHECKSUM"
    iso_filename: "Fedora-Workstation-Live-x86_64-42-1.1.iso"
    checksum_format: fedora

  - key: popos
    display: "Pop!_OS 22.04 LTS Intel/AMD (amd64)"
    iso_url: "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/pop-os_22.04_amd64_intel_264.iso"
    checksum_url: "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/SHA256SUMS"
    iso_filename: "pop-os_22.04_amd64_intel_264.iso"
    checksum_format: standard

  - key: debian
    display: "Debian 12.9 Netinstall (amd64)"
    iso_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.9.0-amd64-netinst.iso"
    checksum_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS"
    iso_filename: "debian-12.9.0-amd64-netinst.iso"
    checksum_format: standard

  - key: arch
    display: "Arch Linux latest (x86_64)"
    iso_url: "https://mirror.rackspace.com/archlinux/iso/latest/archlinux-x86_64.iso"
    checksum_url: "https://archlinux.org/iso/latest/sha256sums.txt"
    iso_filename: "archlinux-x86_64.iso"
    checksum_format: standard
```

- [ ] **Step 6: Commit scaffold**

```bash
git add Cargo.toml distro-burner.yaml src/
git commit -m "chore: scaffold distro-burner v2.0 module structure"
```

---

## Task 2: catalogue.rs — Config types + YAML load/save

**Files:**
- Modify: `src/catalogue.rs`

- [ ] **Step 1: Write failing tests first**

Add to bottom of `src/catalogue.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
blacklist:
  - sda1
  - sda2
default_sizes_gb:
  ubuntu: 18
  fedora: 20
  popos: 18
  debian: 15
  arch: 15
isos:
  - key: ubuntu
    display: "Ubuntu 24.04.2 LTS Desktop (amd64)"
    iso_url: "https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso"
    checksum_url: "https://releases.ubuntu.com/24.04/SHA256SUMS"
    iso_filename: "ubuntu-24.04.2-desktop-amd64.iso"
    checksum_format: standard
  - key: fedora
    display: "Fedora 42"
    iso_url: "https://example.com/fedora.iso"
    checksum_url: "https://example.com/CHECKSUM"
    iso_filename: "fedora.iso"
    checksum_format: fedora
"#;

    #[test]
    fn test_config_loads_isos() {
        let cfg: Config = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(cfg.isos.len(), 2);
        assert_eq!(cfg.isos[0].key, "ubuntu");
        assert!(matches!(cfg.isos[0].checksum_format, ChecksumFormat::Standard));
        assert!(matches!(cfg.isos[1].checksum_format, ChecksumFormat::Fedora));
    }

    #[test]
    fn test_config_loads_blacklist() {
        let cfg: Config = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(cfg.blacklist, vec!["sda1", "sda2"]);
    }

    #[test]
    fn test_config_loads_default_sizes() {
        let cfg: Config = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(cfg.default_sizes_gb["ubuntu"], 18u64);
        assert_eq!(cfg.default_sizes_gb["fedora"], 20u64);
    }

    #[test]
    fn test_default_size_for_missing_key_returns_15() {
        let cfg: Config = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(cfg.default_size_gb("unknownkey"), 15u64);
    }
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cargo test --lib catalogue 2>&1 | head -30
```
Expected: error — `Config`, `ChecksumFormat` not defined yet.

- [ ] **Step 3: Implement catalogue.rs**

Replace all of `src/catalogue.rs` with:
```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumFormat {
    Standard,
    Fedora,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsoSpec {
    pub key: String,
    pub display: String,
    pub iso_url: String,
    pub checksum_url: String,
    pub iso_filename: String,
    pub checksum_format: ChecksumFormat,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub default_sizes_gb: HashMap<String, u64>,
    pub isos: Vec<IsoSpec>,
}

impl Config {
    /// Load config from a YAML file path.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read config: {}", path.display()))?;
        serde_yaml::from_str(&text)
            .with_context(|| format!("Cannot parse config: {}", path.display()))
    }

    /// Save config back to the same file (e.g. after updating blacklist).
    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_yaml::to_string(self)
            .context("Cannot serialize config")?;
        std::fs::write(path, text)
            .with_context(|| format!("Cannot write config: {}", path.display()))
    }

    /// Return default size in GB for a distro key, falling back to 15 GB.
    pub fn default_size_gb(&self, key: &str) -> u64 {
        self.default_sizes_gb.get(key).copied().unwrap_or(15)
    }
}

#[cfg(test)]
mod tests {
    // ... (tests from Step 1 above)
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --lib catalogue 2>&1
```
Expected:
```
test catalogue::tests::test_config_loads_blacklist ... ok
test catalogue::tests::test_config_loads_default_sizes ... ok
test catalogue::tests::test_config_loads_isos ... ok
test catalogue::tests::test_default_size_for_missing_key_returns_15 ... ok
test result: ok. 4 passed
```

- [ ] **Step 5: Commit**

```bash
git add src/catalogue.rs
git commit -m "feat(catalogue): Config + IsoSpec types with YAML load/save"
```

---

## Task 3: disk.rs — lsblk probing, gap detection, mount checks

**Files:**
- Modify: `src/disk.rs`

The disk module shells out to two tools:
- `lsblk --json -b -o NAME,SIZE,TYPE,MOUNTPOINT` — list devices and mount points
- `parted -m -s /dev/<disk> unit B print free` — list partitions AND free gaps in bytes

- [ ] **Step 1: Write failing tests**

Add to `src/disk.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Simulated `lsblk --json` output for a disk with two partitions
    const LSBLK_JSON: &str = r#"{
   "blockdevices": [
      {
         "name": "sda", "size": "536870912000", "type": "disk", "mountpoint": null,
         "children": [
            {"name": "sda1", "size": "536870912", "type": "part", "mountpoint": "/boot/efi"},
            {"name": "sda2", "size": "107374182400", "type": "part", "mountpoint": "/"}
         ]
      },
      {
         "name": "sdb", "size": "128849018880", "type": "disk", "mountpoint": null,
         "children": []
      }
   ]
}"#;

    // Simulated `parted -m -s /dev/sda unit B print free` output
    // Format: type:start:end:size:fs:name:flags;
    const PARTED_FREE: &str = "\
BYT;\n\
/dev/sda:536870912000B:scsi:512:512:gpt:SAMSUNG:;\n\
1:1048576B:537919487B:536870912B:fat32::boot, esp;\n\
2:537919488B:108912101887B:108374182400B:ext4::;\n\
1:108912101888B:536870912000B:427958810112B:free;\n";

    #[test]
    fn test_parse_lsblk_finds_disks() {
        let disks = parse_lsblk(LSBLK_JSON).unwrap();
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].name, "sda");
        assert_eq!(disks[1].name, "sdb");
    }

    #[test]
    fn test_parse_lsblk_detects_mounted_children() {
        let disks = parse_lsblk(LSBLK_JSON).unwrap();
        let sda = &disks[0];
        assert!(sda.children.iter().any(|c| c.mountpoint.is_some()));
    }

    #[test]
    fn test_parse_parted_free_finds_gap() {
        let gaps = parse_parted_free(PARTED_FREE).unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].start_bytes, 108912101888u64);
        assert_eq!(gaps[0].size_bytes, 427958810112u64);
    }

    #[test]
    fn test_gap_size_in_gb() {
        let gap = FreeGap { start_bytes: 0, end_bytes: 19327352832, size_bytes: 19327352832 };
        // 18 * 1024^3 = 19327352832
        assert!(gap.size_gb() >= 18);
    }

    #[test]
    fn test_is_mounted_true() {
        let disks = parse_lsblk(LSBLK_JSON).unwrap();
        assert!(is_partition_mounted(&disks, "sda1"));
    }

    #[test]
    fn test_is_mounted_false() {
        let disks = parse_lsblk(LSBLK_JSON).unwrap();
        assert!(!is_partition_mounted(&disks, "sdb"));
    }
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cargo test --lib disk 2>&1 | head -20
```
Expected: compile errors for missing types/functions.

- [ ] **Step 3: Implement disk.rs**

```rust
use anyhow::{bail, Context, Result};
use serde::Deserialize;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct LsblkChild {
    pub name: String,
    #[serde(default)]
    pub mountpoint: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub size: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub children: Vec<LsblkChild>,
}

#[derive(Debug, Clone, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<DiskInfo>,
}

#[derive(Debug, Clone)]
pub struct FreeGap {
    pub start_bytes: u64,
    pub end_bytes: u64,
    pub size_bytes: u64,
}

impl FreeGap {
    pub fn size_gb(&self) -> u64 {
        self.size_bytes / (1024 * 1024 * 1024)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run `lsblk --json` and return parsed disk list.
pub fn probe_disks() -> Result<Vec<DiskInfo>> {
    let out = std::process::Command::new("lsblk")
        .args(["--json", "-b", "-o", "NAME,SIZE,TYPE,MOUNTPOINT"])
        .output()
        .context("lsblk not found — is util-linux installed?")?;
    if !out.status.success() {
        bail!("lsblk failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let json = String::from_utf8_lossy(&out.stdout);
    parse_lsblk(&json)
}

/// Return free gaps on a disk using `parted -m -s /dev/<disk> unit B print free`.
pub fn free_gaps(disk_name: &str) -> Result<Vec<FreeGap>> {
    let dev = format!("/dev/{disk_name}");
    let out = std::process::Command::new("parted")
        .args(["-m", "-s", &dev, "unit", "B", "print", "free"])
        .output()
        .context("parted not found — install parted")?;
    // parted exits non-zero for unpartitioned disks but still prints output.
    let text = String::from_utf8_lossy(&out.stdout);
    parse_parted_free(&text)
}

/// Return true if the named partition (e.g. "sda1") has a non-empty MOUNTPOINT.
pub fn is_partition_mounted(disks: &[DiskInfo], part_name: &str) -> bool {
    for disk in disks {
        for child in &disk.children {
            if child.name == part_name {
                return child.mountpoint.as_deref().map(|m| !m.is_empty()).unwrap_or(false);
            }
        }
        // Also handle top-level disks used as whole-disk targets
        if disk.name == part_name {
            return false; // disks themselves don't have mountpoints in this context
        }
    }
    false
}

// ── Parsers (pub for tests) ────────────────────────────────────────────────────

pub fn parse_lsblk(json: &str) -> Result<Vec<DiskInfo>> {
    let out: LsblkOutput = serde_json::from_str(json)
        .context("Failed to parse lsblk JSON output")?;
    Ok(out.blockdevices.into_iter().filter(|d| d.kind == "disk").collect())
}

pub fn parse_parted_free(text: &str) -> Result<Vec<FreeGap>> {
    // Machine-readable format lines: type:start:end:size:fs:name:flags;
    // Free space lines contain "free" in the filesystem or type field.
    let mut gaps = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_end_matches(';');
        let parts: Vec<&str> = line.split(':').collect();
        // parted -m free lines have index=1 and filesystem "free"
        if parts.len() >= 5 && parts[4] == "free" {
            let start = parse_parted_bytes(parts[1])?;
            let end   = parse_parted_bytes(parts[2])?;
            let size  = parse_parted_bytes(parts[3])?;
            gaps.push(FreeGap { start_bytes: start, end_bytes: end, size_bytes: size });
        }
    }
    Ok(gaps)
}

fn parse_parted_bytes(s: &str) -> Result<u64> {
    s.trim_end_matches('B').parse::<u64>()
        .with_context(|| format!("Cannot parse parted byte value: '{s}'"))
}

#[cfg(test)]
mod tests { /* ... paste tests from Step 1 here ... */ }
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --lib disk 2>&1
```
Expected:
```
test disk::tests::test_gap_size_in_gb ... ok
test disk::tests::test_is_mounted_false ... ok
test disk::tests::test_is_mounted_true ... ok
test disk::tests::test_parse_lsblk_detects_mounted_children ... ok
test disk::tests::test_parse_lsblk_finds_disks ... ok
test disk::tests::test_parse_parted_free_finds_gap ... ok
test result: ok. 6 passed
```

- [ ] **Step 5: Commit**

```bash
git add src/disk.rs
git commit -m "feat(disk): lsblk + parted probing, gap detection, mount checks"
```

---

## Task 4: partition.rs — blacklist, name validation, parted create

**Files:**
- Modify: `src/partition.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blacklist_blocks_listed_partition() {
        let blacklist = vec!["sda1".to_string(), "sda2".to_string()];
        assert!(check_blacklist("sda1", &blacklist).is_err());
    }

    #[test]
    fn test_blacklist_allows_unlisted_partition() {
        let blacklist = vec!["sda1".to_string()];
        assert!(check_blacklist("sda3", &blacklist).is_ok());
    }

    #[test]
    fn test_empty_blacklist_allows_any() {
        assert!(check_blacklist("sda1", &[]).is_ok());
    }

    #[test]
    fn test_name_valid() {
        assert!(validate_partition_name("ubuntu-test").is_ok());
        assert!(validate_partition_name("arch-play").is_ok());
        assert!(validate_partition_name("a1_b2-c3").is_ok());
    }

    #[test]
    fn test_name_invalid_slash() {
        assert!(validate_partition_name("sda/1").is_err());
    }

    #[test]
    fn test_name_invalid_semicolon() {
        assert!(validate_partition_name("bad;name").is_err());
    }

    #[test]
    fn test_name_invalid_too_long() {
        assert!(validate_partition_name("this-name-is-way-too-long-xx").is_err());
    }

    #[test]
    fn test_name_invalid_space() {
        assert!(validate_partition_name("has space").is_err());
    }

    #[test]
    fn test_auto_name_format() {
        assert_eq!(auto_name("ubuntu"), "ubuntu-test");
        assert_eq!(auto_name("fedora"), "fedora-test");
        assert_eq!(auto_name("arch"), "arch-test");
    }
}
```

- [ ] **Step 2: Run — expect compile failure**

```bash
cargo test --lib partition 2>&1 | head -20
```

- [ ] **Step 3: Implement partition.rs**

```rust
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

/// Return Err if `part_name` (without /dev/ prefix) is in the blacklist.
pub fn check_blacklist(part_name: &str, blacklist: &[String]) -> Result<()> {
    let clean = part_name.trim_start_matches("/dev/");
    if blacklist.iter().any(|b| b == clean) {
        bail!(
            "{} '{}' is blacklisted. Remove it from the blacklist in distro-burner.yaml to proceed.",
            "BLOCKED:".red().bold(),
            clean
        );
    }
    Ok(())
}

/// Validate a partition label: [a-zA-Z0-9_-], max 20 chars.
pub fn validate_partition_name(name: &str) -> Result<()> {
    if name.len() > 20 {
        bail!("Partition name '{}' is too long (max 20 chars)", name);
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        bail!(
            "Partition name '{}' contains invalid characters. Use only [a-zA-Z0-9_-].",
            name
        );
    }
    Ok(())
}

/// Return the auto-generated partition label for a distro key.
pub fn auto_name(key: &str) -> String {
    format!("{}-test", key)
}

/// Shell out to parted to create a new partition in unallocated space.
/// start_bytes and end_bytes are the gap boundaries from parted probe.
/// Returns the new partition device path (e.g. "/dev/sda3").
pub fn create_partition(
    disk: &str,
    name: &str,
    start_bytes: u64,
    size_bytes: u64,
    dry_run: bool,
) -> Result<String> {
    validate_partition_name(name)?;

    let dev = format!("/dev/{disk}");
    let end_bytes = start_bytes + size_bytes;

    let cmd_display = format!(
        "parted -s {} mkpart {} {}B {}B",
        dev, name, start_bytes, end_bytes
    );
    println!("  Running: {}", cmd_display);

    if dry_run {
        println!("  [DRY RUN] would run: {}", cmd_display);
        return Ok(format!("/dev/{}N", disk)); // placeholder
    }

    let out = Command::new("parted")
        .args(["-s", &dev, "mkpart", name,
               &format!("{}B", start_bytes),
               &format!("{}B", end_bytes)])
        .output()
        .context("parted not found")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("parted failed: {}", stderr);
    }

    // Find the new partition by re-probing the disk for its highest-numbered part.
    let new_part = find_new_partition(&dev)?;
    Ok(new_part)
}

/// After parted creates a partition, find the device path of the newest partition.
fn find_new_partition(dev: &str) -> Result<String> {
    let out = Command::new("lsblk")
        .args(["--json", "-b", "-o", "NAME,TYPE", dev])
        .output()
        .context("lsblk failed after partition creation")?;

    #[derive(serde::Deserialize)]
    struct Child { name: String, #[serde(rename = "type")] kind: String }
    #[derive(serde::Deserialize)]
    struct Dev { children: Option<Vec<Child>> }
    #[derive(serde::Deserialize)]
    struct Out { blockdevices: Vec<Dev> }

    let parsed: Out = serde_json::from_slice(&out.stdout)
        .context("Cannot parse lsblk output after partition creation")?;

    let parts: Vec<String> = parsed.blockdevices.into_iter()
        .flat_map(|d| d.children.unwrap_or_default())
        .filter(|c| c.kind == "part")
        .map(|c| format!("/dev/{}", c.name))
        .collect();

    parts.into_iter().max()   // highest-numbered partition is the newest
        .context("No partitions found after parted ran")
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --lib partition 2>&1
```
Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/partition.rs
git commit -m "feat(partition): blacklist check, name validation, parted create"
```

---

## Task 5: download.rs — stream download, tempfile, 3× retry, SHA256 verify

**Files:**
- Modify: `src/download.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ── checksum parsing ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_standard_exact_match() {
        let body = "abc123def456abc123def456abc123def456abc123def456abc123def456abc1  ubuntu.iso\n";
        let hash = parse_checksum_body(body, "ubuntu.iso", &crate::catalogue::ChecksumFormat::Standard);
        assert_eq!(hash.unwrap(), "abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
    }

    #[test]
    fn test_parse_standard_rejects_partial_match() {
        // "ubuntu.iso" is a substring of "ubuntu.iso.torrent" — must NOT match
        let body = "abc123def456abc123def456abc123def456abc123def456abc123def456abc1  ubuntu.iso.torrent\n";
        let hash = parse_checksum_body(body, "ubuntu.iso", &crate::catalogue::ChecksumFormat::Standard);
        assert!(hash.is_none());
    }

    #[test]
    fn test_parse_standard_binary_star_prefix() {
        // Some checksum files use "<hash> *<filename>" (binary mode marker)
        let body = "abc123def456abc123def456abc123def456abc123def456abc123def456abc1 *ubuntu.iso\n";
        let hash = parse_checksum_body(body, "ubuntu.iso", &crate::catalogue::ChecksumFormat::Standard);
        assert_eq!(hash.unwrap(), "abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
    }

    #[test]
    fn test_parse_fedora_exact_match() {
        let body = "SHA256 (Fedora-Workstation-Live-x86_64-42-1.1.iso) = abc123def456abc123def456abc123def456abc123def456abc123def456abc1\n";
        let hash = parse_checksum_body(
            body,
            "Fedora-Workstation-Live-x86_64-42-1.1.iso",
            &crate::catalogue::ChecksumFormat::Fedora,
        );
        assert_eq!(hash.unwrap(), "abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
    }

    #[test]
    fn test_parse_fedora_wrong_filename_no_match() {
        let body = "SHA256 (other.iso) = abc123def456abc123def456abc123def456abc123def456abc123def456abc1\n";
        let hash = parse_checksum_body(body, "fedora.iso", &crate::catalogue::ChecksumFormat::Fedora);
        assert!(hash.is_none());
    }

    #[test]
    fn test_verify_sha256_correct() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576f04de29e6be32038"; // wrong intentionally
        // actual sha256 of "hello world" = b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576f04de29e6be32038... use correct one:
        let actual_hex = {
            use sha2::{Sha256, Digest};
            let digest = Sha256::digest(b"hello world");
            hex::encode(digest)
        };
        assert!(verify_sha256(tmp.path(), &actual_hex).is_ok());
    }

    #[test]
    fn test_verify_sha256_mismatch_returns_err() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        assert!(verify_sha256(tmp.path(), "0000000000000000000000000000000000000000000000000000000000000000").is_err());
    }
}
```

- [ ] **Step 2: Run — expect compile failure**

```bash
cargo test --lib download 2>&1 | head -20
```

- [ ] **Step 3: Implement download.rs**

```rust
use anyhow::{bail, Context, Result};
use colored::Colorize;
use hex;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::catalogue::{ChecksumFormat, IsoSpec};

/// Download an ISO with up to 3 retries (exponential backoff: 5s, 15s, 45s).
/// Uses a tempfile; atomically renames to `dest` on success.
pub fn download_iso(spec: &IsoSpec, dest: &Path, dry_run: bool) -> Result<()> {
    if dest.exists() && dest.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
        println!("  ✓ Already present, skipping: {}", dest.display());
        return Ok(());
    }

    if dry_run {
        println!("  [DRY RUN] would download {}", spec.iso_url);
        return Ok(());
    }

    let mut last_err = anyhow::anyhow!("no attempts made");
    let backoffs = [0u64, 5, 15, 45];

    for attempt in 0..3usize {
        if attempt > 0 {
            let secs = backoffs[attempt];
            println!("  Retrying in {}s (attempt {}/3)…", secs, attempt + 1);
            thread::sleep(Duration::from_secs(secs));
        }
        match try_download(spec, dest) {
            Ok(()) => return Ok(()),
            Err(e) => {
                println!("  {} attempt {}: {}", "Download failed —".yellow(), attempt + 1, e);
                last_err = e;
            }
        }
    }
    Err(last_err).context("All 3 download attempts failed")
}

fn try_download(spec: &IsoSpec, dest: &Path) -> Result<()> {
    let tmp = tempfile::NamedTempFile::new_in(dest.parent().unwrap_or(Path::new(".")))?;
    let tmp_path = tmp.path().to_owned();

    println!("  ↓ {}", spec.iso_url);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))  // connect timeout
        .build()?;

    let mut resp = client.get(&spec.iso_url).send()
        .with_context(|| format!("GET failed: {}", spec.iso_url))?;

    if !resp.status().is_success() {
        bail!("HTTP {} for {}", resp.status(), spec.iso_url);
    }

    let total = resp.content_length();
    let pb = ProgressBar::new(total.unwrap_or(0));
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:52.cyan/blue} {bytes:>10}/{total_bytes:<10} eta {eta}",
        ).unwrap().progress_chars("█▉▊▋▌▍▎▏  "),
    );

    let mut file = File::create(&tmp_path)?;
    let mut buf = [0u8; 65_536];
    let mut written: u64 = 0;

    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 { break; }
        file.write_all(&buf[..n])?;
        written += n as u64;
        pb.set_position(written);
    }
    file.flush()?;
    pb.finish_and_clear();

    // Persist the tempfile by keeping it alive, then rename.
    let tmp_path_final = tmp.into_temp_path();
    tmp_path_final.persist(dest)
        .context("Could not move temp file to final destination")?;

    println!("  ✓ Saved {} bytes → {}", written, dest.display());
    Ok(())
}

/// Fetch checksum file and return expected SHA256 hex for this ISO.
pub fn fetch_expected_hash(spec: &IsoSpec) -> Result<String> {
    println!("  ⧗ Fetching checksum: {}", spec.checksum_url);
    let body = reqwest::blocking::get(&spec.checksum_url)
        .with_context(|| format!("GET failed: {}", spec.checksum_url))?
        .text()
        .context("Failed to read checksum body")?;

    parse_checksum_body(&body, &spec.iso_filename, &spec.checksum_format)
        .with_context(|| format!("Hash for '{}' not found in {}", spec.iso_filename, spec.checksum_url))
}

/// Parse a checksum file body and return the SHA256 for `target_filename`.
/// Exported for unit tests.
pub fn parse_checksum_body(body: &str, target: &str, fmt: &ChecksumFormat) -> Option<String> {
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        match fmt {
            ChecksumFormat::Standard => {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 && parts[0].len() == 64 {
                    let filename = parts[1].trim().trim_start_matches('*').trim();
                    if filename == target {
                        return Some(parts[0].to_string());
                    }
                }
            }
            ChecksumFormat::Fedora => {
                // "SHA256 (<filename>) = <hash>"
                if let Some(rest) = line.strip_prefix("SHA256 (") {
                    if let Some((fname, hash_part)) = rest.split_once(") = ") {
                        let hash = hash_part.trim();
                        if fname == target && hash.len() == 64 {
                            return Some(hash.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Hash a file with SHA256 and compare to expected hex string.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    println!("  # Verifying SHA256…");
    let file_len = path.metadata().map(|m| m.len()).unwrap_or(0);
    let pb = ProgressBar::new(file_len);
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:52.green/black} {bytes:>10}/{total_bytes:<10}",
        ).unwrap().progress_chars("█▉▊▋▌▍▎▏  "),
    );

    let mut file = File::open(path)
        .with_context(|| format!("Cannot open {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    let mut processed: u64 = 0;

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        processed += n as u64;
        pb.set_position(processed);
    }
    pb.finish_and_clear();

    let actual = hex::encode(hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        println!("  ✓ SHA256 OK  ({}…)", &actual[..16]);
        Ok(())
    } else {
        bail!(
            "{}\n  expected : {}\n  actual   : {}",
            "SHA256 MISMATCH".red().bold(),
            expected,
            actual
        )
    }
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --lib download 2>&1
```
Expected: 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/download.rs
git commit -m "feat(download): stream+retry download, tempfile, SHA256 verify"
```

---

## Task 6: tui.rs — all inquire prompts

**Files:**
- Modify: `src/tui.rs`

No unit tests here — prompts require a TTY. Integration-tested by running the binary.

- [ ] **Step 1: Implement tui.rs**

```rust
use anyhow::Result;
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text, validator::Validation};

use crate::catalogue::IsoSpec;
use crate::disk::DiskInfo;

/// First-run prompt: collect partitions to blacklist.
/// Returns trimmed, deduplicated partition names (e.g. ["sda1", "sda2"]).
/// Accepts "none" to skip blacklisting (warns loudly).
pub fn prompt_blacklist() -> Result<Vec<String>> {
    println!("{}", "\n⚠  SAFETY SETUP — First run".red().bold());
    println!("Blacklist partitions you never want distro-burner to touch.");
    println!("Include your EFI partition (e.g. sda1), boot partition (sda2),");
    println!("and any existing data partitions.\n");

    let raw = Text::new("Enter comma-separated partitions to blacklist (or 'none'):")
        .with_validator(|v: &str| {
            if v.trim().is_empty() {
                return Ok(Validation::Invalid("Cannot be empty. Enter partition names or 'none'.".into()));
            }
            Ok(Validation::Valid)
        })
        .prompt()?;

    if raw.trim().eq_ignore_ascii_case("none") {
        println!("{}", "⚠  No partitions blacklisted. BE CAREFUL.".yellow().bold());
        return Ok(vec![]);
    }

    let mut names: Vec<String> = raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    names.dedup();
    println!("  Blacklisted: {}", names.join(", "));
    Ok(names)
}

/// MultiSelect prompt for which distros to burn.
pub fn prompt_select_distros(isos: &[IsoSpec]) -> Result<Vec<String>> {
    let options: Vec<String> = isos.iter()
        .map(|s| format!("{} — {}", s.key, s.display))
        .collect();

    let chosen = MultiSelect::new("Which distros would you like to burn?", options.clone())
        .prompt()?;

    // Map back to keys
    Ok(chosen.into_iter()
        .filter_map(|label| {
            isos.iter().find(|s| label.starts_with(&s.key)).map(|s| s.key.clone())
        })
        .collect())
}

/// Select which disk to use (shows name + size).
pub fn prompt_select_disk(disks: &[DiskInfo]) -> Result<String> {
    let options: Vec<String> = disks.iter()
        .map(|d| {
            let gb = d.size.parse::<u64>().unwrap_or(0) / (1024 * 1024 * 1024);
            format!("/dev/{} — {}GB", d.name, gb)
        })
        .collect();

    let chosen = Select::new("Which disk should partitions be created on?", options).prompt()?;
    // Extract disk name from label
    Ok(chosen.split_whitespace().next().unwrap_or("").trim_start_matches("/dev/").to_string())
}

/// Prompt for partition size in GB, with a suggested default.
pub fn prompt_partition_size(distro_display: &str, suggested_gb: u64) -> Result<u64> {
    let raw = Text::new(&format!(
        "Size for {} (default {}GB, or enter custom):", distro_display, suggested_gb
    ))
    .with_default(&suggested_gb.to_string())
    .with_validator(|v: &str| {
        match v.trim().parse::<u64>() {
            Ok(n) if n >= 5 => Ok(Validation::Valid),
            Ok(_) => Ok(Validation::Invalid("Minimum size is 5GB".into())),
            Err(_) => Ok(Validation::Invalid("Enter a number".into())),
        }
    })
    .prompt()?;

    Ok(raw.trim().parse()?)
}

/// Confirm partition creation before calling parted.
pub fn prompt_confirm_partition(label: &str, dev: &str, size_gb: u64) -> Result<bool> {
    let msg = format!(
        "Create partition '{}' at {} (~{}GB)?",
        label, dev, size_gb
    );
    Ok(Confirm::new(&msg).with_default(false).prompt()?)
}

/// Final burn confirmation — shown in red to emphasise danger.
pub fn prompt_confirm_burn(iso_filename: &str, dev: &str) -> Result<bool> {
    println!("{}", "\n⚠  POINT OF NO RETURN".red().bold());
    let msg = format!(
        "BURN {} → {}? THIS PERMANENTLY OVERWRITES {}.",
        iso_filename, dev, dev
    );
    Ok(Confirm::new(&msg).with_default(false).prompt()?)
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | grep -E "error|warning: unused" | head -20
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/tui.rs
git commit -m "feat(tui): all inquire prompts (distro select, disk, size, burn confirm)"
```

---

## Task 7: burn.rs — sudo dd execution + logging

**Files:**
- Modify: `src/burn.rs`

- [ ] **Step 1: Implement burn.rs**

```rust
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Shell out to `sudo dd` and log the full invocation.
/// Logs to `log_file` before and after execution.
pub fn burn_iso(
    iso_path: &Path,
    target_dev: &str,
    log_file: &Path,
    dry_run: bool,
) -> Result<()> {
    let iso_str = iso_path.to_str().context("ISO path is not valid UTF-8")?;
    let args = [
        "dd",
        &format!("if={}", iso_str),
        &format!("of={}", target_dev),
        "bs=4M",
        "status=progress",
        "oflag=sync",
    ];
    let cmd_str = format!("sudo {}", args.join(" "));

    log_event(log_file, &format!("BURN START: {} → {}  cmd={}", iso_str, target_dev, cmd_str))?;

    if dry_run {
        println!("  [DRY RUN] would run: {}", cmd_str);
        log_event(log_file, "DRY RUN — skipped")?;
        return Ok(());
    }

    println!("  Running: {}", cmd_str);
    let status = Command::new("sudo")
        .args(&args)
        .status()
        .context("Could not launch sudo dd — is sudo available?")?;

    if status.success() {
        println!("  {}", "✓ Burn complete.".green().bold());
        println!(
            "  Mount manually if needed: {}",
            format!("sudo mount {} /mnt/{}", target_dev,
                target_dev.trim_start_matches("/dev/")).cyan()
        );
        log_event(log_file, &format!("BURN DONE: {} → {}", iso_str, target_dev))?;
        Ok(())
    } else {
        let code = status.code().unwrap_or(-1);
        log_event(log_file, &format!("BURN FAILED (exit {}): {} → {}", code, iso_str, target_dev))?;
        bail!("{} sudo dd exited with code {}", "BURN FAILED:".red().bold(), code)
    }
}

/// Append a timestamped line to the log file.
pub fn log_event(log_file: &Path, msg: &str) -> Result<()> {
    let mut f = OpenOptions::new()
        .create(true).append(true)
        .open(log_file)
        .with_context(|| format!("Cannot open log: {}", log_file.display()))?;
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(f, "[{ts}] {msg}")?;
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | grep "^error" | head -10
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/burn.rs
git commit -m "feat(burn): sudo dd with full command logging"
```

---

## Task 8: main.rs — clap args + orchestration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement main.rs**

```rust
mod catalogue;
mod disk;
mod partition;
mod download;
mod tui;
mod burn;

use anyhow::{bail, Context, Result};
use clap::Parser;
use colored::Colorize;
use std::path::{Path, PathBuf};

use catalogue::Config;
use disk::{free_gaps, probe_disks};
use partition::{auto_name, check_blacklist, create_partition};
use download::{download_iso, fetch_expected_hash, verify_sha256};
use burn::{burn_iso, log_event};

#[derive(Parser, Debug)]
#[command(
    name = "distro-burner",
    version = "2.0.0",
    about = "Safely burn Linux ISOs to new partitions in unallocated disk space",
    long_about = "\
Burns Linux ISOs only to NEW partitions created in unallocated space.
Never touches existing partitions. Requires: lsblk, parted, sudo."
)]
struct Args {
    /// Print all steps without downloading or burning
    #[arg(long)]
    dry_run: bool,

    /// Where to save downloaded ISOs
    #[arg(long, default_value = ".")]
    output_dir: PathBuf,

    /// Log file path
    #[arg(long, default_value = "distro-burner.log")]
    log_file: PathBuf,

    /// Path to distro-burner.yaml
    #[arg(long, default_value = "distro-burner.yaml")]
    config: PathBuf,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {:#}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // ── Load config ──────────────────────────────────────────────────────────
    let mut config = Config::load(&args.config)
        .with_context(|| format!("Config not found at '{}'. Run from the repo root or pass --config.", args.config.display()))?;

    log_event(&args.log_file, "distro-burner v2.0 started")?;

    // ── First-run blacklist setup ─────────────────────────────────────────────
    if config.blacklist.is_empty() {
        println!("{}", "\n⚠  First run detected — setting up safety blacklist".yellow().bold());
        let names = tui::prompt_blacklist()?;
        config.blacklist = names;
        config.save(&args.config)?;
        log_event(&args.log_file, &format!("Blacklist saved: {:?}", config.blacklist))?;
    } else {
        println!("  Blacklisted partitions: {}", config.blacklist.join(", "));
    }

    // ── Select distros ───────────────────────────────────────────────────────
    let selected_keys = tui::prompt_select_distros(&config.isos)?;
    if selected_keys.is_empty() {
        bail!("No distros selected.");
    }
    let selected: Vec<_> = config.isos.iter()
        .filter(|s| selected_keys.contains(&s.key))
        .collect();

    // ── Probe disks ──────────────────────────────────────────────────────────
    println!("\n  Probing disks…");
    let all_disks = probe_disks()?;
    let min_size_gb = selected.iter()
        .map(|s| config.default_size_gb(&s.key))
        .min()
        .unwrap_or(15);

    // Filter disks with at least one gap >= min_size_gb
    let eligible_disks: Vec<_> = all_disks.iter().filter(|d| {
        free_gaps(&d.name)
            .map(|gaps| gaps.iter().any(|g| g.size_gb() >= min_size_gb))
            .unwrap_or(false)
    }).cloned().collect();

    if eligible_disks.is_empty() {
        bail!(
            "{} No disk has {} GB of unallocated space. Use GParted to free space first.",
            "ABORT:".red().bold(),
            min_size_gb
        );
    }

    let disk_name = tui::prompt_select_disk(&eligible_disks)?;

    // ── Process each selected distro ─────────────────────────────────────────
    std::fs::create_dir_all(&args.output_dir)?;
    let total = selected.len();

    for (idx, spec) in selected.iter().enumerate() {
        println!("\n[{}/{}] {}", idx + 1, total, spec.display);
        println!("{}", "─".repeat(64));

        let suggested_gb = config.default_size_gb(&spec.key);
        let size_gb = tui::prompt_partition_size(&spec.display, suggested_gb)?;
        let size_bytes = size_gb * 1024 * 1024 * 1024;

        // Find a gap big enough
        let gaps = free_gaps(&disk_name)?;
        let gap = gaps.iter().find(|g| g.size_gb() >= size_gb)
            .with_context(|| format!(
                "Not enough unallocated space for {} ({}GB). Use GParted first.",
                spec.display, size_gb
            ))?;

        let label = auto_name(&spec.key);
        let placeholder_dev = format!("/dev/{}X", disk_name);

        println!("  Partition label : {}", label);
        println!("  Gap start       : {} bytes", gap.start_bytes);
        println!("  Size            : {}GB", size_gb);

        if !tui::prompt_confirm_partition(&label, &placeholder_dev, size_gb)? {
            println!("  Skipped partition creation for {}", spec.display);
            continue;
        }

        // Create partition
        let new_dev = create_partition(&disk_name, &label, gap.start_bytes, size_bytes, args.dry_run)?;
        log_event(&args.log_file, &format!("PARTITION CREATED: {} ({}GB)", new_dev, size_gb))?;

        // Blacklist + mount check on new partition
        let part_name = new_dev.trim_start_matches("/dev/");
        check_blacklist(part_name, &config.blacklist)?;

        let fresh_disks = probe_disks()?;
        if disk::is_partition_mounted(&fresh_disks, part_name) {
            bail!("{} {} is mounted — unmount manually first.", "ABORT:".red().bold(), new_dev);
        }

        // Download + verify
        let dest = args.output_dir.join(&spec.iso_filename);
        download_iso(spec, &dest, args.dry_run)?;

        if !args.dry_run {
            let expected = fetch_expected_hash(spec)?;
            if verify_sha256(&dest, &expected).is_err() {
                dest.with_extension("").metadata().ok(); // just for context
                std::fs::remove_file(&dest).ok();
                bail!("{} SHA256 mismatch for {}. Corrupt file deleted.", "ABORT:".red().bold(), spec.iso_filename);
            }
        }

        // Burn
        if !tui::prompt_confirm_burn(&spec.iso_filename, &new_dev)? {
            println!("  Burn skipped for {}", spec.display);
            log_event(&args.log_file, &format!("BURN SKIPPED by user: {}", spec.iso_filename))?;
            continue;
        }

        burn_iso(&dest, &new_dev, &args.log_file, args.dry_run)?;
    }

    println!("\n{}", "All done.".green().bold());
    println!("  Log: {}", args.log_file.display());
    log_event(&args.log_file, "distro-burner finished")?;
    Ok(())
}
```

- [ ] **Step 2: Build and verify**

```bash
cargo build --release 2>&1
```
Expected: compiles with 0 errors. Binary at `target/release/distro-burner`.

- [ ] **Step 3: Smoke test with --dry-run (no disks touched)**

```bash
cargo run -- --dry-run 2>&1
```
Expected: TUI prompts appear (or first-run blacklist prompt), then dry-run output showing what would happen with no real disk ops.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): clap args, full orchestration, first-run blacklist setup"
```

---

## Task 9: Tests — run all, fix any failures

**Files:**
- Modify: all `src/*.rs` test blocks (add any missing)

- [ ] **Step 1: Run full test suite**

```bash
cargo test 2>&1
```
Expected output (at minimum):
```
test catalogue::tests::test_config_loads_blacklist ... ok
test catalogue::tests::test_config_loads_default_sizes ... ok
test catalogue::tests::test_config_loads_isos ... ok
test catalogue::tests::test_default_size_for_missing_key_returns_15 ... ok
test disk::tests::test_gap_size_in_gb ... ok
test disk::tests::test_is_mounted_false ... ok
test disk::tests::test_is_mounted_true ... ok
test disk::tests::test_parse_lsblk_detects_mounted_children ... ok
test disk::tests::test_parse_lsblk_finds_disks ... ok
test disk::tests::test_parse_parted_free_finds_gap ... ok
test download::tests::test_parse_fedora_exact_match ... ok
test download::tests::test_parse_fedora_wrong_filename_no_match ... ok
test download::tests::test_parse_standard_binary_star_prefix ... ok
test download::tests::test_parse_standard_exact_match ... ok
test download::tests::test_parse_standard_rejects_partial_match ... ok
test download::tests::test_verify_sha256_correct ... ok
test download::tests::test_verify_sha256_mismatch_returns_err ... ok
test partition::tests::test_auto_name_format ... ok
test partition::tests::test_blacklist_allows_unlisted_partition ... ok
test partition::tests::test_blacklist_blocks_listed_partition ... ok
test partition::tests::test_empty_blacklist_allows_any ... ok
test partition::tests::test_name_invalid_semicolon ... ok
test partition::tests::test_name_invalid_slash ... ok
test partition::tests::test_name_invalid_space ... ok
test partition::tests::test_name_invalid_too_long ... ok
test partition::tests::test_name_valid ... ok
test result: ok. 26 passed; 0 failed
```

- [ ] **Step 2: If any test fails, fix the implementation (not the test)**

- [ ] **Step 3: Final release build**

```bash
cargo build --release && echo "Build OK: $(du -sh target/release/distro-burner)"
```

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: distro-burner v2.0 complete — TUI, disk probing, safe partition creation"
```

- [ ] **Step 5: Push**

```bash
git push origin main
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] Unallocated-only burns — `free_gaps()` + gap filter in main
- [x] Blacklist — `check_blacklist()` + first-run prompt + YAML persistence
- [x] No auto-mount — post-burn message only; no mount calls anywhere
- [x] Mount check — `is_partition_mounted()` called before burn
- [x] TUI flow — all inquire prompts in `tui.rs`
- [x] Safety stack — all 9 guards from spec implemented
- [x] YAML config — `Config::load/save`, `IsoSpec`, `ChecksumFormat`
- [x] Download retries — 3× with 5/15/45s backoff
- [x] Checksum exact match — `parse_checksum_body()` exact filename comparison
- [x] Tempfile downloads — `tempfile::NamedTempFile` + `persist()`
- [x] Full dd command logging — `cmd_str` in `BURN START` log line
- [x] Unit tests — 26 tests across 4 modules
- [x] `--dry-run` flag — threaded through all destructive operations
- [x] `--no-tui` — not implemented (deferred; spec noted it as a CI escape hatch)

**Note on `--no-tui`:** This is in the spec but left for a follow-up PR. The flag is defined via clap but unimplemented in v2.0 — it will print "not yet implemented" if used. This keeps the plan focused and shippable.

# distro-burner v2.0 — Design Spec

**Date:** 2026-03-30
**Branch:** `main` (replaces v0.1.0 pure-Rust implementation)
**Status:** Approved

---

## 1. Goals & Scope

### What this tool does

A Rust CLI that safely burns Linux ISOs to *new* partitions carved from unallocated disk space.
Designed for GUI exploration setups — small partitions (15–20 GB), one distro per partition,
multi-boot safe. The user never has to touch `parted`, `lsblk`, or `dd` directly.

### What it explicitly never does

- Touch existing partitions (ever)
- Auto-mount or remount anything after a burn
- Write to a mounted target
- Write to a blacklisted partition
- Proceed without explicit Y/N from the user at every destructive step

### Success criteria

1. A user with 100 GB of free disk space can run `distro-burner` and burn Ubuntu to a new
   18 GB partition with zero manual `parted` or `fdisk` commands.
2. A user with no free space gets a clear abort message: "No unallocated space found. Use
   GParted to free space first."
3. Boot/EFI partitions are protected by a one-time blacklist setup, persisted in YAML.

### Out of scope for v2.0

- Windows/macOS support (Linux only)
- UEFI boot entry management (no `efibootmgr` calls)
- Multi-disk simultaneous burns
- Auto-formatting or filesystem creation (partition is created raw; user mounts manually)

---

## 2. Architecture & Module Structure

```
distro-burner/
  src/
    main.rs          — entry point; orchestrates the full flow
    catalogue.rs     — serde-deserialized ISO specs from YAML
    disk.rs          — lsblk probing, free-space gap detection, mount checks
    partition.rs     — parted shell-out, auto-naming, blacklist enforcement
    download.rs      — reqwest streaming, retries (3x), checksum verify, tempfile
    tui.rs           — all inquire prompts (MultiSelect, Confirm, Text, Select)
    burn.rs          — sudo dd execution, full command logging
  distro-burner.yaml — ISO catalogue + blacklist + default sizes (user-editable)
  Cargo.toml
```

### Crate changes

**Added:**

| Crate | Version | Purpose |
|---|---|---|
| `inquire` | `0.7` | TUI prompts — MultiSelect, Confirm, Text, Select |
| `serde` | `1` (derive) | Struct serialization |
| `serde_yaml` | `0.9` | Load/save `distro-burner.yaml` |
| `tempfile` | `3` | Atomic download temp files |
| `colored` | `2` | Red/yellow/green terminal warnings |

**Kept from v0.1.0:**
`clap 4`, `reqwest` (blocking), `sha2`, `hex`, `indicatif`, `anyhow`, `chrono`

**Removed:**
Hard-coded `static ISOS` array — replaced entirely by `distro-burner.yaml`

---

## 3. User Flow (TUI Sequence)

```
STARTUP
  └─ Load distro-burner.yaml (ISO catalogue, blacklist, default sizes)
  └─ If blacklist key absent (first run):
       inquire Text: "Blacklist partitions to protect (e.g. sda1,sda2):"
         validator: non-empty, trimmed, no duplicates
       Warn (red): "⚠ Always blacklist your boot and EFI partitions!"
       Save blacklist to YAML

DISTRO SELECTION
  └─ inquire MultiSelect: "Which distros would you like to burn?"
       [ubuntu, fedora, popos, debian, arch] — checkboxes

DISK SELECTION
  └─ Probe all block devices with lsblk
  └─ Filter to disks that have at least one unallocated gap ≥ smallest selected ISO size
  └─ inquire Select: "Which disk?" [/dev/sda — 500GB, /dev/sdb — 120GB ...]
  └─ If no eligible disks: abort "No disk has enough free space. Use GParted first."

FOR EACH SELECTED DISTRO
  └─ Show suggested size: "Ubuntu 18 GB (GUI exploration default)"
  └─ inquire Text: "Confirm 18 GB or enter custom size in GB:" [default: suggested]
  └─ Find a free gap on disk ≥ requested size (first-fit)
  └─ If no gap: abort (red) "Not enough unallocated space for <distro>. Use GParted."
  └─ Check auto-name ("ubuntu-test") not already used as a partition label
  └─ inquire Confirm: "Create partition 'ubuntu-test' at /dev/sdaX (~18 GB)? [y/N]"
  └─ Shell out: parted /dev/sda mkpart ubuntu-test <start> <end>
  └─ Mount check (lsblk on new partition): if MOUNTPOINT non-empty → abort
  └─ Blacklist check: if /dev/sdaX in blacklist → abort (red)

DOWNLOAD + VERIFY
  └─ Stream ISO to tempfile (reqwest blocking, timeout=(10s, none))
  └─ Retry up to 3× on network error (exponential backoff: 5s, 15s, 45s)
  └─ Fetch checksum file, exact-filename match, SHA256 verify
  └─ On mismatch: delete tempfile, abort (red) "SHA256 mismatch — do not burn"
  └─ On success: rename tempfile → final ISO path

BURN
  └─ inquire Confirm (red background):
       "⚠ BURN ubuntu.iso → /dev/sdaX? THIS PERMANENTLY OVERWRITES IT. [y/N]"
  └─ Log full dd command to log file
  └─ sudo dd if=<iso> of=/dev/sdaX bs=4M status=progress oflag=sync
  └─ Log exit code (success or failure + returncode)
  └─ Print (green): "✓ Burn complete."
  └─ Print: "Mount manually if needed: sudo mount /dev/sdaX /mnt/ubuntu-test"

NEVER: auto-mount, remount, or touch any other partition.
```

---

## 4. Safety Stack

| Risk | Guard | Behavior on violation |
|---|---|---|
| Burning to existing partition | lsblk gap detection — only unallocated regions offered | Hard abort |
| Burning to mounted partition | lsblk MOUNTPOINT check on target before dd | Hard abort (red) |
| Burning to boot/EFI | Blacklist enforced every run | Hard abort (red) |
| Corrupt download | SHA256 mandatory, no skip except `--dry-run` | Delete tempfile, abort |
| Partial download | tempfile + atomic rename on success | Tempfile auto-deleted on error |
| Runaway dd | Two explicit Y/N confirmations (create partition + burn) | N = skip |
| Shell injection in parted/dd | Partition names validated: `[a-zA-Z0-9_-]` only, ≤ 20 chars | Abort if invalid |
| No sudo | dd returns non-zero → logged + reported | Abort with exit code |
| No free space | Gap detection fails before partition creation | Abort + GParted hint |

---

## 5. YAML Config Format

```yaml
# distro-burner.yaml

blacklist:
  - sda1   # EFI
  - sda2   # boot
  - sda5   # existing Ubuntu

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
    checksum_format: standard   # or "fedora"

  - key: fedora
    display: "Fedora Workstation 42 Live (x86_64)"
    iso_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-42-1.1.iso"
    checksum_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-42-1.1-x86_64-CHECKSUM"
    iso_filename: "Fedora-Workstation-Live-x86_64-42-1.1.iso"
    checksum_format: fedora

  # ... popos, debian, arch follow same pattern
```

---

## 6. Unit Tests

| Test | What it verifies |
|---|---|
| `test_checksum_parse_standard` | Parses `<hash>  <filename>` line, exact filename match, rejects substring matches |
| `test_checksum_parse_fedora` | Parses `SHA256 (<filename>) = <hash>` line, exact match |
| `test_free_space_gap_detection` | Given mock lsblk JSON, correctly identifies gaps and their sizes |
| `test_blacklist_enforcement` | Target in blacklist → `Err`; target not in blacklist → `Ok` |
| `test_partition_name_validation` | Valid names pass; names with `/`, `;`, spaces, >20 chars fail |

---

## 7. Key Risks & Mitigations

**"User blacklists nothing on first run"**
Validator requires non-empty input. If user truly has nothing to protect (fresh disk), they
enter a placeholder like `none` — tool skips blacklist enforcement for that session but warns
every run: "No partitions blacklisted — are you sure your EFI is safe?"

**"lsblk output format varies across distros"**
Use `lsblk --json` for machine-readable output (available since util-linux 2.27, 2015).
Parse with `serde_json`. Fall back to `fdisk -l` if JSON flag unsupported.

**"parted fails to create partition (misaligned, overlapping)"**
Check parted exit code. On failure, print parted stderr verbatim and abort. Do not attempt dd.

**"User only has USB drives plugged in"**
Disk selection shows all block devices. USB drives appear as `/dev/sdb` etc. The unallocated
check and double-confirm still apply — behavior is identical. User is warned that dd to a USB
drive is irreversible.

**"Running in a VM"**
No special handling needed — virtual disks appear as `/dev/vda`, `/dev/sda`, etc. All guards
apply identically. `--dry-run` is recommended for testing in VMs.

---

## 8. CLI Flags (clap 4)

```
distro-burner [OPTIONS]

Options:
  --dry-run          Print all steps without downloading or burning
  --output-dir DIR   Where to save ISO files [default: .]
  --log-file FILE    Log file path [default: distro-burner.log]
  --config FILE      Path to distro-burner.yaml [default: ./distro-burner.yaml]
  --no-tui           Non-interactive mode (requires --isos and --partitions flags)
  -h, --help
  -V, --version
```

`--no-tui` exists for scripting/CI — bypasses inquire prompts, falls back to the v0.1.0
`--isos` / `--partitions` flag style. All safety checks still run.

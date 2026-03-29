//! distro-burner — download, verify, and burn Linux ISOs
//!
//! Examples:
//!   distro-burner --isos ubuntu,debian --partitions sda3,sda5
//!   distro-burner --isos all --partitions sda3,sda5,sda7,sda9,sda11 --dry-run
//!   distro-burner --isos fedora --dry-run   # no partition needed in dry-run

use anyhow::{bail, Context, Result};
use clap::Parser;
use hex;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::Local;

// ── ISO catalogue ────────────────────────────────────────────────────────────

/// Everything distro-burner needs to know about one ISO source.
struct IsoSpec {
    /// Short CLI key (used in --isos flag)
    key: &'static str,
    /// Human-readable label shown in progress output
    display: &'static str,
    /// Direct download URL for the ISO
    iso_url: &'static str,
    /// URL of the official checksum file
    checksum_url: &'static str,
    /// Local filename to save the ISO as
    iso_filename: &'static str,
    /// Substring to identify this ISO's line inside the checksum file.
    /// For most distros this equals iso_filename; for Arch it's a pattern
    /// because the checksum file uses dated names (archlinux-YYYY.MM.DD-…).
    checksum_grep: &'static str,
    checksum_format: ChecksumFmt,
}

#[derive(Clone, Copy)]
enum ChecksumFmt {
    /// Standard BSD/GNU style: "<hash>  <filename>"  (Ubuntu, Debian, Pop, Arch)
    Standard,
    /// Fedora style: "SHA256 (<filename>) = <hash>"
    Fedora,
}

/// Official mirrors + checksum sources.
/// NOTE: Point-release numbers (24.04.2, 12.9.0, etc.) can drift. If a download
/// 404s, check the distro's release page and update the URL + iso_filename here.
static ISOS: &[IsoSpec] = &[
    IsoSpec {
        key: "ubuntu",
        display: "Ubuntu 24.04.2 LTS Desktop (amd64)",
        iso_url: "https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso",
        checksum_url: "https://releases.ubuntu.com/24.04/SHA256SUMS",
        iso_filename: "ubuntu-24.04.2-desktop-amd64.iso",
        checksum_grep: "ubuntu-24.04.2-desktop-amd64.iso",
        checksum_format: ChecksumFmt::Standard,
    },
    IsoSpec {
        key: "fedora",
        display: "Fedora Workstation 42 Live (x86_64)",
        // Update the minor version (42-1.x) after release if needed.
        iso_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-42-1.1.iso",
        checksum_url: "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Workstation/x86_64/iso/Fedora-Workstation-42-1.1-x86_64-CHECKSUM",
        iso_filename: "Fedora-Workstation-Live-x86_64-42-1.1.iso",
        checksum_grep: "Fedora-Workstation-Live-x86_64-42-1.1.iso",
        checksum_format: ChecksumFmt::Fedora,
    },
    IsoSpec {
        key: "popos",
        display: "Pop!_OS 22.04 LTS Intel/AMD (amd64)",
        iso_url: "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/pop-os_22.04_amd64_intel_264.iso",
        checksum_url: "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/SHA256SUMS",
        iso_filename: "pop-os_22.04_amd64_intel_264.iso",
        checksum_grep: "pop-os_22.04_amd64_intel_264.iso",
        checksum_format: ChecksumFmt::Standard,
    },
    IsoSpec {
        key: "debian",
        display: "Debian 12.9 Netinstall (amd64)",
        iso_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.9.0-amd64-netinst.iso",
        checksum_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS",
        iso_filename: "debian-12.9.0-amd64-netinst.iso",
        checksum_grep: "debian-12.9.0-amd64-netinst.iso",
        checksum_format: ChecksumFmt::Standard,
    },
    IsoSpec {
        key: "arch",
        display: "Arch Linux latest (x86_64)",
        // Rackspace mirror carries a stable `archlinux-x86_64.iso` symlink.
        iso_url: "https://mirror.rackspace.com/archlinux/iso/latest/archlinux-x86_64.iso",
        checksum_url: "https://archlinux.org/iso/latest/sha256sums.txt",
        iso_filename: "archlinux-x86_64.iso",
        // The checksum file uses dated names; grep for the common suffix.
        checksum_grep: "x86_64.iso",
        checksum_format: ChecksumFmt::Standard,
    },
];

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "distro-burner",
    about = "Download, SHA256-verify, and burn Linux ISOs to disk partitions",
    long_about = "\
Download official Linux ISOs from canonical mirrors, verify their SHA256 checksums,
then burn each one to a target partition via `sudo dd`.

Requires sudo privileges for the burn step. Always double-check target partitions —
dd will OVERWRITE them without further warning beyond the Y/N prompt this tool shows.

Valid ISO keys: ubuntu, fedora, popos, debian, arch  (or 'all')"
)]
struct Args {
    /// ISOs to process: comma-separated keys or 'all'
    /// e.g. --isos ubuntu,debian  or  --isos all
    #[arg(long, default_value = "all")]
    isos: String,

    /// Target partitions, one per ISO, in matching order
    /// e.g. --partitions sda3,sda5   or   --partitions /dev/sda3,/dev/sda5
    /// Omit to download/verify only (no burning).
    #[arg(long)]
    partitions: Option<String>,

    /// Print what would happen without downloading or burning anything
    #[arg(long)]
    dry_run: bool,

    /// Directory where ISO files are saved (created if absent)
    #[arg(long, default_value = ".")]
    output_dir: PathBuf,

    /// Path to the log file
    #[arg(long, default_value = "distro-burner.log")]
    log_file: PathBuf,
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    let selected = parse_iso_selection(&args.isos)?;

    let partitions: Vec<String> = args
        .partitions
        .as_deref()
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Sanity: if partitions are given, count must match ISO count.
    if !partitions.is_empty() && partitions.len() != selected.len() {
        bail!(
            "Got {} ISO(s) but {} partition(s). Counts must match.",
            selected.len(),
            partitions.len()
        );
    }

    std::fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("Could not create output dir {:?}", &args.output_dir))?;

    log_event(
        &args.log_file,
        &format!(
            "distro-burner started  isos={}  dry_run={}  output_dir={:?}",
            args.isos, args.dry_run, args.output_dir
        ),
    )?;

    let total = selected.len();
    for (idx, spec) in selected.iter().enumerate() {
        println!("\n[{}/{}] {}", idx + 1, total, spec.display);
        println!("{}", "─".repeat(64));

        let iso_path = args.output_dir.join(spec.iso_filename);

        // Step 1 — download
        if args.dry_run {
            println!("  [DRY RUN] would download  {}", spec.iso_url);
            println!("  [DRY RUN] would save to   {:?}", iso_path);
        } else {
            download_iso(spec, &iso_path, &args.log_file)?;
        }

        // Step 2 — fetch expected hash from official checksum file
        let expected_hash = if args.dry_run {
            String::from("(skipped in dry-run)")
        } else {
            fetch_expected_hash(spec)
                .with_context(|| format!("Could not retrieve checksum for {}", spec.display))?
        };

        // Step 3 — verify SHA256
        if args.dry_run {
            println!("  [DRY RUN] would verify SHA256 of {:?}", iso_path);
        } else {
            verify_sha256(&iso_path, &expected_hash, spec, &args.log_file)?;
        }

        // Step 4 — burn (or show the dd command)
        let partition = partitions.get(idx).map(String::as_str);
        handle_burn(spec, &iso_path, partition, args.dry_run, &args.log_file)?;
    }

    println!("\nAll done. Log: {:?}", args.log_file);
    log_event(&args.log_file, "distro-burner finished successfully")?;
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve "ubuntu,fedora" or "all" into a list of IsoSpec references.
fn parse_iso_selection(isos_arg: &str) -> Result<Vec<&'static IsoSpec>> {
    if isos_arg.trim().eq_ignore_ascii_case("all") {
        return Ok(ISOS.iter().collect());
    }
    let mut result = Vec::new();
    for raw_key in isos_arg.split(',') {
        let key = raw_key.trim().to_lowercase();
        match ISOS.iter().find(|s| s.key == key) {
            Some(spec) => result.push(spec),
            None => bail!(
                "Unknown ISO key '{}'. Valid keys: ubuntu, fedora, popos, debian, arch",
                key
            ),
        }
    }
    if result.is_empty() {
        bail!("No ISOs selected. Pass --isos ubuntu,fedora (etc.) or --isos all.");
    }
    Ok(result)
}

/// Append a timestamped line to the log file.
fn log_event(log_file: &Path, msg: &str) -> Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .with_context(|| format!("Cannot open log file {:?}", log_file))?;
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(f, "[{ts}] {msg}")?;
    Ok(())
}

/// Stream-download an ISO, showing a live byte-count progress bar.
/// Skips re-download if the file already exists and is non-empty.
fn download_iso(spec: &IsoSpec, dest: &Path, log_file: &Path) -> Result<()> {
    // Skip if already present (idempotent runs).
    if dest.exists() && dest.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
        println!("  ✓ Already present, skipping download: {:?}", dest);
        log_event(log_file, &format!("SKIP (exists): {}", spec.iso_filename))?;
        return Ok(());
    }

    println!("  ↓ {}", spec.iso_url);
    log_event(log_file, &format!("DOWNLOAD START: {}", spec.iso_url))?;

    let mut resp = reqwest::blocking::get(spec.iso_url)
        .with_context(|| format!("HTTP GET failed: {}", spec.iso_url))?;

    if !resp.status().is_success() {
        bail!("Server returned HTTP {} for {}", resp.status(), spec.iso_url);
    }

    let total = resp.content_length();
    let pb = ProgressBar::new(total.unwrap_or(0));
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:52.cyan/blue} {bytes:>10}/{total_bytes:<10} eta {eta}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    let mut file = File::create(dest)
        .with_context(|| format!("Cannot create file {:?}", dest))?;

    let mut buf = [0u8; 65_536]; // 64 KiB chunks
    let mut total_written: u64 = 0;

    loop {
        let n = resp.read(&mut buf).context("Stream read error")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .with_context(|| format!("Write error to {:?}", dest))?;
        total_written += n as u64;
        pb.set_position(total_written);
    }

    pb.finish_and_clear();
    println!("  ✓ Saved {} bytes → {:?}", total_written, dest);
    log_event(
        log_file,
        &format!("DOWNLOAD DONE: {} ({} bytes)", spec.iso_filename, total_written),
    )?;
    Ok(())
}

/// Fetch the distro's official checksum file and extract the hash for this ISO.
fn fetch_expected_hash(spec: &IsoSpec) -> Result<String> {
    println!("  ⧗ Fetching checksum: {}", spec.checksum_url);
    let body = reqwest::blocking::get(spec.checksum_url)
        .with_context(|| format!("GET failed: {}", spec.checksum_url))?
        .text()
        .context("Failed to read checksum response")?;

    parse_hash_from_body(&body, spec)
        .with_context(|| format!("'{}' not found in {}", spec.checksum_grep, spec.checksum_url))
}

/// Parse a hash out of a checksum file body.
/// Supports two formats: Standard ("hash  filename") and Fedora ("SHA256 (filename) = hash").
/// Uses `spec.checksum_grep` as a substring match, so Arch's dated filename is handled
/// without knowing the exact release date.
fn parse_hash_from_body(body: &str, spec: &IsoSpec) -> Option<String> {
    for line in body.lines() {
        let line = line.trim();
        if !line.contains(spec.checksum_grep) {
            continue;
        }
        let hash = match spec.checksum_format {
            ChecksumFmt::Standard => {
                // "<hash>  <filename>"  or  "<hash> *<filename>"
                line.split_whitespace().next().map(str::to_string)
            }
            ChecksumFmt::Fedora => {
                // "SHA256 (Fedora-…-42-1.1.iso) = <hash>"
                line.split(" = ").nth(1).map(|h| h.trim().to_string())
            }
        };
        if let Some(h) = hash {
            if h.len() == 64 {
                // Looks like a valid SHA-256 hex string
                return Some(h);
            }
        }
    }
    None
}

/// Hash the downloaded file and compare against the expected value.
/// Bails loudly on mismatch so a corrupted ISO never reaches dd.
fn verify_sha256(
    iso_path: &Path,
    expected: &str,
    spec: &IsoSpec,
    log_file: &Path,
) -> Result<()> {
    println!("  # Verifying SHA256 …");

    let file_len = iso_path.metadata().map(|m| m.len()).unwrap_or(0);
    let pb = ProgressBar::new(file_len);
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:52.green/black} {bytes:>10}/{total_bytes:<10}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    let mut file = File::open(iso_path)
        .with_context(|| format!("Cannot open {:?} for hashing", iso_path))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    let mut processed: u64 = 0;

    loop {
        let n = file.read(&mut buf).context("Read error during hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        processed += n as u64;
        pb.set_position(processed);
    }

    pb.finish_and_clear();

    let actual = hex::encode(hasher.finalize());

    if actual.eq_ignore_ascii_case(expected) {
        // Only print first 16 chars of hash to keep output tidy
        println!("  ✓ SHA256 OK  ({}…)", &actual[..16]);
        log_event(log_file, &format!("SHA256 PASS: {}", spec.iso_filename))?;
    } else {
        log_event(
            log_file,
            &format!(
                "SHA256 FAIL: {}  expected={}  actual={}",
                spec.iso_filename, expected, actual
            ),
        )?;
        bail!(
            "SHA256 mismatch for {}!\n  expected : {}\n  actual   : {}",
            spec.iso_filename,
            expected,
            actual
        );
    }

    Ok(())
}

/// Prompt the user and, if confirmed, shell out to `sudo dd`.
/// With --dry-run, only prints the command that would be executed.
/// If no partition was given, the burn step is skipped entirely.
fn handle_burn(
    spec: &IsoSpec,
    iso_path: &Path,
    partition: Option<&str>,
    dry_run: bool,
    log_file: &Path,
) -> Result<()> {
    let Some(part) = partition else {
        println!("  ⊘ No partition specified — skipping burn for {}", spec.display);
        return Ok(());
    };

    // Reject partition names with shell metacharacters to prevent injection.
    let safe_chars = |c: char| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-';
    if !part.chars().all(safe_chars) {
        bail!("Refusing suspicious partition name '{}' (contains shell metacharacters)", part);
    }

    // Accept both "sda3" and "/dev/sda3".
    let dev = if part.starts_with("/dev/") {
        part.to_string()
    } else {
        format!("/dev/{}", part)
    };

    let iso_str = iso_path
        .to_str()
        .context("ISO path is not valid UTF-8")?;

    // The full dd invocation for display / execution.
    let dd_cmd = format!(
        "sudo dd if={} of={} bs=4M status=progress oflag=sync",
        iso_str, dev
    );

    if dry_run {
        println!("  [DRY RUN] would run: {}", dd_cmd);
        log_event(log_file, &format!("DRY RUN burn: {}", dd_cmd))?;
        return Ok(());
    }

    // ── Y/N confirmation ─────────────────────────────────────────────
    println!();
    println!("  Target : {}", dev);
    println!("  Source : {:?}", iso_path);
    println!("  Command: {}", dd_cmd);
    println!();
    println!("  ⚠  WARNING: this will PERMANENTLY OVERWRITE {dev}.");
    print!("  Proceed? [y/N] ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Failed to read user input")?;

    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        println!("  Skipped burn for {}", dev);
        log_event(log_file, &format!("BURN SKIPPED by user: {} → {}", spec.iso_filename, dev))?;
        return Ok(());
    }

    // ── Run sudo dd ──────────────────────────────────────────────────
    log_event(
        log_file,
        &format!("BURN START: {} → {}  cmd={}", spec.iso_filename, dev, dd_cmd),
    )?;

    let status = Command::new("sudo")
        .args([
            "dd",
            &format!("if={}", iso_str),
            &format!("of={}", dev),
            "bs=4M",
            "status=progress",
            "oflag=sync",
        ])
        .status()
        .context("Failed to spawn sudo dd — is sudo available?")?;

    if status.success() {
        println!("  ✓ Burn complete: {} → {}", spec.iso_filename, dev);
        log_event(log_file, &format!("BURN DONE: {} → {}", spec.iso_filename, dev))?;
    } else {
        let code = status.code().unwrap_or(-1);
        log_event(
            log_file,
            &format!("BURN FAILED (exit {}): {} → {}", code, spec.iso_filename, dev),
        )?;
        bail!(
            "sudo dd exited with code {} for {}. Check {:?} for details.",
            code,
            dev,
            log_file
        );
    }

    Ok(())
}

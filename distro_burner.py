#!/usr/bin/env python3
"""
distro_burner.py — download, SHA256-verify, and burn Linux ISOs.

Can be run directly:
    python3 distro_burner.py --isos ubuntu,debian --partitions sda3,sda5

Or invoked by the Rust wrapper binary:
    distro-burner --isos ubuntu,debian --partitions sda3,sda5
"""

import argparse
import hashlib
import logging
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path

# ── Optional deps (graceful fallbacks if not installed) ──────────────────────
try:
    import requests
except ImportError:
    sys.exit("Missing dependency: pip install requests")

try:
    from tqdm import tqdm
    HAS_TQDM = True
except ImportError:
    HAS_TQDM = False  # Falls back to periodic print statements

# ── ISO catalogue ─────────────────────────────────────────────────────────────

# Each entry: (key, display_name, iso_url, checksum_url, iso_filename,
#              checksum_grep, checksum_format)
#
# checksum_grep  : substring to find this ISO's line in the checksum file
# checksum_format: "standard" = "<hash>  <filename>"
#                  "fedora"   = "SHA256 (<filename>) = <hash>"
#
# NOTE: Point-release numbers drift — if a URL 404s, check the distro's
# release page and update the version string here.

ISO_CATALOGUE = [
    {
        "key": "ubuntu",
        "display": "Ubuntu 24.04.2 LTS Desktop (amd64)",
        "iso_url": "https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso",
        "checksum_url": "https://releases.ubuntu.com/24.04/SHA256SUMS",
        "iso_filename": "ubuntu-24.04.2-desktop-amd64.iso",
        "checksum_grep": "ubuntu-24.04.2-desktop-amd64.iso",
        "checksum_format": "standard",
    },
    {
        "key": "fedora",
        "display": "Fedora Workstation 42 Live (x86_64)",
        "iso_url": (
            "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/"
            "Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-42-1.1.iso"
        ),
        "checksum_url": (
            "https://dl.fedoraproject.org/pub/fedora/linux/releases/42/"
            "Workstation/x86_64/iso/Fedora-Workstation-42-1.1-x86_64-CHECKSUM"
        ),
        "iso_filename": "Fedora-Workstation-Live-x86_64-42-1.1.iso",
        "checksum_grep": "Fedora-Workstation-Live-x86_64-42-1.1.iso",
        "checksum_format": "fedora",
    },
    {
        "key": "popos",
        "display": "Pop!_OS 22.04 LTS Intel/AMD (amd64)",
        "iso_url": "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/pop-os_22.04_amd64_intel_264.iso",
        "checksum_url": "https://iso.pop-os.org/22.04/amd64/intel/22.04.4/SHA256SUMS",
        "iso_filename": "pop-os_22.04_amd64_intel_264.iso",
        "checksum_grep": "pop-os_22.04_amd64_intel_264.iso",
        "checksum_format": "standard",
    },
    {
        "key": "debian",
        "display": "Debian 12.9 Netinstall (amd64)",
        "iso_url": "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.9.0-amd64-netinst.iso",
        "checksum_url": "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS",
        "iso_filename": "debian-12.9.0-amd64-netinst.iso",
        "checksum_grep": "debian-12.9.0-amd64-netinst.iso",
        "checksum_format": "standard",
    },
    {
        "key": "arch",
        "display": "Arch Linux latest (x86_64)",
        # Rackspace hosts a stable symlink; checksum file uses dated names —
        # we match on "x86_64.iso" substring to avoid needing the exact date.
        "iso_url": "https://mirror.rackspace.com/archlinux/iso/latest/archlinux-x86_64.iso",
        "checksum_url": "https://archlinux.org/iso/latest/sha256sums.txt",
        "iso_filename": "archlinux-x86_64.iso",
        "checksum_grep": "x86_64.iso",
        "checksum_format": "standard",
    },
]

ISO_BY_KEY = {entry["key"]: entry for entry in ISO_CATALOGUE}

# ── CLI ───────────────────────────────────────────────────────────────────────

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="distro-burner",
        description="Download, SHA256-verify, and burn Linux ISOs to disk partitions.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "ISO keys: ubuntu  fedora  popos  debian  arch  (or 'all')\n\n"
            "Examples:\n"
            "  %(prog)s --isos ubuntu,debian --partitions sda3,sda5\n"
            "  %(prog)s --isos all --partitions sda3,sda5,sda7,sda9,sda11 --dry-run\n"
            "  %(prog)s --isos fedora --output-dir ~/isos   # download + verify only\n"
        ),
    )
    p.add_argument(
        "--isos",
        default="all",
        metavar="KEYS",
        help="Comma-separated ISO keys or 'all'  (default: all)",
    )
    p.add_argument(
        "--partitions",
        default="",
        metavar="PARTS",
        help="Comma-separated partitions, one per ISO, e.g. sda3,sda5 or /dev/sda3,/dev/sda5",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would happen; download/burn nothing",
    )
    p.add_argument(
        "--output-dir",
        default=".",
        metavar="DIR",
        help="Directory to save ISOs  (default: current dir)",
    )
    p.add_argument(
        "--log-file",
        default="distro-burner.log",
        metavar="FILE",
        help="Log file path  (default: distro-burner.log)",
    )
    return p

# ── Logging ───────────────────────────────────────────────────────────────────

def setup_logger(log_path: str) -> logging.Logger:
    logger = logging.getLogger("distro-burner")
    logger.setLevel(logging.DEBUG)

    fmt = logging.Formatter("[%(asctime)s] %(levelname)s %(message)s",
                            datefmt="%Y-%m-%d %H:%M:%S")

    # File handler — always on
    fh = logging.FileHandler(log_path, encoding="utf-8")
    fh.setFormatter(fmt)
    logger.addHandler(fh)

    # Console handler — INFO and above
    ch = logging.StreamHandler()
    ch.setLevel(logging.INFO)
    ch.setFormatter(logging.Formatter("  %(message)s"))
    logger.addHandler(ch)

    return logger

# ── Helpers ───────────────────────────────────────────────────────────────────

def parse_iso_selection(isos_arg: str) -> list[dict]:
    """Return list of IsoSpec dicts matching the user's --isos value."""
    if isos_arg.strip().lower() == "all":
        return list(ISO_CATALOGUE)
    selected = []
    for raw in isos_arg.split(","):
        key = raw.strip().lower()
        if key not in ISO_BY_KEY:
            valid = ", ".join(ISO_BY_KEY)
            sys.exit(f"Unknown ISO key '{key}'. Valid keys: {valid}")
        selected.append(ISO_BY_KEY[key])
    if not selected:
        sys.exit("No ISOs selected.")
    return selected


def download_iso(spec: dict, dest: Path, logger: logging.Logger) -> None:
    """Stream-download an ISO to dest, showing a progress bar."""
    # Idempotent: skip if already present and non-empty.
    if dest.exists() and dest.stat().st_size > 0:
        logger.info("SKIP (exists): %s", dest.name)
        print(f"  ✓ Already present, skipping: {dest.name}")
        return

    logger.info("DOWNLOAD START: %s", spec["iso_url"])
    print(f"  ↓ {spec['iso_url']}")

    with requests.get(spec["iso_url"], stream=True, timeout=60) as resp:
        resp.raise_for_status()
        total = int(resp.headers.get("content-length", 0))

        if HAS_TQDM:
            bar = tqdm(
                total=total or None,
                unit="B",
                unit_scale=True,
                unit_divisor=1024,
                desc=f"  {dest.name[:40]}",
                ncols=72,
                leave=False,
            )
        else:
            bar = None
            print(f"  (tqdm not installed — progress suppressed)")

        written = 0
        chunk_size = 65_536  # 64 KiB

        with open(dest, "wb") as f:
            for chunk in resp.iter_content(chunk_size=chunk_size):
                f.write(chunk)
                written += len(chunk)
                if bar:
                    bar.update(len(chunk))

        if bar:
            bar.close()

    print(f"  ✓ Saved {written:,} bytes → {dest}")
    logger.info("DOWNLOAD DONE: %s (%d bytes)", dest.name, written)


def fetch_expected_hash(spec: dict, logger: logging.Logger) -> str:
    """Download the distro's checksum file and extract the SHA256 for this ISO."""
    logger.debug("Fetching checksum: %s", spec["checksum_url"])
    print(f"  ⧗ Fetching checksum: {spec['checksum_url']}")

    resp = requests.get(spec["checksum_url"], timeout=30)
    resp.raise_for_status()
    body = resp.text

    grep = spec["checksum_grep"]
    fmt  = spec["checksum_format"]

    for line in body.splitlines():
        line = line.strip()
        if grep not in line:
            continue
        if fmt == "standard":
            # "<hash>  <filename>"  or  "<hash> *<filename>"
            parts = line.split(None, 1)
            if len(parts) == 2 and len(parts[0]) == 64:
                return parts[0]
        elif fmt == "fedora":
            # "SHA256 (<filename>) = <hash>"
            m = re.search(r"=\s+([0-9a-fA-F]{64})$", line)
            if m:
                return m.group(1)

    raise RuntimeError(
        f"Hash for '{grep}' not found in {spec['checksum_url']}"
    )


def verify_sha256(dest: Path, expected: str, spec: dict, logger: logging.Logger) -> None:
    """Hash the local file and bail if it doesn't match expected."""
    print(f"  # Verifying SHA256 …")
    total = dest.stat().st_size
    hasher = hashlib.sha256()

    if HAS_TQDM:
        bar = tqdm(
            total=total,
            unit="B",
            unit_scale=True,
            unit_divisor=1024,
            desc=f"  hashing",
            ncols=72,
            leave=False,
        )
    else:
        bar = None

    chunk_size = 65_536
    with open(dest, "rb") as f:
        while chunk := f.read(chunk_size):
            hasher.update(chunk)
            if bar:
                bar.update(len(chunk))

    if bar:
        bar.close()

    actual = hasher.hexdigest()

    if actual.lower() == expected.lower():
        print(f"  ✓ SHA256 OK  ({actual[:16]}…)")
        logger.info("SHA256 PASS: %s", spec["iso_filename"])
    else:
        logger.error(
            "SHA256 FAIL: %s  expected=%s  actual=%s",
            spec["iso_filename"], expected, actual,
        )
        sys.exit(
            f"\n  ✗ SHA256 MISMATCH for {spec['iso_filename']}!\n"
            f"    expected : {expected}\n"
            f"    actual   : {actual}\n"
            f"  Deleting corrupt file."
        )
        # Remove corrupt file so a re-run triggers a fresh download.
        dest.unlink(missing_ok=True)


def handle_burn(
    spec: dict,
    dest: Path,
    partition: str | None,
    dry_run: bool,
    logger: logging.Logger,
) -> None:
    """Prompt Y/N and shell out to sudo dd, or print the command in dry-run."""
    if not partition:
        print(f"  ⊘ No partition given — skipping burn for {spec['display']}")
        return

    # Validate partition name to prevent shell injection.
    if not re.fullmatch(r"[a-zA-Z0-9/_-]+", partition):
        sys.exit(f"Refusing suspicious partition name '{partition}'")

    dev = partition if partition.startswith("/dev/") else f"/dev/{partition}"
    cmd = f"sudo dd if={dest} of={dev} bs=4M status=progress oflag=sync"

    if dry_run:
        print(f"  [DRY RUN] would run: {cmd}")
        logger.info("DRY RUN burn: %s", cmd)
        return

    # ── Y/N confirmation ─────────────────────────────────────────
    print()
    print(f"  Target  : {dev}")
    print(f"  Source  : {dest}")
    print(f"  Command : {cmd}")
    print()
    print(f"  ⚠  WARNING: this will PERMANENTLY OVERWRITE {dev}.")
    answer = input("  Proceed? [y/N] ").strip().lower()

    if answer not in ("y", "yes"):
        print(f"  Skipped burn for {dev}")
        logger.info("BURN SKIPPED by user: %s → %s", spec["iso_filename"], dev)
        return

    # ── Run sudo dd ───────────────────────────────────────────────
    logger.info("BURN START: %s → %s", spec["iso_filename"], dev)

    result = subprocess.run(
        ["sudo", "dd", f"if={dest}", f"of={dev}", "bs=4M", "status=progress", "oflag=sync"],
        check=False,
    )

    if result.returncode == 0:
        print(f"  ✓ Burn complete: {spec['iso_filename']} → {dev}")
        logger.info("BURN DONE: %s → %s", spec["iso_filename"], dev)
    else:
        logger.error(
            "BURN FAILED (exit %d): %s → %s",
            result.returncode, spec["iso_filename"], dev,
        )
        sys.exit(f"sudo dd failed with exit code {result.returncode} for {dev}")

# ── Main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    selected   = parse_iso_selection(args.isos)
    partitions = [p.strip() for p in args.partitions.split(",")] if args.partitions else []

    # Partition count must match ISO count when partitions are supplied.
    if partitions and len(partitions) != len(selected):
        sys.exit(
            f"Got {len(selected)} ISO(s) but {len(partitions)} partition(s). "
            "Counts must match."
        )

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    logger = setup_logger(args.log_file)
    logger.info(
        "distro-burner started  isos=%s  dry_run=%s  output_dir=%s",
        args.isos, args.dry_run, output_dir,
    )

    total = len(selected)
    for idx, spec in enumerate(selected):
        print(f"\n[{idx + 1}/{total}] {spec['display']}")
        print("─" * 64)

        dest = output_dir / spec["iso_filename"]

        # 1 — Download
        if args.dry_run:
            print(f"  [DRY RUN] would download  {spec['iso_url']}")
            print(f"  [DRY RUN] would save to   {dest}")
        else:
            download_iso(spec, dest, logger)

        # 2 — Fetch expected hash
        if args.dry_run:
            expected_hash = "(skipped in dry-run)"
        else:
            expected_hash = fetch_expected_hash(spec, logger)

        # 3 — Verify SHA256
        if args.dry_run:
            print(f"  [DRY RUN] would verify SHA256 of {dest}")
        else:
            verify_sha256(dest, expected_hash, spec, logger)

        # 4 — Burn
        partition = partitions[idx] if partitions else None
        handle_burn(spec, dest, partition, args.dry_run, logger)

    print(f"\nAll done. Log: {args.log_file}")
    logger.info("distro-burner finished successfully")


if __name__ == "__main__":
    main()

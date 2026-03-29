# distro-burner

A fast Rust CLI that downloads five major Linux ISOs from official mirrors, verifies their SHA256 checksums, and burns each one to a target partition via `sudo dd`.

```
distro-burner --isos ubuntu,debian --partitions sda3,sda5
```

---

## Features

- **Downloads from canonical mirrors** — Ubuntu, Fedora, Pop!_OS, Debian, Arch Linux
- **SHA256 verification** — fetches official checksum files and compares; bails loudly on any mismatch so a corrupt ISO never reaches `dd`
- **Live progress bars** — byte-count display during both download and hashing (via `indicatif`)
- **Y/N prompt before every burn** — no silent overwrites
- **Idempotent downloads** — skips re-downloading files that already exist and are non-empty
- **Dry-run mode** — prints what would happen without touching the filesystem or any disk
- **Persistent log file** — every event timestamped, including failures and user skips
- **Injection-safe** — partition names are validated before being passed to `sudo dd`

---

## Supported ISOs

| Key      | Distro                              | Mirror                                      |
|----------|-------------------------------------|---------------------------------------------|
| `ubuntu` | Ubuntu 24.04.2 LTS Desktop (amd64)  | releases.ubuntu.com                         |
| `fedora` | Fedora Workstation 42 Live (x86_64) | dl.fedoraproject.org                        |
| `popos`  | Pop!\_OS 22.04 LTS Intel/AMD        | iso.pop-os.org                              |
| `debian` | Debian 12.9 Netinstall (amd64)      | cdimage.debian.org                          |
| `arch`   | Arch Linux latest (x86\_64)         | mirror.rackspace.com / archlinux.org        |

> **Note on URLs:** Point-release numbers (e.g. `24.04.2`, `12.9.0`, `42-1.1`) drift between distro releases. If a download returns 404, check the distro's official release page and update the relevant `IsoSpec` in `src/main.rs`. Each entry is clearly commented.

---

## Installation

### Prerequisites

- Rust toolchain — [rustup.rs](https://rustup.rs)
- `sudo` access on the machine (only needed for the burn step)
- `gh` CLI (optional, for repo management)

### Build

```zsh
git clone https://github.com/tom2025b/distro-burner
cd distro-burner
cargo build --release
# Binary at: ./target/release/distro-burner
```

---

## Usage

### Download + verify + burn

```zsh
# Burn two ISOs to two partitions (order must match)
distro-burner --isos ubuntu,debian --partitions sda3,sda5

# Burn all five ISOs
distro-burner --isos all --partitions sda3,sda5,sda7,sda9,sda11
```

### Download + verify only (no burn)

```zsh
# Omit --partitions to skip the dd step entirely
distro-burner --isos fedora,arch --output-dir ~/isos
```

### Dry run (print commands, touch nothing)

```zsh
distro-burner --isos all --partitions sda3,sda5,sda7,sda9,sda11 --dry-run
```

### Full flag reference

```
Options:
  --isos <ISOS>            Comma-separated ISO keys or 'all'  [default: all]
  --partitions <PARTS>     Comma-separated partition names, one per ISO
                           Accepts both 'sda3' and '/dev/sda3'
  --dry-run                Print what would be done; download/burn nothing
  --output-dir <DIR>       Where to save downloaded ISOs  [default: .]
  --log-file <FILE>        Log file path  [default: distro-burner.log]
  -h, --help               Print help
  -V, --version            Print version
```

---

## How it works

```
for each ISO:
  1. Download  →  streams to disk with progress bar
                  skips if file already exists (idempotent)
  2. Checksum  →  fetches official SHA256SUMS from the distro mirror
                  parses the hash for this filename
  3. Verify    →  hashes the local file with SHA-256
                  bails on mismatch — no ISO reaches dd if it's corrupt
  4. Prompt    →  prints the exact sudo dd command and asks Y/N
  5. Burn      →  shells out to: sudo dd if=<iso> of=/dev/<part> bs=4M status=progress oflag=sync
                  logs result (success / failure / skipped by user)
```

### Checksum format handling

Different distros publish checksums in different formats; both are supported:

| Format   | Example line                                       | Distros              |
|----------|----------------------------------------------------|----------------------|
| Standard | `abc123…  ubuntu-24.04.2-desktop-amd64.iso`        | Ubuntu, Debian, Pop, Arch |
| Fedora   | `SHA256 (Fedora-Workstation-Live-x86_64-42-1.1.iso) = abc123…` | Fedora |

Arch Linux publishes dated ISO filenames (e.g. `archlinux-2026.01.01-x86_64.iso`) in its checksum file, but hosts a stable `archlinux-x86_64.iso` symlink for download. distro-burner handles this with a substring match on `x86_64.iso`.

---

## Log format

Every run appends to the log file:

```
[2026-03-28 14:05:01] distro-burner started  isos=ubuntu,debian  dry_run=false
[2026-03-28 14:05:01] DOWNLOAD START: https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso
[2026-03-28 14:11:33] DOWNLOAD DONE: ubuntu-24.04.2-desktop-amd64.iso (5173995520 bytes)
[2026-03-28 14:11:33] SHA256 PASS: ubuntu-24.04.2-desktop-amd64.iso
[2026-03-28 14:11:35] BURN START: ubuntu-24.04.2-desktop-amd64.iso → /dev/sda3
[2026-03-28 14:16:02] BURN DONE: ubuntu-24.04.2-desktop-amd64.iso → /dev/sda3
[2026-03-28 14:16:02] distro-burner finished successfully
```

---

## Safety notes

- **dd is destructive.** Always double-check `--partitions` before confirming the Y/N prompt. There is no undo.
- distro-burner does not call `sudo` at startup — only at the burn step, and only after your explicit confirmation.
- Partition names are validated against `[a-zA-Z0-9/_-]` before being passed to the shell.
- SHA256 verification is mandatory for every ISO before burn; this cannot be skipped except via `--dry-run`.

---

## License

MIT — © Thomas Lane

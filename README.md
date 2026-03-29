# distro-burner · `python-wrapper` branch

> **Two branches, one CLI surface:**
> | Branch | Implementation | When to use |
> |---|---|---|
> | [`main`](../../tree/main) | Pure Rust (no Python required) | Production / fast builds |
> | **`python-wrapper`** ← you are here | Python logic + Rust launcher | Easier to hack / extend |

---

A CLI that downloads five Linux ISOs from official mirrors, verifies their SHA256 checksums, and burns each one to a target partition via `sudo dd`.

```
distro-burner --isos ubuntu,debian --partitions sda3,sda5
```

---

## How this branch works

```
distro-burner  (Rust binary — thin launcher)
       │
       └─► python3 distro_burner.py  (all real logic)
                   │
                   ├─ requests      (streaming HTTP downloads)
                   ├─ tqdm          (progress bars)
                   ├─ hashlib       (SHA-256, stdlib)
                   └─ subprocess    (sudo dd)
```

The Rust binary exists purely to give users a single executable in their `PATH`. It locates `distro_burner.py` (first next to itself, then in the repo root for `cargo run`) and `exec`s `python3` with all arguments forwarded unchanged.

All business logic — ISO catalogue, checksum parsing, verification, burn prompts, logging — lives in the Python script and is easy to edit without recompiling.

---

## Installation

### Prerequisites

- Python 3.10+ — [python.org](https://python.org)
- Rust toolchain — [rustup.rs](https://rustup.rs) *(only needed to build the launcher)*
- `sudo` access *(only needed for the burn step)*

### Install Python dependencies

```zsh
pip install -r requirements.txt
# or just: pip install requests tqdm
```

`tqdm` is optional — if missing, progress output is suppressed but everything else works.

### Build the Rust launcher

```zsh
git clone https://github.com/tom2025b/distro-burner
cd distro-burner
git checkout python-wrapper
cargo build --release
```

The binary at `target/release/distro-burner` looks for `distro_burner.py` next to itself, so copy both when installing:

```zsh
cp target/release/distro-burner ~/bin/
cp distro_burner.py ~/bin/
```

Or just run from the repo root with `cargo run --`:

```zsh
cargo run -- --isos ubuntu --dry-run
```

### Run the Python script directly (no Rust required)

```zsh
python3 distro_burner.py --isos all --dry-run
```

---

## Usage

```zsh
# Download + verify + burn two ISOs
distro-burner --isos ubuntu,debian --partitions sda3,sda5

# Burn all five ISOs
distro-burner --isos all --partitions sda3,sda5,sda7,sda9,sda11

# Download + verify only (no burn)
distro-burner --isos fedora,arch --output-dir ~/isos

# Dry run — print commands, touch nothing
distro-burner --isos all --partitions sda3,sda5,sda7,sda9,sda11 --dry-run
```

### Full flag reference

```
  --isos KEYS        Comma-separated keys or 'all'          (default: all)
  --partitions PARTS Comma-separated partitions, one per ISO
                     Accepts 'sda3' or '/dev/sda3'
  --dry-run          Print what would happen; download/burn nothing
  --output-dir DIR   Where to save ISOs                     (default: .)
  --log-file FILE    Log file path                          (default: distro-burner.log)
  -h, --help
```

---

## Supported ISOs

| Key      | Distro                              | Mirror                        |
|----------|-------------------------------------|-------------------------------|
| `ubuntu` | Ubuntu 24.04.2 LTS Desktop (amd64)  | releases.ubuntu.com           |
| `fedora` | Fedora Workstation 42 Live (x86_64) | dl.fedoraproject.org          |
| `popos`  | Pop!\_OS 22.04 LTS Intel/AMD        | iso.pop-os.org                |
| `debian` | Debian 12.9 Netinstall (amd64)      | cdimage.debian.org            |
| `arch`   | Arch Linux latest (x86\_64)         | mirror.rackspace.com          |

> Point-release numbers drift. If a URL 404s, update the version string in the `ISO_CATALOGUE` list near the top of `distro_burner.py`.

---

## How it works

```
for each ISO:
  1. Download  →  streams to disk in 64 KiB chunks with tqdm bar
                  skips existing non-empty files (idempotent)
  2. Checksum  →  fetches the distro's official SHA256SUMS file
                  parses the hash for this filename
  3. Verify    →  hashes the local file with hashlib.sha256()
                  sys.exit() on mismatch; deletes corrupt file
  4. Prompt    →  prints exact sudo dd command, asks Y/N
  5. Burn      →  subprocess.run(["sudo", "dd", ...])
                  logs result to file
```

### Checksum formats

| Format   | Example                                               | Distros              |
|----------|-------------------------------------------------------|----------------------|
| Standard | `abc123…  ubuntu-24.04.2-desktop-amd64.iso`           | Ubuntu, Debian, Pop, Arch |
| Fedora   | `SHA256 (Fedora-Workstation-Live-x86_64-42-1.1.iso) = abc123…` | Fedora |

---

## Log format

```
[2026-03-28 14:05:01] INFO distro-burner started  isos=ubuntu  dry_run=False
[2026-03-28 14:05:02] INFO DOWNLOAD START: https://releases.ubuntu.com/…
[2026-03-28 14:11:33] INFO DOWNLOAD DONE: ubuntu-24.04.2-desktop-amd64.iso (5173995520 bytes)
[2026-03-28 14:11:33] INFO SHA256 PASS: ubuntu-24.04.2-desktop-amd64.iso
[2026-03-28 14:11:35] INFO BURN START: ubuntu-24.04.2-desktop-amd64.iso → /dev/sda3
[2026-03-28 14:16:02] INFO BURN DONE: ubuntu-24.04.2-desktop-amd64.iso → /dev/sda3
```

---

## Extending / hacking

Because all logic is in plain Python, customisation is straightforward:

- **Add a new distro** — append an entry to `ISO_CATALOGUE` in `distro_burner.py`
- **Change checksum format** — add a branch in `fetch_expected_hash()`
- **Add GPG verification** — call `gpg --verify` after the SHA256 check
- **Parallel downloads** — swap the `for` loop for `concurrent.futures.ThreadPoolExecutor`

No recompile needed for any of the above.

---

## Safety notes

- `dd` is destructive. Always verify `--partitions` before confirming the Y/N prompt.
- Partition names are validated against `[a-zA-Z0-9/_-]` before reaching `subprocess`.
- SHA256 verification cannot be skipped (except with `--dry-run`).
- A corrupt download is deleted automatically so a re-run fetches fresh.

---

## License

MIT — © Thomas Lane

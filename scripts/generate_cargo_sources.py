#!/usr/bin/env python3
"""
Zero-dependency Flatpak cargo-sources generator for Pop! Profile Manager.
Parses Cargo.lock and generates cargo-sources.json compliant with flatpak-builder.
"""

import argparse
import json
import os
import sys
import tomllib


def generate_cargo_sources(cargo_lock_path: str, output_path: str):
    if not os.path.isfile(cargo_lock_path):
        print(f"Error: {cargo_lock_path} not found.", file=sys.stderr)
        sys.exit(1)

    with open(cargo_lock_path, "rb") as f:
        lock_data = tomllib.load(f)

    packages = lock_data.get("package", [])
    sources = []

    for pkg in packages:
        name = pkg.get("name")
        version = pkg.get("version")
        checksum = pkg.get("checksum")
        source = pkg.get("source", "")

        # Only process crates.io registry dependencies
        if not checksum or not source.startswith("registry+"):
            continue

        crate_dest = f"cargo/vendor/{name}-{version}"

        # 1. Download archive source
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "dest": crate_dest,
        })

        # 2. Inline .cargo-checksum.json for cargo vendor verification
        sources.append({
            "type": "inline",
            "contents": json.dumps({"package": checksum, "files": {}}),
            "dest-filename": ".cargo-checksum.json",
            "dest": crate_dest,
        })

    # 3. Cargo configuration to redirect crates.io to vendored directory
    cargo_config = (
        "[source.crates-io]\n"
        'replace-with = "vendored-sources"\n\n'
        "[source.vendored-sources]\n"
        'directory = "cargo/vendor"\n'
    )
    sources.append({
        "type": "inline",
        "contents": cargo_config,
        "dest": ".cargo",
        "dest-filename": "config.toml",
    })

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(sources, f, indent=2)

    print(f"[+] Generated {output_path} with {len(packages)} packages (total {len(sources)} source entries).")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate cargo-sources.json for Flatpak")
    parser.add_argument("cargo_lock", nargs="?", default="Cargo.lock", help="Path to Cargo.lock")
    parser.add_argument("-o", "--output", default="cargo-sources.json", help="Output JSON path")
    args = parser.parse_args()

    generate_cargo_sources(args.cargo_lock, args.output)

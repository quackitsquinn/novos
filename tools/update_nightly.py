#!/usr/bin/env python3.13
# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "tomlkit"
# ]
# ///
# Updates the workspace's rust-toolchain.toml to be pinned to the latest nightly version of Rust.
import subprocess
import tomlkit
from datetime import date as Date;

def update_nightly() -> Date:
    subprocess.check_call(["rustup", "update", "nightly"])
    output = subprocess.check_output(["rustc", "+nightly", "--version"]).decode("utf-8").strip()
    date = output.replace(")", "").split(" ")[3]
    print(f"Updating nightly version to {date}")
    return Date.fromisoformat(date)

with open("rust-toolchain.toml", "r+") as f:
    text = f.read()
    toml = tomlkit.loads(text)
    new_version = update_nightly()
    version_string = f"nightly-{new_version.isoformat()}"
    if toml["toolchain"]["channel"] != version_string:
        print(f"Updating rust-toolchain.toml to {version_string}")
        toml["toolchain"]["channel"] = version_string
        f.seek(0)
        f.write(toml.as_string())
        f.truncate()


#!/usr/bin/env python3
import subprocess
import sys
import os
import shutil
import tomllib


DOCS = """
SmartCheck is a `cargo check` wrapper to force specific targets for individual crates in a workspace.
This is useful for cross-compilation scenarios where different crates may need to be checked
with different target architectures or configurations.

This was made to work as a rust-analyzer check override, due to this, the tool currently only supports
json-diagnostic-rendered-ansi output.

== Configuration for specific crate targets ==

You can override the check target for individual crates by specifying them in the workspace root's Cargo.toml 
under a [package.metadata.smartcheck] table where the key is the crate and the value is the target.

e.g.

[workspace.metadata.smartcheck]
kernel = "x86_64-unknown-none"
"""

if len(sys.argv) >= 2 and sys.argv[1] == "help":
    print(DOCS)
    sys.exit(0)


TOML = tomllib.loads(open("Cargo.toml", "r").read())
SHOULD_LOG = os.environ.get("SMARTCHECK_LOG")

LOG = open(".smartcheck.log", "a") if SHOULD_LOG else None


# Logging utility. Only logs if SMARTCHECK_LOG is set.
def log(*args):
    if SHOULD_LOG:
        print(*args, file=LOG)
        LOG.flush()

# Error utility. Prints to stderr and log, then exits.
def err(*args):
    print("Error:", *args, file=sys.stderr)
    if LOG:
        print("Error:", *args, file=LOG)
        LOG.close()
    exit(-1)


if "workspace" not in TOML or "members" not in TOML["workspace"]:
    err("Not a workspace!")

members = TOML["workspace"]["members"]

CARGO = shutil.which("cargo")
if not CARGO:
    err("cargo not found in PATH")
    sys.exit(1)

def cargo_check(*args, **kwargs):
    check = "check"
    if os.environ.get("CHECK_COMMAND"):
        check = os.environ["CHECK_COMMAND"]

    return subprocess.run([CARGO, check, *args], *kwargs)


def workspace_check():
    return cargo_check(
        "--workspace",
        "--message-format=json-diagnostic-rendered-ansi",
        "--all-targets",
    )


def package_check(member, target=None):
    args = [
        "-p",
        member,
        "--message-format=json-diagnostic-rendered-ansi",
    ]
    if target:
        args.extend(["--target", target])
    return cargo_check(*args)


if not TOML.get("workspace", {}).get("metadata", {}).get("smartcheck"):
    log("No smartcheck overrides found, falling back to workspace check")
    sys.exit(workspace_check().returncode)


overrides = TOML["workspace"]["metadata"]["smartcheck"]

for member in members:
    member = member.split("/")[-1]
    target = overrides.get(member)
    log(f"Checking {member} with target {target}")
    ck = package_check(member, target=target)
    if ck.returncode != 0:
        err(f"Error checking {member}")



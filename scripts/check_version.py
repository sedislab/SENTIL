"""Every package declares the same version as the workspace.

The version is written into roughly seventy tracked files, and each ecosystem keeps
it in its own manifest, so a bump that misses one produces a Julia package that
fetches a release tarball that does not exist or a CMake config that reports the old
version to `find_package`. The workspace `Cargo.toml` is the source of truth.

Run as `python scripts/check_version.py` from anywhere.
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, ".."))

MANIFESTS = [
    ("Cargo.toml", r'^version = "([^"]+)"'),
    ("CITATION.cff", r'^version: "([^"]+)"'),
    ("sentil-py/pyproject.toml", r'^version = "([^"]+)"'),
    ("sentil-jl/Project.toml", r'^version = "([^"]+)"'),
    ("sentil-java/pom.xml", r"^  <version>([^<]+)</version>"),
    ("sentil-ros/package.xml", r"^  <version>([^<]+)</version>"),
    ("sentil-embedded/rust/Cargo.toml", r'^version = "([^"]+)"'),
    ("sentil-embedded/packaging/arduino/library.properties", r"^version=(.+)$"),
    ("sentil-cpp/CMakeLists.txt", r"^    VERSION (\S+)"),
    ("sentil-autosar-adaptive/CMakeLists.txt", r"^project\(\S+ VERSION (\S+)"),
    ("sentil-ros/CMakeLists.txt", r'^set\(SENTIL_VERSION "([^"]+)"'),
]

def declared(path, pattern):
    with open(os.path.join(ROOT, path), encoding="utf-8") as handle:
        found = re.search(pattern, handle.read(), re.MULTILINE)
    return found.group(1).strip() if found else None

def main():
    expected = declared("Cargo.toml", r'^version = "([^"]+)"')
    if expected is None:
        print("could not read the workspace version from Cargo.toml")
        sys.exit(1)

    mismatched = []
    for path, pattern in MANIFESTS:
        actual = declared(path, pattern)
        if actual is None:
            mismatched.append(f"{path}: no version found")
        elif actual != expected:
            mismatched.append(f"{path}: {actual}")

    if mismatched:
        print(f"workspace version is {expected}, but:")
        for line in mismatched:
            print(f"  {line}")
        sys.exit(1)
    print(f"all {len(MANIFESTS)} manifests declare {expected}")

if __name__ == "__main__":
    main()
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

IMAGE_REFS = ["docker/docker-compose.yml", "docker/README.md"]
IMAGE_TAG = re.compile(r"sentil-artifact:(\S+)")
ARTIFACT_REFS = ["sentil-jl/Artifacts.toml"]
ARTIFACT_VERSION = re.compile(r"/releases/download/v(\S+?)/sentil-(\S+?)-")

def declared(path, pattern):
    with open(os.path.join(ROOT, path), encoding="utf-8") as handle:
        found = re.search(pattern, handle.read(), re.MULTILINE)
    return found.group(1).strip() if found else None

def image_tags(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as handle:
        found = IMAGE_TAG.findall(handle.read())
    return [tag.removesuffix("-gpu") for tag in found if tag != "latest" and tag != "latest-gpu"]

def artifact_versions(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as handle:
        return [v for pair in ARTIFACT_VERSION.findall(handle.read()) for v in pair]

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

    tags = 0
    for path in IMAGE_REFS:
        found = image_tags(path)
        if not found:
            mismatched.append(f"{path}: no image tag found")
        for tag in found:
            tags += 1
            if tag != expected:
                mismatched.append(f"{path}: image tag {tag}")

    artifacts = 0
    for path in ARTIFACT_REFS:
        found = artifact_versions(path)
        if not found:
            mismatched.append(f"{path}: no download URL found")
        for version in found:
            artifacts += 1
            if version != expected:
                mismatched.append(f"{path}: download URL {version}")

    if mismatched:
        print(f"workspace version is {expected}, but:")
        for line in mismatched:
            print(f"  {line}")
        sys.exit(1)
    print(f"all {len(MANIFESTS)} manifests, {tags} image tags and {artifacts} artifact URLs declare {expected}")

if __name__ == "__main__":
    main()
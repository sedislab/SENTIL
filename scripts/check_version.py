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
    ("packaging/vcpkg/ports/sentil/vcpkg.json", r'^  "version": "([^"]+)"'),
    ("website/src/app/layout.tsx", r"softwareVersion: '([^']+)'"),
    ("website/src/app/(home)/page.tsx", r"^  <version>([^<]+)</version>"),
]

IMAGE_REFS = ["docker/docker-compose.yml", "docker/README.md"]
IMAGE_TAG = re.compile(r"sentil-artifact:(\S+)")
ARTIFACT_REFS = ["sentil-jl/Artifacts.toml"]
ARTIFACT_VERSION = re.compile(r"/releases/download/v(\S+?)/sentil-(\S+?)-")
DOCS = "website/content/docs"
DOC_REFS = [
    "examples/recipes/benchmark.mdx",
    "languages/apollo.mdx",
    "languages/autosar.mdx",
    "languages/c.mdx",
    "languages/cli.mdx",
    "languages/cpp.mdx",
    "languages/embedded.mdx",
    "languages/index.mdx",
    "languages/java.mdx",
    "languages/julia.mdx",
    "languages/matlab.mdx",
    "languages/python.mdx",
    "languages/ros.mdx",
    "languages/rust.mdx",
    "monitoring/concepts/pipeline.mdx",
    "start/install.mdx",
    "start/understand/comparison.mdx",
]

# The docs quote the version literally so every command stays copy-pasteable, which
# only works if a release bump reaches them. Every pattern names sentil or the SENTIL
# repository, because the same pages carry numbers that are not ours: vsomeip 3.7.3,
# CARLA 0.9.15, an ISO clause, an IP address. The last one is the version print, where
# the number stands alone in an output block under a command that names sentil.
DOC_VERSION = [
    re.compile(r"sedislab/SENTIL/(?:releases/(?:download|tag)|archive/refs/tags)/v(\d+\.\d+\.\d+)"),
    re.compile(r"sentil[a-z-]*[-_/:@ ]\^?v?(\d+\.\d+\.\d+)", re.IGNORECASE),
    re.compile(r'sentil = \{ version = "(\d+\.\d+\.\d+)"'),
    re.compile(r"sentil\w*</\w+>[^<]*<version>(\d+\.\d+\.\d+)</version>"),
    re.compile(r"sentil[^\n]*\n```\n+```text\n(\d+\.\d+\.\d+)\n```"),
]

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

def doc_pages():
    root = os.path.join(ROOT, DOCS)
    for base, _, names in os.walk(root):
        parent = os.path.relpath(base, root).replace(os.sep, "/")
        for name in sorted(names):
            if name.endswith(".mdx"):
                yield name if parent == "." else f"{parent}/{name}"

def doc_versions(page):
    with open(os.path.join(ROOT, DOCS, page), encoding="utf-8") as handle:
        text = handle.read()
    return sorted({v for pattern in DOC_VERSION for v in pattern.findall(text)})

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

    quoted = {page: doc_versions(page) for page in doc_pages()}
    for page in DOC_REFS:
        if not quoted.get(page):
            mismatched.append(f"{DOCS}/{page}: no version found")
    for page, versions in quoted.items():
        for version in versions:
            if version != expected:
                mismatched.append(f"{DOCS}/{page}: {version}")
    pages = sum(1 for versions in quoted.values() if versions)

    if mismatched:
        print(f"workspace version is {expected}, but:")
        for line in mismatched:
            print(f"  {line}")
        sys.exit(1)
    print(f"all {len(MANIFESTS)} manifests, {tags} image tags, {artifacts} artifact URLs and {pages} docs pages declare {expected}")

if __name__ == "__main__":
    main()
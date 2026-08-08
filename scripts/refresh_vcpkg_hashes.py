import re
import sys

# The portfile pins one hash for the source archive and one per platform bundle

ARCHIVE = re.compile(r"^(\s*SHA512 )[0-9a-fA-F]{128}(\s*)$")
CORE = re.compile(r"^(\s*set\(core_sha512 )[0-9a-fA-F]{128}(\)\s*)$")
DIGEST = re.compile(r"[0-9a-fA-F]{128}")

KEYS = ["source", "linux", "mac_arm", "mac_x86", "windows"]

def refresh(text, hashes):
    lines = text.split("\n")
    out = []
    filled = set()
    in_github = False
    in_chain = False
    platform = None

    for line in lines:
        bare = line.strip()

        if bare.startswith("vcpkg_from_github("):
            in_github = True
        elif in_github and bare == ")":
            in_github = False
        elif in_github:
            found = ARCHIVE.match(line)
            if found:
                line = found.group(1) + hashes["source"] + found.group(2)
                filled.add("source")

        if bare.startswith("if(") and "VCPKG_TARGET_IS_LINUX" in bare:
            in_chain = True
            platform = "linux"
        elif in_chain and bare == "endif()":
            in_chain = False
            platform = None
        elif in_chain and bare.startswith("elseif(") and "VCPKG_TARGET_IS_OSX" in bare:
            platform = "mac_arm" if "arm64" in bare else "mac_x86"
        elif in_chain and bare == "else()":
            platform = "windows"

        if platform:
            found = CORE.match(line)
            if found:
                line = found.group(1) + hashes[platform] + found.group(2)
                filled.add(platform)

        out.append(line)

    missing = set(KEYS) - filled
    if missing:
        raise SystemExit("no hash slot found for: " + ", ".join(sorted(missing)))
    return "\n".join(out)

def main():
    args = sys.argv[1:]
    if len(args) != len(KEYS) + 1:
        raise SystemExit("usage: refresh_vcpkg_hashes.py PORTFILE " + " ".join(k.upper() for k in KEYS))

    path, given = args[0], args[1:]
    bad = [k for k, digest in zip(KEYS, given) if not DIGEST.fullmatch(digest)]
    if bad:
        raise SystemExit("not a sha512 digest: " + ", ".join(bad))

    with open(path, encoding="utf-8") as handle:
        text = handle.read()

    # a portfile refresh() rejects must survive intact, so build the new text before opening for write
    updated = refresh(text, dict(zip(KEYS, given)))
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(updated)

if __name__ == "__main__":
    main()
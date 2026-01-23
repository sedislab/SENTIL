#!/usr/bin/env python3
import glob
import json
import os
import sys
import xml.etree.ElementTree as ET


def main(root):
    ok = True

    interfaces = set()
    for path in sorted(glob.glob(os.path.join(root, "model", "*.arxml"))):
        try:
            tree = ET.parse(path)
        except ET.ParseError as error:
            print("  {}: not well-formed: {}".format(path, error))
            ok = False
            continue
        for element in tree.iter():
            if element.tag.endswith("SERVICE-INTERFACE"):
                name = next((c.text for c in element if c.tag.endswith("SHORT-NAME")), None)
                if name in interfaces:
                    print("  duplicate service interface {}".format(name))
                    ok = False
                interfaces.add(name)

    provided = {}
    for path in sorted(glob.glob(os.path.join(root, "manifest", "*.si.manifest.json"))):
        with open(path) as handle:
            data = json.load(handle)
        for service in data.get("services", []):
            if service.get("role") != "provided":
                continue
            key = (service["service"], service["instance"])
            if key in provided:
                print("  service-instance {} provided by {} and {}".format(key, provided[key], path))
                ok = False
            provided[key] = path

    print("arxml_validate: {} interfaces, {} provided instances, {}".format(
        len(interfaces), len(provided), "ok" if ok else "FAILED"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
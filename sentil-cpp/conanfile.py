import os

from conan import ConanFile
from conan.tools.files import copy

class SentilCppConan(ConanFile):
    name = "sentil-cpp"
    version = "0.3.0"
    license = "MIT OR Apache-2.0"
    description = "C++ bindings for SENTIL, runtime verification for STL and PrSTL"
    homepage = "https://github.com/sedislab/SENTIL"
    topics = ("runtime-verification", "temporal-logic", "stl", "formal-methods")
    settings = "os", "arch", "compiler", "build_type"
    no_copy_source = True

    def requirements(self):
        self.requires("sentil/0.3.0", transitive_headers=True, transitive_libs=True)

    def export_sources(self):
        copy(self, "*", os.path.join(self.recipe_folder, "include"),
             os.path.join(self.export_sources_folder, "include"))

    def package(self):
        copy(self, "*.hpp", os.path.join(self.source_folder, "include"),
             os.path.join(self.package_folder, "include"))

    def package_info(self):
        self.cpp_info.bindirs = []
        self.cpp_info.libdirs = []
        self.cpp_info.requires = ["sentil::sentil"]

    def package_id(self):
        self.info.clear()
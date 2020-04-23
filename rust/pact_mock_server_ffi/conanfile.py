import shutil
from conans import ConanFile, VisualStudioBuildEnvironment, CMake, tools

class CbindgenTestConan(ConanFile):
    name = "pact_mock_server_ffi"
    version = "0.0.1"
    description = "Pact/Rust FFI bindings"
    url = "https://github.com/pact-foundation/pact-reference"
    license = "MIT"
    settings = "os", "compiler", "build_type", "arch"
    no_copy_source = True
    requires = "openssl/1.1.1d"

    def build(self):
        if self.settings.os == "Windows":
            url = ("https://github.com/pact-foundation/pact-reference/releases/download/libpact_mock_server_ffi-v%s/libpact_mock_server_ffi-linux-x86_64-%s.lib.gz"
                   % (str(self.version), str(self.version)))
            tools.download(url, "libpact_mock_server_ffi.lib.gz")
            tools.unzip("libpact_mock_server_ffi.lib.gz")
        elif self.settings.os == "Linux":
            url = ("https://github.com/pact-foundation/pact-reference/releases/download/libpact_mock_server_ffi-v%s/libpact_mock_server_ffi-linux-x86_64-%s.a.gz"
                % (str(self.version), str(self.version)))
            tools.download(url, "libpact_mock_server_ffi.a.gz")
            tools.unzip("libpact_mock_server_ffi.a.gz")
        else:
            raise Exception("Binary does not exist for these settings")
        tools.download(("https://github.com/pact-foundation/pact-reference/releases/download/libpact_mock_server_ffi-v%s/pact_mock_server_ffi.h"
                % (str(self.version))), "include/pact_mock_server_ffi.h")

    def package(self):
        self.copy("libpact_mock_server_ffi*.a", "lib", "")
        self.copy("libpact_mock_server_ffi*.lib", "lib", "")
        self.copy("*.h", "", "")

    def package_info(self):  # still very useful for package consumers
        self.cpp_info.libs = ["pact_mock_server_ffi"]
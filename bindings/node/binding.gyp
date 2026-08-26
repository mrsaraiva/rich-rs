{
  "variables": {
    "rich_ffi_include_dir%": "<(module_root_dir)/../../ffi/include",
    "rich_ffi_static%": ""
  },
  "targets": [
    {
      "target_name": "rich_native",
      "sources": [
        "src/addon.cc",
        "src/common.cc"
      ],
      "include_dirs": [
        "<!@(node -p \"require('node-addon-api').include\")",
        "<(rich_ffi_include_dir)"
      ],
      "defines": [
        "NAPI_VERSION=10",
        "NAPI_CPP_EXCEPTIONS"
      ],
      "cflags": [
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror"
      ],
      "cflags_cc": [
        "-std=c++20",
        "-fexceptions",
        "-fno-rtti"
      ],
      "cflags_cc!": [
        "-fno-exceptions",
        "-frtti"
      ],
      "conditions": [
        ["rich_ffi_static != ''", {
          "libraries": ["<(rich_ffi_static)"]
        }],
        ["rich_ffi_static == '' and OS == 'linux'", {
          "libraries": ["<(module_root_dir)/../../ffi/target/debug/librich_ffi.a"]
        }],
        ["rich_ffi_static == '' and OS == 'mac'", {
          "libraries": ["<(module_root_dir)/../../ffi/target/debug/librich_ffi.a"]
        }],
        ["rich_ffi_static == '' and OS == 'win'", {
          "libraries": ["<(module_root_dir)/../../ffi/target/debug/rich_ffi.lib"]
        }],
        ["OS == 'linux'", {
          "libraries": ["-lpthread", "-ldl", "-lm"]
        }],
        ["OS == 'mac'", {
          "xcode_settings": {
            "CLANG_CXX_LANGUAGE_STANDARD": "c++20",
            "GCC_ENABLE_CPP_EXCEPTIONS": "YES",
            "GCC_ENABLE_CPP_RTTI": "NO",
            "WARNING_CFLAGS": ["-Wall", "-Wextra", "-Wpedantic", "-Werror"]
          }
        }],
        ["OS == 'win'", {
          "libraries": [
            "advapi32.lib",
            "bcrypt.lib",
            "kernel32.lib",
            "ntdll.lib",
            "userenv.lib",
            "ws2_32.lib"
          ],
          "msvs_settings": {
            "VCCLCompilerTool": {
              "AdditionalOptions": ["/std:c++20", "/W4", "/WX", "/GR-"],
              "ExceptionHandling": 1
            }
          }
        }]
      ]
    }
  ]
}

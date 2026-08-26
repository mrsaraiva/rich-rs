{
  "targets": [
    {
      "target_name": "foreign_owner",
      "sources": ["foreign_owner.cc"],
      "defines": ["NAPI_VERSION=10"],
      "cflags": ["-Wall", "-Wextra", "-Wpedantic", "-Werror"],
      "cflags_cc": ["-std=c++20", "-fno-exceptions", "-fno-rtti"],
      "conditions": [
        ["OS == 'win'", {
          "msvs_settings": {
            "VCCLCompilerTool": {
              "AdditionalOptions": ["/std:c++20", "/W4", "/WX", "/GR-"],
              "ExceptionHandling": 0
            }
          }
        }]
      ]
    }
  ]
}

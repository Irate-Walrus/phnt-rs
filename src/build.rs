// MIT License
//
// Copyright (c) 2024 oberrich <oberrich.llvm@proton.me>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(dead_code)]

#[cfg_attr(docsrs, doc(cfg(feature = "regenerate")))]
#[cfg(feature = "regenerate")]
pub use regen::main;

#[cfg_attr(docsrs, doc(cfg(not(feature = "regenerate"))))]
#[cfg(not(feature = "regenerate"))]
fn main() {
   println!("Using vendored bindings, build script skipped.");
}

#[cfg_attr(docsrs, doc(cfg(feature = "regenerate")))]
#[cfg(feature = "regenerate")]
mod regen {
   use regex::Regex;
   use std::collections::HashMap;
   use std::env;
   use std::fs;
   use std::path::PathBuf;

   pub struct BindgenConfig {
      pub blocklist_types: Vec<String>,
      pub raw_lines: Vec<String>,
   }

   #[rustfmt::skip]
   impl Default for BindgenConfig {
      fn default() -> Self {
         let type_overrides: HashMap<String,String> = HashMap::new()
            .into_iter()
            .map(|(k, v): (&str, &str)| (k.to_owned(), v.to_owned()))
            .collect();

         let blocklist_types = type_overrides.clone().into_keys().collect();
         let raw_lines = type_overrides
            .into_iter()
            .map(|(key, value)| format!("pub use {value} as {key};"))
            .collect();

         Self {
            blocklist_types,
            raw_lines,
         }
      }
   }

   impl BindgenConfig {
      pub fn new(blocklist_types: Vec<String>, raw_lines: Vec<String>) -> Self {
         Self { blocklist_types, raw_lines }
      }

      pub fn generate_bindings(&self) -> Result<bindgen::Bindings, bindgen::BindgenError> {
         let allowlist_regexpr = Regex::new(&format!(r"({}\\deps\\phnt-nightly\\.*\.h)|winnt\.h|ntstatus\.h", regex::escape(env!("CARGO_MANIFEST_DIR")))).unwrap();

         let blocklist_regexpr = Regex::new(&format!(r"({})", self.blocklist_types.join("|"))).unwrap();

         let mut raw_lines = vec![format!("// Generated at {}", chrono::offset::Local::now())];
         raw_lines.append(&mut self.raw_lines.clone());
         raw_lines.push(String::default());

         let mut clang_args: Vec<String> = vec!["-Iwindows.h".to_owned(), "-Iwinnt.h".to_owned(), concat!("-I", env!("CARGO_MANIFEST_DIR"), "\\deps\\phnt-nightly/").to_owned()];

         for name in ["PHNT_VERSION", "PHNT_MODE"] {
            println!("cargo:rerun-if-env-changed={}", name);
            if let Ok(str) = env::var(name) {
               clang_args.push(format!("-D{}={}", name, str));

               let value = if let Ok(_) = str.parse::<u32>() { str } else { format!("self::{}", str) };

               raw_lines.push(format!("pub const {}: u32 = {};", name, value));
            }
         }

         bindgen::builder()
            .disable_header_comment()
            .header(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ffi/wrapper.h"))
            .raw_line(raw_lines.join("\r\n").as_str())
            .clang_args(clang_args)
            .allowlist_file(allowlist_regexpr.as_str())
            .blocklist_type(blocklist_regexpr.as_str())
            .type_alias("NTSTATUS")
            .opaque_type("std::.*")
            .ctypes_prefix("cty")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .default_enum_style(bindgen::EnumVariation::Rust { non_exhaustive: true })
            .default_alias_style(::bindgen::AliasVariation::TypeAlias)
            .default_macro_constant_type(bindgen::MacroTypeVariation::Unsigned)
            .default_non_copy_union_style(bindgen::NonCopyUnionStyle::ManuallyDrop)
            .translate_enum_integer_types(true)
            .derive_copy(true)
            .derive_default(true)
            .size_t_is_usize(true)
            .allowlist_recursively(true)
            .merge_extern_blocks(true)
            .generate_inline_functions(true)
            .vtable_generation(true)
            .generate_comments(true)
            .generate_block(true)
            .detect_include_paths(true)
            .prepend_enum_name(false)
            .block_extern_crate(false)
            .fit_macro_constants(false)
            .layout_tests(false)
            .use_core()
            .emit_builtins()
            .enable_function_attribute_detection()
            .generate()
      }
   }

   fn replace_extern_with_type(bindings: &str) -> String {
      let ext_re = Regex::new(r#"extern\s+"([^"]*)"\s*\{([^}]*)\}"#).expect("Unable to compiled extern block regex");
      let ext_fn_re = Regex::new(r#"pub fn\s+([a-zA-Z_][a-zA-Z_0-9]*)\s*\((.*?)\)\s*->\s*(.*?);"#).expect("Unable to compiled function regex");
      let mut fn_types = Vec::new();

      let mut replacements = Vec::new();
      let mut src = bindings.to_owned();

      for block_match in ext_re.captures_iter(bindings) {
         let extern_type = &block_match[1];
         let inner_src = &block_match[2];
         let block_text = &block_match[0];

         // Insert #[cfg(feature = "native_fn")] above function blocks
         let annotated_block = format!("#[cfg(feature=\"native_fn\")]\n{}", block_text);
         replacements.push((block_text.to_string(), annotated_block));

         // Extract function signatures and generate function types
         for caps in ext_fn_re.captures_iter(&inner_src) {
            println!("cargo:debug=Match extern `\"{}\" {}`", extern_type, &caps[0]);
            let fn_name = &caps[1];
            let args = &caps[2];
            let return_type = &caps[3];
            fn_types.push(format!("pub type {}Fn = unsafe extern \"{}\" fn({}) -> {};", fn_name, extern_type, args, return_type));
         }
      }

      // Apply replacements to the source
      for (original, replacement) in replacements {
         src = src.replacen(&original, &replacement, 1);
      }

      // Wrap function types in a block annotated with #[cfg(feature = "fn_types")]
      if !fn_types.is_empty() {
         src += "\n#[cfg(feature=\"fn_types\")]\nmod fn_types {\n";
         for fn_type in fn_types {
            src += &format!("    {}\n", fn_type);
         }
         src += "}\n";
      }

      src
   }

   pub fn main() {
      std::process::Command::new("git").args(["submodule", "update", "--init", "--remote", "--recursive"]).output().expect("phnt/build.rs: failed to update the `phnt-nightly` submodule!");

      println!(concat!("cargo:rerun-if-changed=", env!("CARGO_MANIFEST_DIR"), "\\deps\\phnt-nightly"));

      let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("{}_bindgen.rs", std::env::var("CARGO_CFG_TARGET_ARCH").unwrap()));

      BindgenConfig::default().generate_bindings().expect("Unable to generate bindings!").write_to_file(out_path.clone()).expect("Unable to write bindings");

      let src = fs::read_to_string(&out_path).expect("Unable to read generated bindings");

      let modified_src = replace_extern_with_type(&src);

      fs::write(&out_path, modified_src).expect("Unable to write modified bindings");

      println!("cargo:info=Wrote phnt bindings to `{}`", out_path.display());
   }
}

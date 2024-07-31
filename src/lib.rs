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
#![doc = include_str!("../README.md")]
#![allow(
   warnings,
   unused,
   non_snake_case,
   non_camel_case_types,
   non_upper_case_globals
)]

/// Bindings for `phnt` (nightly) generated by `bindgen`
pub mod ffi {
   // use vendored bindings
   #[cfg_attr(docsrs, doc(cfg(not(feature = "regenerate"))))]
   #[cfg(not(feature = "regenerate"))]
   include!("ffi/generated.rs");

   // use re-generated bindings
   #[cfg_attr(docsrs, doc(cfg(feature = "regenerate")))]
   #[cfg(feature = "regenerate")]
   include!(concat!(env!("OUT_DIR"), "\\generated.rs"));
}

/// Extensions to the bindings (useful functions, macros, etc.)
pub mod ext {
   use crate::ffi::*;
   use core::{arch::asm, mem, ptr};

   #[macro_export]
   macro_rules! InitializeObjectAttributes {
      ($p:expr, $n:expr, $a:expr, $r:expr, $s:expr) => {{
         let _o = $p;
         _o.Length = mem::size_of::<ffi::OBJECT_ATTRIBUTES>() as u32;
         _o.RootDirectory = $r;
         _o.ObjectName = $n;
         _o.Attributes = $a;
         _o.SecurityDescriptor = $s;
         _o.SecurityQualityOfService = ptr::null_mut();
      }};
   }

   #[inline]
   pub unsafe fn __readfsdword(offset: u32) -> usize {
      let out: usize;
      asm!(
          "mov {:e}, fs:[{:e}]",
          lateout(reg) out,
          in(reg) offset,
          options(nostack, pure, readonly),
      );
      out
   }

   #[inline]
   #[cfg(target_pointer_width = "64")]
   pub unsafe fn __readgsqword(offset: u32) -> usize {
      let out: usize;
      asm!(
          "mov {}, gs:[{:e}]",
          lateout(reg) out,
          in(reg) offset,
          options(nostack, pure, readonly),
      );
      out
   }

   #[inline]
   pub unsafe fn NtCurrentTeb() -> *mut TEB {
      let teb_offset = mem::offset_of!(NT_TIB, Self_) as u32;
      #[cfg(target_arch = "x86_64")]
      {
         __readgsqword(teb_offset) as _
      }
      #[cfg(target_arch = "x86")]
      {
         __readfsdword(teb_offset) as _
      }
   }

   #[cfg(test)]
   mod tests {
      use super::*;
      use windows_sys::Win32::System::Threading::GetCurrentThreadId;

      #[test]
      fn test_teb() {
         let cur_thread = unsafe { (*NtCurrentTeb()).ClientId.UniqueThread as isize };
         let cur_thread_sys = unsafe { GetCurrentThreadId() as isize };
         assert_eq!(cur_thread, cur_thread_sys);
      }
   }
}

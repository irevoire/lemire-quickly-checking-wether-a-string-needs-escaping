#![feature(portable_simd)]

use std::{intrinsics::simd::simd_gather, sync::OnceLock};

pub fn simple_needs_escaping_str(s: &str) -> bool {
    for c in s.chars() {
        if c < ' ' || c == '"' || c == '\\' {
            return true;
        }
    }
    false
}

pub fn simple_needs_escaping_bytes(s: &str) -> bool {
    for c in s.bytes() {
        if c < b' ' || c == b'"' || c == b'\\' {
            return true;
        }
    }
    false
}

pub fn branchless_needs_escaping(s: &str) -> bool {
    let mut b = false;
    for c in s.bytes() {
        b |= (c < 32) | (c == b'"') | (c == b'\\');
    }
    b
}

fn json_quotable_character() -> &'static [bool] {
    static JSON_QUOTABLE_CHARACTER: OnceLock<[bool; 256]> = OnceLock::new();
    JSON_QUOTABLE_CHARACTER
        .get_or_init(|| std::array::from_fn(|i| i < 32 || i as u8 == b'"' || i as u8 == b'\\'))
}

pub fn table_needs_escaping(s: &str) -> bool {
    let table = json_quotable_character();
    let mut needs = false;
    for c in s.bytes() {
        needs |= table[c as usize];
    }
    needs
}

pub unsafe fn simd_needs_escaping(s: &str) -> bool {
    use core::arch::aarch64::*;

    if s.len() < 16 {
        return simple_needs_escaping_bytes(s);
    }
    let view = s.as_bytes();
    let mut i = 0;
    let rnt_array: [u8; 16] = [1, 0, 34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 92, 0, 0, 0];
    let rnt = unsafe { vld1q_u8(rnt_array.as_ptr()) };
    let mut running = vdupq_n_u8(0);

    while i + 15 < view.len() {
        let word = vld1q_u8(view.as_ptr().add(i));
        running = vorrq_u8(
            running,
            vceqq_u8(vqtbl1q_u8(rnt, vandq_u8(word, vdupq_n_u8(15))), word),
        );
        running = vorrq_u8(running, vcltq_u8(word, vdupq_n_u8(32)));
        i += 16;
    }

    if i < view.len() {
        let word = vld1q_u8(view.as_ptr().add(view.len() - 16));
        running = vorrq_u8(
            running,
            vceqq_u8(vqtbl1q_u8(rnt, vandq_u8(word, vdupq_n_u8(15))), word),
        );
        running = vorrq_u8(running, vcltq_u8(word, vdupq_n_u8(32)));
    }
    return vmaxvq_u32(vreinterpretq_u32_u8(running)) != 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    #[test]
    fn bug1() {
        let s = "\u{80}  \u{80}\u{80}ࠀ\" ࠀ";
        assert_eq!(simple_needs_escaping_str(s), true);
        assert_eq!(simple_needs_escaping_bytes(s), true);
        assert_eq!(branchless_needs_escaping(s), true);
        assert_eq!(table_needs_escaping(s), true);
        assert_eq!(unsafe { simd_needs_escaping(s) }, true);
    }

    #[test]
    fn bug2() {
        let s = "    \u{80}ࠀࠀ ࠀ\\";
        assert_eq!(simple_needs_escaping_str(s), true);
        assert_eq!(simple_needs_escaping_bytes(s), true);
        assert_eq!(branchless_needs_escaping(s), true);
        assert_eq!(table_needs_escaping(s), true);
        assert_eq!(unsafe { simd_needs_escaping(s) }, true);
    }

    quickcheck! {
        fn prop(s: String) -> bool {
            let expected = simple_needs_escaping_str(&s);

            simple_needs_escaping_bytes(&s) == expected &&
            branchless_needs_escaping(&s) == expected &&
            table_needs_escaping(&s) == expected &&
            unsafe { simd_needs_escaping(&s)} == expected
        }
    }
}

use lemire_quickly_checking_wether_a_string_needs_escaping::*;

fn main() {
    for (s, ret) in [("hello world", false), ("\"e", true), ("\\e", true)] {
        assert_eq!(simple_needs_escaping_str(s), ret);
        assert_eq!(simple_needs_escaping_bytes(s), ret);
        assert_eq!(branchless_needs_escaping(s), ret);
        assert_eq!(table_needs_escaping(s), ret);
    }
}

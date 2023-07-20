use pretty_assertions::assert_eq;
use regex_syntax::hir::{Class, Hir};

use yara_x_parser::ast::HexByte;

use super::compiler::{Compiler, Location, RegexpAtom};
use crate::compiler::{hex_byte_to_class, Atom};
use crate::re::instr::{
    epsilon_closure, BckCodeLoc, EpsilonClosureState, FwdCodeLoc,
};

macro_rules! assert_re_code {
    ($re:expr, $fwd:expr, $bck:expr, $atoms:expr, $fwd_closure:expr, $bck_closure:expr) => {{
        let mut parser = regex_syntax::ParserBuilder::new()
            .utf8(false)
            .unicode(false)
            .build();

        let (fwd_code, bck_code, atoms) =
            Compiler::new().compile(&parser.parse($re).unwrap());

        assert_eq!(fwd_code.to_string(), $fwd);
        assert_eq!(bck_code.to_string(), $bck);
        assert_eq!(atoms, $atoms);

        let mut fwd_closure = vec![];
        let mut cache = EpsilonClosureState::new();

        epsilon_closure(
            fwd_code.as_ref(),
            FwdCodeLoc::try_from(0_usize).unwrap(),
            None,
            None,
            &mut cache,
            &mut fwd_closure,
        );
        assert_eq!(fwd_closure, $fwd_closure);

        let mut bck_closure = vec![];
        epsilon_closure(
            bck_code.as_ref(),
            BckCodeLoc::try_from(0_usize).unwrap(),
            None,
            None,
            &mut cache,
            &mut bck_closure,
        );
        assert_eq!(bck_closure, $bck_closure);
    }};
}

#[test]
fn re_code_1() {
    assert_re_code!(
        "(?s)abcd",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: LIT 0x64
00004: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x64
00001: LIT 0x63
00002: LIT 0x62
00003: LIT 0x61
00004: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::exact(vec![0x61, 0x62, 0x63, 0x64]),
            code_loc: Location { fwd: 0x00, bck: 0x04, bck_seq_id: 0 }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_2() {
    assert_re_code!(
        "(?s)abcde",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: LIT 0x64
00004: LIT 0x65
00005: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x65
00001: LIT 0x64
00002: LIT 0x63
00003: LIT 0x62
00004: LIT 0x61
00005: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x64]),
            code_loc: Location { fwd: 0x00, bck: 0x05, bck_seq_id: 0 }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_3() {
    assert_re_code!(
        "(?s)abc.",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: ANY_BYTE
00005: MATCH
"#,
        // Backward code
        r#"
00000: ANY_BYTE
00002: LIT 0x63
00003: LIT 0x62
00004: LIT 0x61
00005: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x61, 0x62, 0x63]),
            code_loc: Location { fwd: 0x00, bck: 0x05, bck_seq_id: 0 }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_4() {
    assert_re_code!(
        r"(?s)a\xAAcde123",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0xaa
00003: LIT 0x63
00004: LIT 0x64
00005: LIT 0x65
00006: LIT 0x31
00007: LIT 0x32
00008: LIT 0x33
00009: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x33
00001: LIT 0x32
00002: LIT 0x31
00003: LIT 0x65
00004: LIT 0x64
00005: LIT 0x63
00006: LIT 0xaa
00008: LIT 0x61
00009: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x65, 0x31, 0x32, 0x33]),
            code_loc: Location { fwd: 0x05, bck: 0x04, bck_seq_id: 0 }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_5() {
    assert_re_code!(
        "(?s)ab|cd|ef",
        // Forward code
        r#"
00000: SPLIT_N 00009 0000f 00015
00009: LIT 0x61
0000a: LIT 0x62
0000b: JUMP 00017
0000f: LIT 0x63
00010: LIT 0x64
00011: JUMP 00017
00015: LIT 0x65
00016: LIT 0x66
00017: MATCH
"#,
        // Backward code
        r#"
00000: SPLIT_N 00009 0000f 00015
00009: LIT 0x62
0000a: LIT 0x61
0000b: JUMP 00017
0000f: LIT 0x64
00010: LIT 0x63
00011: JUMP 00017
00015: LIT 0x66
00016: LIT 0x65
00017: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62]),
                code_loc: Location { fwd: 0x09, bck: 0x0b, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x63, 0x64]),
                code_loc: Location { fwd: 0x0f, bck: 0x11, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x65, 0x66]),
                code_loc: Location { fwd: 0x15, bck: 0x17, bck_seq_id: 0 }
            }
        ],
        // Epsilon closure starting at forward code 0.
        vec![0x09, 0x0f, 0x15],
        // Epsilon closure starting at backward code 0.
        vec![0x09, 0x0f, 0x15]
    );
}

#[test]
fn re_code_6() {
    assert_re_code!(
        "(?s)1(ab|cd|ef)",
        // Forward code
        r#"
00000: LIT 0x31
00001: SPLIT_N 0000a 00010 00016
0000a: LIT 0x61
0000b: LIT 0x62
0000c: JUMP 00018
00010: LIT 0x63
00011: LIT 0x64
00012: JUMP 00018
00016: LIT 0x65
00017: LIT 0x66
00018: MATCH
"#,
        // Backward code
        r#"
00000: SPLIT_N 00009 0000f 00015
00009: LIT 0x62
0000a: LIT 0x61
0000b: JUMP 00017
0000f: LIT 0x64
00010: LIT 0x63
00011: JUMP 00017
00015: LIT 0x66
00016: LIT 0x65
00017: LIT 0x31
00018: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::exact(vec![0x31, 0x61, 0x62]),
                code_loc: Location { fwd: 0, bck: 0x18, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![0x31, 0x63, 0x64]),
                code_loc: Location { fwd: 0, bck: 0x18, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![0x31, 0x65, 0x66]),
                code_loc: Location { fwd: 0, bck: 0x18, bck_seq_id: 0 }
            }
        ],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0x09, 0x0f, 0x15]
    );
}

#[test]
fn re_code_7() {
    assert_re_code!(
        "(?s)a(bcd.+e)*fg",
        // Forward code
        r#"
00000: LIT 0x61
00001: SPLIT_A 00013
00005: LIT 0x62
00006: LIT 0x63
00007: LIT 0x64
00008: ANY_BYTE
0000a: SPLIT_B 00008
0000e: LIT 0x65
0000f: JUMP 00001
00013: LIT 0x66
00014: LIT 0x67
00015: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x67
00001: LIT 0x66
00002: SPLIT_A 00014
00006: LIT 0x65
00007: ANY_BYTE
00009: SPLIT_B 00007
0000d: LIT 0x64
0000e: LIT 0x63
0000f: LIT 0x62
00010: JUMP 00002
00014: LIT 0x61
00015: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![97, 98, 99, 100]),
                code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x15 },
            },
            RegexpAtom {
                atom: Atom::exact(vec![97, 102, 103]),
                code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x15 },
            },
        ],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_8() {
    assert_re_code!(
        "(?s)a(bcd.+?de)*?fg",
        // Forward code
        r#"
00000: LIT 0x61
00001: SPLIT_B 00014
00005: LIT 0x62
00006: LIT 0x63
00007: LIT 0x64
00008: ANY_BYTE
0000a: SPLIT_A 00008
0000e: LIT 0x64
0000f: LIT 0x65
00010: JUMP 00001
00014: LIT 0x66
00015: LIT 0x67
00016: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x67
00001: LIT 0x66
00002: SPLIT_B 00015
00006: LIT 0x65
00007: LIT 0x64
00008: ANY_BYTE
0000a: SPLIT_A 00008
0000e: LIT 0x64
0000f: LIT 0x63
00010: LIT 0x62
00011: JUMP 00002
00015: LIT 0x61
00016: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::exact(vec![97, 102, 103]),
                code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x16 },
            },
            RegexpAtom {
                atom: Atom::inexact(vec![97, 98, 99, 100]),
                code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x16 },
            },
        ],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_9() {
    assert_re_code!(
        "(?s)abc[0-2x-y]def",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: CLASS_RANGES [0x30-0x32] [0x78-0x79] 
0000a: LIT 0x64
0000b: LIT 0x65
0000c: LIT 0x66
0000d: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x66
00001: LIT 0x65
00002: LIT 0x64
00003: CLASS_RANGES [0x30-0x32] [0x78-0x79] 
0000a: LIT 0x63
0000b: LIT 0x62
0000c: LIT 0x61
0000d: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x30]),
                code_loc: Location { bck: 0x0d, fwd: 0, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x31]),
                code_loc: Location { bck: 0x0d, fwd: 0, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x32]),
                code_loc: Location { bck: 0x0d, fwd: 0, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x78]),
                code_loc: Location { bck: 0x0d, fwd: 0, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x79]),
                code_loc: Location { bck: 0x0d, fwd: 0, bck_seq_id: 0 }
            },
        ],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_10() {
    assert_re_code!(
        "(?s)abcd[acegikmoqsuwy024]ef",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: LIT 0x64
00004: CLASS_BITMAP 0x30 0x32 0x34 0x61 0x63 0x65 0x67 0x69 0x6b 0x6d 0x6f 0x71 0x73 0x75 0x77 0x79 
00026: LIT 0x65
00027: LIT 0x66
00028: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x66
00001: LIT 0x65
00002: CLASS_BITMAP 0x30 0x32 0x34 0x61 0x63 0x65 0x67 0x69 0x6b 0x6d 0x6f 0x71 0x73 0x75 0x77 0x79 
00024: LIT 0x64
00025: LIT 0x63
00026: LIT 0x62
00027: LIT 0x61
00028: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x64]),
            code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x28 },
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_11() {
    assert_re_code!(
        "(?s)(abc){2,}",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: SPLIT_B 00000
00007: LIT 0x61
00008: LIT 0x62
00009: LIT 0x63
0000a: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x63
00001: LIT 0x62
00002: LIT 0x61
00003: SPLIT_B 00000
00007: LIT 0x63
00008: LIT 0x62
00009: LIT 0x61
0000a: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x61]),
            code_loc: Location { fwd: 0, bck_seq_id: 0, bck: 0x0a }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_12() {
    assert_re_code!(
        "(?s)(abc123){3,}",
        // Forward code
        r#"
00000: LIT 0x61
00001: LIT 0x62
00002: LIT 0x63
00003: LIT 0x31
00004: LIT 0x32
00005: LIT 0x33
00006: LIT 0x61
00007: LIT 0x62
00008: LIT 0x63
00009: LIT 0x31
0000a: LIT 0x32
0000b: LIT 0x33
0000c: SPLIT_B 00006
00010: LIT 0x61
00011: LIT 0x62
00012: LIT 0x63
00013: LIT 0x31
00014: LIT 0x32
00015: LIT 0x33
00016: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x33
00001: LIT 0x32
00002: LIT 0x31
00003: LIT 0x63
00004: LIT 0x62
00005: LIT 0x61
00006: LIT 0x33
00007: LIT 0x32
00008: LIT 0x31
00009: LIT 0x63
0000a: LIT 0x62
0000b: LIT 0x61
0000c: SPLIT_B 00006
00010: LIT 0x33
00011: LIT 0x32
00012: LIT 0x31
00013: LIT 0x63
00014: LIT 0x62
00015: LIT 0x61
00016: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x63, 0x31, 0x32, 0x33]),
            code_loc: Location { fwd: 2, bck_seq_id: 0, bck: 0x14 }
        }],
        // Epsilon closure starting at forward code 0.
        vec![0],
        // Epsilon closure starting at backward code 0.
        vec![0]
    );
}

#[test]
fn re_code_13() {
    assert_re_code!(
        "(?s)(abcdef|ghijkl){2,}",
        // Forward code
        r#"
00000: SPLIT_N 00007 00011
00007: LIT 0x61
00008: LIT 0x62
00009: LIT 0x63
0000a: LIT 0x64
0000b: LIT 0x65
0000c: LIT 0x66
0000d: JUMP 00017
00011: LIT 0x67
00012: LIT 0x68
00013: LIT 0x69
00014: LIT 0x6a
00015: LIT 0x6b
00016: LIT 0x6c
00017: SPLIT_B 00000
0001b: SPLIT_N 00022 0002c
00022: LIT 0x61
00023: LIT 0x62
00024: LIT 0x63
00025: LIT 0x64
00026: LIT 0x65
00027: LIT 0x66
00028: JUMP 00032
0002c: LIT 0x67
0002d: LIT 0x68
0002e: LIT 0x69
0002f: LIT 0x6a
00030: LIT 0x6b
00031: LIT 0x6c
00032: MATCH
"#,
        // Backward code
        r#"
00000: SPLIT_N 00007 00011
00007: LIT 0x66
00008: LIT 0x65
00009: LIT 0x64
0000a: LIT 0x63
0000b: LIT 0x62
0000c: LIT 0x61
0000d: JUMP 00017
00011: LIT 0x6c
00012: LIT 0x6b
00013: LIT 0x6a
00014: LIT 0x69
00015: LIT 0x68
00016: LIT 0x67
00017: SPLIT_B 00000
0001b: SPLIT_N 00022 0002c
00022: LIT 0x66
00023: LIT 0x65
00024: LIT 0x64
00025: LIT 0x63
00026: LIT 0x62
00027: LIT 0x61
00028: JUMP 00032
0002c: LIT 0x6c
0002d: LIT 0x6b
0002e: LIT 0x6a
0002f: LIT 0x69
00030: LIT 0x68
00031: LIT 0x67
00032: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63, 0x64]),
                code_loc: Location { fwd: 0x7, bck_seq_id: 0, bck: 0x28 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x67, 0x68, 0x69, 0x6a]),
                code_loc: Location { fwd: 0x11, bck_seq_id: 0, bck: 0x32 }
            }
        ],
        // Epsilon closure starting at forward code 0.
        vec![0x07, 0x11],
        // Epsilon closure starting at backward code 0.
        vec![0x07, 0x11]
    );
}

#[test]
fn re_code_14() {
    assert_re_code!(
        "(?s)(abc){0,2}",
        // Forward code
        r#"
00000: SPLIT_A 0000e
00004: LIT 0x61
00005: LIT 0x62
00006: LIT 0x63
00007: SPLIT_A 0000e
0000b: LIT 0x61
0000c: LIT 0x62
0000d: LIT 0x63
0000e: MATCH
"#,
        // Backward code
        r#"
00000: SPLIT_A 0000e
00004: LIT 0x63
00005: LIT 0x62
00006: LIT 0x61
00007: SPLIT_A 0000e
0000b: LIT 0x63
0000c: LIT 0x62
0000d: LIT 0x61
0000e: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![0x61, 0x62, 0x63]),
                code_loc: Location { fwd: 0x04, bck_seq_id: 0, bck: 0x0e }
            },
            RegexpAtom {
                atom: Atom::exact(vec![]),
                code_loc: Location { fwd: 0x04, bck_seq_id: 0, bck: 0x0e }
            }
        ],
        // Epsilon closure starting at forward code 0.
        vec![0x04, 0x0e],
        // Epsilon closure starting at backward code 0.
        vec![0x04, 0x0e]
    );
}

#[test]
fn re_code_15() {
    assert_re_code!(
        "(?s)(a+|b)*",
        // Forward code
        r#"
00000: SPLIT_A 00019
00004: SPLIT_N 0000b 00014
0000b: LIT 0x61
0000c: SPLIT_B 0000b
00010: JUMP 00015
00014: LIT 0x62
00015: JUMP 00000
00019: MATCH
"#,
        // Backward code
        r#"
00000: SPLIT_A 00019
00004: SPLIT_N 0000b 00014
0000b: LIT 0x61
0000c: SPLIT_B 0000b
00010: JUMP 00015
00014: LIT 0x62
00015: JUMP 00000
00019: MATCH
"#,
        // Atoms
        vec![
            RegexpAtom {
                atom: Atom::inexact(vec![0x61]),
                code_loc: Location { fwd: 0x00, bck_seq_id: 0, bck: 0x19 }
            },
            RegexpAtom {
                atom: Atom::inexact(vec![0x62]),
                code_loc: Location { fwd: 0x00, bck_seq_id: 0, bck: 0x19 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![]),
                code_loc: Location { fwd: 0x00, bck_seq_id: 0, bck: 0x19 }
            }
        ],
        // Epsilon closure starting at forward code 0.
        vec![0x0b, 0x14, 0x19],
        // Epsilon closure starting at backward code 0.
        vec![0x0b, 0x14, 0x19]
    );
}

#[test]
fn re_code_16() {
    assert_re_code!(
        "(?s)(|abc)de",
        // Forward code
        r#"
00000: SPLIT_N 00007 0000b
00007: JUMP 0000e
0000b: LIT 0x61
0000c: LIT 0x62
0000d: LIT 0x63
0000e: LIT 0x64
0000f: LIT 0x65
00010: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x65
00001: LIT 0x64
00002: SPLIT_N 00009 0000d
00009: JUMP 00010
0000d: LIT 0x63
0000e: LIT 0x62
0000f: LIT 0x61
00010: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x64, 0x65]),
            code_loc: Location { fwd: 0x0e, bck_seq_id: 0, bck: 0x02 }
        },],
        // Epsilon closure starting at forward code 0.
        vec![0x0e, 0x0b],
        // Epsilon closure starting at backward code 0.
        vec![0x00]
    );
}

/* TODO
#[test]
fn re_code_16() {
    assert_re_code!(
        "(?s).b{2}",
        // Forward code
        r#"
00000: ANY_BYTE
00002: LIT 0x62
00003: LIT 0x62
00004: MATCH
"#,
        // Backward code
        r#"
00000: LIT 0x62
00001: LIT 0x62
00002: ANY_BYTE
00004: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x61, 0x62]),
            code_loc: Location { fwd: 0x00, bck_seq_id: 0, bck: 0x19 }
        },],
        // Epsilon closure starting at forward code 0.
        vec![0x0b, 0x14, 0x19],
        // Epsilon closure starting at backward code 0.
        vec![0x0b, 0x14, 0x19]
    );
}
*/

#[test]
fn re_code_17() {
    assert_re_code!(
        "(?s)a.(bc.){2}",
        // Forward code
        r#"
00000: LIT 0x61
00001: ANY_BYTE
00003: LIT 0x62
00004: LIT 0x63
00005: ANY_BYTE
00007: LIT 0x62
00008: LIT 0x63
00009: ANY_BYTE
0000b: MATCH
"#,
        // Backward code
        r#"
00000: ANY_BYTE
00002: LIT 0x63
00003: LIT 0x62
00004: ANY_BYTE
00006: LIT 0x63
00007: LIT 0x62
00008: ANY_BYTE
0000a: LIT 0x61
0000b: MATCH
"#,
        // Atoms
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x62, 0x63]),
            code_loc: Location { fwd: 0x03, bck_seq_id: 0, bck: 0x08 }
        },],
        // Epsilon closure starting at forward code 0.
        vec![0x00],
        // Epsilon closure starting at backward code 0.
        vec![0x00]
    );
}

#[test]
fn re_code_18() {
    let (forward_code, backward_code, atoms) =
        Compiler::new().compile(&Hir::concat(vec![
            Hir::literal([0x01, 0x02]),
            Hir::class(Class::Bytes(hex_byte_to_class(HexByte {
                value: 0x00,
                mask: 0xFC,
            }))),
            Hir::literal([0x03]),
        ]));

    assert_eq!(
        r#"
00000: LIT 0x01
00001: LIT 0x02
00002: MASKED_BYTE 0x00 0xfc
00006: LIT 0x03
00007: MATCH
"#,
        forward_code.to_string(),
    );

    assert_eq!(
        r#"
00000: LIT 0x03
00001: MASKED_BYTE 0x00 0xfc
00005: LIT 0x02
00006: LIT 0x01
00007: MATCH
"#,
        backward_code.to_string(),
    );

    assert_eq!(
        atoms,
        vec![
            RegexpAtom {
                atom: Atom::exact(vec![0x01, 0x02, 0x00, 0x03]),
                code_loc: Location { fwd: 0x00, bck: 0x07, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![0x01, 0x02, 0x01, 0x03]),
                code_loc: Location { fwd: 0x00, bck: 0x07, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![0x01, 0x02, 0x02, 0x03]),
                code_loc: Location { fwd: 0x00, bck: 0x07, bck_seq_id: 0 }
            },
            RegexpAtom {
                atom: Atom::exact(vec![0x01, 0x02, 0x03, 0x03]),
                code_loc: Location { fwd: 0x00, bck: 0x07, bck_seq_id: 0 }
            },
        ]
    );
}

#[test]
fn re_code_19() {
    let (forward_code, backward_code, atoms) =
        Compiler::new().compile(&Hir::concat(vec![
            Hir::literal([0x01, 0x02]),
            Hir::class(Class::Bytes(hex_byte_to_class(HexByte {
                value: 0x10,
                mask: 0xF0,
            }))),
            Hir::literal([0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
        ]));

    assert_eq!(
        r#"
00000: LIT 0x01
00001: LIT 0x02
00002: MASKED_BYTE 0x10 0xf0
00006: LIT 0x03
00007: LIT 0x04
00008: LIT 0x05
00009: LIT 0x06
0000a: LIT 0x07
0000b: LIT 0x08
0000c: MATCH
"#,
        forward_code.to_string(),
    );

    assert_eq!(
        r#"
00000: LIT 0x08
00001: LIT 0x07
00002: LIT 0x06
00003: LIT 0x05
00004: LIT 0x04
00005: LIT 0x03
00006: MASKED_BYTE 0x10 0xf0
0000a: LIT 0x02
0000b: LIT 0x01
0000c: MATCH
"#,
        backward_code.to_string(),
    );

    assert_eq!(
        atoms,
        vec![RegexpAtom {
            atom: Atom::inexact(vec![0x03, 0x04, 0x05, 0x06]),
            code_loc: Location { fwd: 0x06, bck: 0x06, bck_seq_id: 0 }
        },]
    );
}
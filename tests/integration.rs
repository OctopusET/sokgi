use sokgi::{Dialect, FlagSet, Warning};

fn canon(input: &str) -> String {
    FlagSet::parse(input, Dialect::C).unwrap().0.canonical()
}

fn canon_d(input: &str, d: Dialect) -> String {
    FlagSet::parse(input, d).unwrap().0.canonical()
}

fn parse(input: &str) -> (FlagSet, Vec<Warning>) {
    FlagSet::parse(input, Dialect::C).unwrap()
}

#[test]
fn o_last_wins() {
    assert_eq!(canon("-O3 -O2"), "-O2");
    assert_eq!(canon("-O0 -O1 -O3"), "-O3");
    assert_eq!(canon("-Os -O2"), "-O2");
}

#[test]
fn o_single() {
    assert_eq!(canon("-O2"), "-O2");
    assert_eq!(canon("-Og"), "-Og");
    assert_eq!(canon("-Oz"), "-Oz");
}

#[test]
fn o_bare_is_o1() {
    assert_eq!(canon("-O"), "-O1");
}

#[test]
fn o_bare_does_not_warn() {
    let (_, warns) = FlagSet::parse("-O", Dialect::C).unwrap();
    assert!(warns.is_empty());
}

#[test]
fn g_last_wins() {
    assert_eq!(canon("-g3 -g0"), "-g0");
    assert_eq!(canon("-g -g2"), "-g2");
}

#[test]
fn g_level_and_format_independent() {
    let c = canon("-g3 -gdwarf-5");
    assert!(c.contains("-g3") && c.contains("-gdwarf-5"), "got {c}");
}

#[test]
fn g_level_and_format_order_insensitive() {
    assert_eq!(canon("-g3 -gdwarf-5"), canon("-gdwarf-5 -g3"));
}

#[test]
fn g_level_last_wins_per_axis() {
    let c = canon("-g2 -g3");
    assert!(c.contains("-g3") && !c.contains("-g2"));
}

#[test]
fn g_format_last_wins_per_axis() {
    let c = canon("-gdwarf-4 -gdwarf-5");
    assert!(c.contains("-gdwarf-5") && !c.contains("-gdwarf-4"));
}

#[test]
fn g0_drops_format() {
    assert_eq!(canon("-gdwarf-5 -g0"), "-g0");
}

#[test]
fn gdwarf_dash_preserved() {
    assert_eq!(canon("-gdwarf-4"), "-gdwarf-4");
}

#[test]
fn std_last_wins() {
    assert_eq!(canon("-std=c99 -std=c11"), "-std=c11");
}

#[test]
fn pipe_idempotent() {
    assert_eq!(canon("-pipe -pipe -pipe"), "-pipe");
}

#[test]
fn unordered_commutative() {
    assert_eq!(canon("-g -O2"), canon("-O2 -g"));
    assert_eq!(canon("-O2 -std=c11 -g"), canon("-g -std=c11 -O2"));
}

#[test]
fn canonical_order_pipe_std_o_g_march() {
    assert_eq!(
        canon("-march=armv8-a -O2 -g -std=c11 -pipe"),
        "-pipe -std=c11 -O2 -g -march=armv8-a"
    );
}

#[test]
fn d_last_wins_warns() {
    let (set, warns) = parse("-DFOO=1 -DFOO=2");
    assert_eq!(set.canonical(), "-DFOO=2");
    assert_eq!(warns, vec![Warning::ConflictingDefine("FOO".into())]);
}

#[test]
fn d_same_value_no_warning() {
    let (set, warns) = FlagSet::parse("-DFOO=1 -DFOO=1", Dialect::C).unwrap();
    assert_eq!(set.canonical(), "-DFOO=1");
    assert!(warns.is_empty());
}

#[test]
fn d_sorted() {
    assert_eq!(canon("-DBETA -DALPHA=1"), "-DALPHA=1 -DBETA");
}

#[test]
fn d_two_token() {
    assert_eq!(canon("-D FOO=1"), "-DFOO=1");
}

#[test]
fn u_overrides_d() {
    let (set, _) = parse("-DFOO=1 -UFOO");
    assert_eq!(set.canonical(), "-UFOO");
}

#[test]
fn u_overrides_d_either_order() {
    assert_eq!(canon("-UFOO -DFOO=1"), "-UFOO");
    assert_eq!(canon("-DFOO=1 -UFOO"), "-UFOO");
}

#[test]
fn i_order_preserved() {
    assert_eq!(canon("-I/a -I/b"), "-I/a -I/b");
    assert_eq!(canon("-I/b -I/a"), "-I/b -I/a");
}

#[test]
fn i_two_token() {
    assert_eq!(canon("-I /a -I /b"), "-I/a -I/b");
}

#[test]
fn unordered_floats_up() {
    assert_eq!(canon("-I/a -O2 -I/b"), "-O2 -I/a -I/b");
    assert_eq!(canon("-O2 -I/a -I/b"), "-O2 -I/a -I/b");
}

#[test]
fn path_not_normalized() {
    assert_eq!(canon("-I/a/../b"), "-I/a/../b");
    assert_eq!(canon("-I./x"), "-I./x");
}

#[test]
fn march_name_lowercased_suffix_verbatim() {
    assert_eq!(canon("-march=Cortex-A76+crc"), "-march=cortex-a76+crc");
}

#[test]
fn mcpu_feature_suffix_preserved_order() {
    assert_eq!(
        canon("-mcpu=cortex-a55+crc+nocrypto+crypto"),
        "-mcpu=cortex-a55+crc+nocrypto+crypto"
    );
}

#[test]
fn march_overrides_mcpu() {
    let (set, warns) = parse("-march=armv8-a -mcpu=cortex-a76");
    assert_eq!(set.canonical(), "-march=armv8-a");
    assert_eq!(warns.len(), 1);
    assert!(matches!(warns[0], Warning::DroppedByOverride { .. }));
}

#[test]
fn march_last_wins() {
    assert_eq!(canon("-march=armv7-a -march=armv8-a"), "-march=armv8-a");
}

#[test]
fn empty_machine_spec_is_raw() {
    let (set, warns) = FlagSet::parse("-march= -mcpu=", Dialect::C).unwrap();
    assert_eq!(set.canonical(), "-march= -mcpu=");
    assert_eq!(warns.len(), 2);
    assert!(matches!(warns[0], Warning::UnknownFlag(_)));
}

#[test]
fn f_last_wins_same_name() {
    assert_eq!(canon("-fwrapv -fno-wrapv"), "-fno-wrapv");
    assert_eq!(canon("-fno-wrapv -fwrapv"), "-fwrapv");
    assert_eq!(canon("-fwrapv -fno-wrapv -fwrapv"), "-fwrapv");
}

#[test]
fn f_different_names_independent() {
    assert_eq!(canon("-fwrapv -ftrapv"), "-ftrapv -fwrapv");
}

#[test]
fn w_last_wins_same_name() {
    assert_eq!(canon("-Wunused -Wno-unused"), "-Wno-unused");
    assert_eq!(canon("-Wno-unused -Wunused"), "-Wunused");
}

#[test]
fn w_different_names_coexist() {
    assert_eq!(canon("-Wall -Wno-unused -Wunused"), "-Wall -Wunused");
}

#[test]
fn wl_preserved() {
    assert_eq!(
        canon("-Wl,--as-needed -lfoo -Wl,--no-as-needed"),
        "-Wl,--as-needed -lfoo -Wl,--no-as-needed"
    );
}

#[test]
fn xlinker_two_token() {
    assert_eq!(canon("-Xlinker --as-needed"), "-Xlinker --as-needed");
}

#[test]
fn xlinker_round_trip() {
    let c1 = canon("-Xlinker --as-needed");
    let (set2, _) = FlagSet::parse(&c1, Dialect::C).unwrap();
    assert_eq!(c1, set2.canonical());
}

#[test]
fn ld_shallow_passthrough() {
    assert_eq!(
        canon_d("-L/x -lfoo -Wl,-rpath,/y", Dialect::Ld),
        "-L/x -lfoo -Wl,-rpath,/y"
    );
}

#[test]
fn ld_whitespace_normalized() {
    assert_eq!(canon_d("  -L/x   -lfoo  ", Dialect::Ld), "-L/x -lfoo");
}

#[test]
fn f_prefix_is_toggle_not_unknown() {
    let (set, warns) = parse("-fnonexistent-gcc-14-flag");
    assert_eq!(set.canonical(), "-fnonexistent-gcc-14-flag");
    assert!(warns.is_empty());
}

#[test]
fn long_opt_is_unknown() {
    let (set, warns) = parse("--some-long-opt");
    assert_eq!(set.canonical(), "--some-long-opt");
    assert_eq!(warns, vec![Warning::UnknownFlag("--some-long-opt".into())]);
}

#[test]
fn positional_source_files_preserved() {
    let (set, _) = parse("main.c foo.o");
    assert_eq!(set.canonical(), "main.c foo.o");
}

#[test]
fn unterminated_quote_errors() {
    let err = FlagSet::parse("-DFOO=\"hello", Dialect::C).unwrap_err();
    assert_eq!(err, sokgi::Error::UnterminatedQuote);
}

#[test]
fn nul_byte_rejected() {
    let err = FlagSet::parse("foo\0bar", Dialect::C).unwrap_err();
    assert_eq!(err, sokgi::Error::NulByte);
}

#[test]
fn empty_input() {
    assert_eq!(canon(""), "");
    assert_eq!(canon("   "), "");
}

#[test]
fn shell_quote_value_with_spaces() {
    let (set, _) = FlagSet::parse(r#"-DFOO="hello world""#, Dialect::C).unwrap();
    let c1 = set.canonical();
    let (set2, _) = FlagSet::parse(&c1, Dialect::C).unwrap();
    assert_eq!(c1, set2.canonical());
}

#[test]
fn shell_quote_path_with_spaces() {
    let (set, _) = FlagSet::parse(r#"-I"/path with spaces""#, Dialect::C).unwrap();
    let c1 = set.canonical();
    let (set2, _) = FlagSet::parse(&c1, Dialect::C).unwrap();
    assert_eq!(c1, set2.canonical());
}

#[test]
fn shell_quote_skip_when_safe() {
    assert_eq!(canon("-march=armv8-a+crc"), "-march=armv8-a+crc");
    assert_eq!(canon("-Wl,--as-needed"), "-Wl,--as-needed");
    assert_eq!(canon("-I/usr/local/include"), "-I/usr/local/include");
    assert_eq!(canon("-DFOO=bar"), "-DFOO=bar");
}

#[test]
fn realistic_gentoo_cflags() {
    let input = "-march=native -O2 -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2";
    let (set, warns) = parse(input);
    assert_eq!(
        set.canonical(),
        "-pipe -O2 -march=native -D_FORTIFY_SOURCE=2 -fstack-protector-strong"
    );
    assert!(warns.is_empty());
}

#[test]
fn same_hash_reordered() {
    let a = canon("-O2 -g -I/a -I/b -DFOO=1");
    let b = canon("-DFOO=1 -g -O2 -I/a -I/b");
    assert_eq!(a, b);
}

#[test]
fn canonical_is_idempotent() {
    let cases = [
        "-O3 -O2 -g -DFOO=1",
        "-march=Cortex-A76+crc -mcpu=cortex-a55",
        r#"-DFOO="hello world" -I/path -Wl,--as-needed -lfoo"#,
        "-DFOO=1 -UFOO -O -g3 -gdwarf-5",
        "main.c foo.o -O2 -lm",
    ];
    for input in cases {
        let c1 = canon(input);
        let c2 = canon(&c1);
        assert_eq!(c1, c2, "not idempotent for {input:?}");
    }
}

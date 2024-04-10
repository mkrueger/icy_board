/*
#[cfg(test)]
mod interpreter_tests {
    use std::path::PathBuf;

    use crate::{
        icy_board::data::IcyBoardData,
        vm::{run, ExecutionContext, HangupType, MemoryIO, PCBoardIO, TerminalTarget},
        parser::parse_program,
        Res,
    };

    struct TestContext {
        output: String,
    }
    impl TestContext {
        pub fn new() -> Self {
            Self {
                output: String::new(),
            }
        }
    }

    impl ExecutionContext for TestContext {
        fn has_sysop(&self) -> bool {
            false
        }

        fn gotoxy(&mut self, _terminal_target: TerminalTarget, _x: i32, _y: i32) -> Res<()> {
            Ok(())
        }
        fn get_char(&mut self) -> Res<Option<char>> {
            todo!()
        }

        fn print(&mut self, _terminal_target: TerminalTarget, str: &str) -> Res<()> {
            self.output.push_str(str);
            Ok(())
        }

        fn write_raw(&mut self, _terminal_target: TerminalTarget, _data: &[u8]) -> Res<()> {
            Ok(())
        }

        fn send_to_com(&mut self, _data: &str) -> Res<()> {
            todo!()
        }

        fn read(&mut self) -> Res<String> {
            Ok(String::new())
        }
        fn inbytes(&mut self) -> i32 {
            0
        }
        fn get_caret_position(&mut self) -> (i32, i32) {
            (0, 0)
        }

        fn set_color(&mut self, _color: u8) {}
        fn hangup(&mut self, _hangup_type: HangupType) -> Res<()> {
            Ok(())
        }
    }

    #[test]
    fn test_bool() {
        check_output(
            r#"
BOOLEAN B
B = TRUE
PRINT B, ","
B = !B
PRINT B, ","
B = !B
PRINT B
"#,
            "1,0,1",
        ); // This is no error. Bools were printed as numbers
    }
    /*
        #[test]
        fn test_real() {
            check_output(r#"
    REAL v
    v = 0.653
    v = v + 10
    PRINT v
    "#, "10.653"); // This is no error. Bools were printed as numbers
        }
    */
    #[test]
    fn test_byte_overflow() {
        check_output(
            r#"
BYTE B
B = 255
PRINT B, ","
INC B
PRINT B, ","
DEC B
PRINT B
"#,
            "255,0,255",
        );
    }

    #[test]
    fn test_sbyte_overflow() {
        check_output(
            r#"
SBYTE B
B = 127
PRINT B, ","
INC B
PRINT B, ","
DEC B
PRINT B
"#,
            "127,-128,127",
        );
    }

    #[test]
    fn test_int_overflow() {
        check_output(
            r#"
INTEGER B
B = 7FFFFFFFh
PRINT B, ","
INC B
PRINT B, ","
DEC B
PRINT B
"#,
            "2147483647,-2147483648,2147483647",
        );
    }

    #[test]
    fn test_unsigned_overflow() {
        check_output(
            r#"
UNSIGNED B
B = 4294967295
PRINT B, ","
INC B
PRINT B, ","
DEC B
PRINT B
"#,
            "4294967295,0,4294967295",
        );
    }

    #[test]
    fn test_word_overflow() {
        check_output(
            r#"
WORD B
B = 65535
PRINT B, ","
INC B
PRINT B, ","
B = 0
DEC B
PRINT B
            "#,
            "65535,0,65535",
        );
    }

    #[test]
    fn test_sword_overflow() {
        check_output(
            r#"
SWORD B
B = 32767
PRINT B, ","
INC B
PRINT B, ","
DEC B
PRINT B
"#,
            "32767,-32768,32767",
        );
    }

    #[test]
    fn test_sword_overflow_case2() {
        check_output(
            r#"
WORD B
B = 0
PRINT B - 1, ","

B = B - 1
PRINT B"#,
            "-1,65535",
        );
    }

    fn check_output(prg: &str, out: &str) {
        let mut io = MemoryIO::new();
        check_output_withio(prg, &mut io, out);
    }

    fn check_output_withio(prg: &str, io: &mut dyn PCBoardIO, out: &str) {
        let mut ctx = TestContext::new();
        run(
            &mut parse_program(PathBuf::from("."), prg),
            &mut ctx,
            io,
            IcyBoardData::default(),
        )
        .unwrap();
        assert_eq!(out, ctx.output);
    }

    #[test]
    fn test_println() {
        let mut ctx = TestContext::new();
        let mut io = MemoryIO::new();

        run(
            &mut parse_program(PathBuf::from("."), "PRINTLN 1, 2, 3, \"Hello World\""),
            &mut ctx,
            &mut io,
            IcyBoardData::default(),
        )
        .unwrap();
        assert_eq!("123Hello World\n".to_string(), ctx.output);

        ctx = TestContext::new();
        run(
            &mut parse_program(PathBuf::from("."), "PRINT TRUE, \",\", $41.43, \",\", 10h"),
            &mut ctx,
            &mut io,
            IcyBoardData::default(),
        )
        .unwrap();
        assert_eq!("1,4143,16".to_string(), ctx.output);
    }

    #[test]
    fn test_func_len() {
        check_output(
            r#"
STRING STR001
STRING STR002
STR001 = "hello"
STR002 = 1234
PRINT LEN(STR001), ",", LEN(STR002), ","
STR001 = TRUE
STR002 = ""
PRINT LEN(STR001), ",", LEN(STR002)
            "#,
            "5,4,1,0",
        );
    }

    #[test]
    fn test_func_lower() {
        check_output(r#"PRINT LOWER("HELLO")"#, "hello");
    }

    #[test]
    fn test_func_upper() {
        check_output(r#"PRINT UPPER("hello")"#, "HELLO");
    }

    #[test]
    fn test_func_mid() {
        check_output(
            r#"
STRING STR001
STR001 = "Hello World"
PRINT MID(STR001, 1, 2), ",", MID(STR001, -1, 3), ",", MID(STR001, 4,1), ",", MID(STR001, 7, -3)
"#,
            "He,  H,l,",
        );
    }

    #[test]
    fn test_func_left() {
        check_output(
            r#"
STRING STR001
STR001 = "Hello World"
PRINT LEFT(STR001, 5), ",", LEFT("A", 5), ",", LEFT(STR001, -10), ",", LEFT(1, 2)
"#,
            "Hello,A    ,,1 ",
        );
    }

    #[test]
    fn test_func_right() {
        check_output(
            r#"
STRING STR001
STR001 = "Hello World"
PRINT RIGHT(STR001, 5), ",", RIGHT("A", 5), ",", RIGHT(STR001, -10), ",", RIGHT(1, 2)
"#,
            "World,    A,, 1",
        );
    }

    #[test]
    fn test_func_space() {
        check_output(
            r#"PRINT SPACE(5), ",", SPACE(0), ",", SPACE(-10), ",", SPACE(1)"#,
            "     ,,, ",
        );
    }

    #[test]
    fn test_func_chr() {
        check_output(
            r#"PRINT CHR(65), ",", CHR(0), ",", CHR(-10), ",",CHR(16705),".""#,
            "A,,, .",
        );
    }

    #[test]
    fn test_func_asc() {
        check_output(r#"PRINT ASC("A"), ",", ASC(""), ",", ASC(true)"#, "65,0,49");
    }

    #[test]
    fn test_func_instr() {
        check_output(
            r#"PRINT INSTR("ABCDEF", "CD"), ",", INSTR("ABCDEF", ""), ",", INSTR("", "ABC")"#,
            "3,0,0",
        );
    }

    #[test]
    fn test_func_ltrim() {
        check_output(
            r#"PRINT LTRIM("....FOO", "."), ",", LTRIM(".BAR", ""), ",""#,
            "FOO,.BAR,",
        );
    }

    #[test]
    fn test_func_ferr() {
        check_output(
            r#"
        PRINT LTRIM("....FOO", "."), ",", LTRIM(".BAR", ""), ","


        "#,
            "FOO,.BAR,",
        );
    }

    #[test]
    fn test_func_fput() {
        let prg = r#"
FCREATE 1, "C:\PCB\MAIN\PPE.LOG", O_RW, S_DN
FPUT 1, "Hello World"
FCLOSE 1
"#;
        let mut io = MemoryIO::new();
        let mut ctx = TestContext::new();
        run(
            &mut parse_program(PathBuf::from("."), prg),
            &mut ctx,
            &mut io,
            IcyBoardData::default(),
        )
        .unwrap();
        assert!(io.files.contains_key(r"C:\PCB\MAIN\PPE.LOG"));
        let content = io.files.get(r"C:\PCB\MAIN\PPE.LOG").unwrap();
        assert!(content == "Hello World");
    }

    #[test]
    fn test_func_fputln() {
        let prg = r#"
        FCREATE 1, "C:\PCB\MAIN\PPE.LOG", O_RW, S_DN
        FPUTLN 1, 1, 2, 3
        FPUTLN 1, "Hello"
        FPUTLN 1
        FCLOSE 1
                "#;
        let mut io = MemoryIO::new();
        let mut ctx = TestContext::new();
        run(
            &mut parse_program(PathBuf::from("."), prg),
            &mut ctx,
            &mut io,
            IcyBoardData::default(),
        )
        .unwrap();
        assert!(io.files.contains_key(r"C:\PCB\MAIN\PPE.LOG"));
    }

    #[test]
    fn test_func_fopen_ferr_fget() {
        let prg = r#"
INTEGER i
STRING s
LET i = 0
FCREATE 1, "FILE.DAT", O_RW, S_DN
FPUTLN 1, "Hello"
FPUTLN 1, "World"
FPUTLN 1, "!"
FCLOSE 1

FOPEN 1,"FILE.DAT", O_RD, S_DW
FGET 1, s
:LOOPME
IF (FERR(1)) GOTO SKIPL
   INC i
   PRINTLN "Line ", RIGHT(i, 2), ": ", s
   FGET 1, s
GOTO LOOPME
:SKIPL
FCLOSE 1
"#;
        let mut io = MemoryIO::new();

        check_output_withio(prg, &mut io, "Line  1: Hello\nLine  2: World\nLine  3: !\n");
    }

    #[test]
    fn test_procedure() {
        check_output(
            r#"
DECLARE PROCEDURE Hello(STRING who)

Hello("World")

PROCEDURE Hello(STRING who)
    PRINT "Hello ", who
ENDPROC
"#,
            "Hello World",
        ); // This is no error. Bools were printed as numbers
    }

    #[test]
    fn equaltest() {
        check_output(
            r"
STRING S
S = CHR(13)
PRINT S <> CHR(13)
",
            "0",
        );
    }
}

*/

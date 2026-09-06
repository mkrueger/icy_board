use crate::vm::tests::run_ppl;

/// Exercise the same serialized PPE boundary with optimization and runtime
/// choices, without changing the shared VM test harness.
fn run_compound_with_options(source: &str, optimize: bool, runtime: u16) -> String {
    run_compound_files_with_options(&[("compound.pps", source)], optimize, runtime)
}

fn run_compound_files_with_options(sources: &[(&str, &str)], optimize: bool, runtime: u16) -> String {
    use crate::{
        compiler::{PPECompiler, workspace::Workspace},
        executable::Executable,
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
        vm::io::DiskIO,
    };
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(runtime);
    workspace.hard_coded_files = Some(sources.iter().map(|(name, _)| PathBuf::from(name)).collect());
    let asts = sources
        .iter()
        .map(|(name, source)| parse_ast(PathBuf::from(name), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect::<Vec<_>>();
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone()).with_optimization(optimize);
    // Root-first emission must still resolve modules before checking the root.
    let programs = asts
        .iter()
        .filter(|ast| ast.module.is_none())
        .chain(asts.iter().filter(|ast| ast.module.is_some()))
        .collect::<Vec<_>>();
    compiler.compile(&programs);
    assert!(
        !errors.lock().unwrap().has_errors(),
        "{:?}",
        errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    let mut bytes = compiler.create_executable().unwrap().to_buffer().unwrap();
    let executable = Executable::from_buffer(&mut bytes, false).unwrap();
    let directory = tempfile::tempdir().unwrap();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        board.config.paths.user_file = board.root_path.join("users.toml");
        board.users.new_user(crate::icy_board::user_base::User {
            name: "SYSOP".to_string(),
            security_level: 10,
            ..Default::default()
        });
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        let sysop = state.get_board().await.users[0].clone();
        state.session.current_user = Some(sysop);
        state.session.cur_user_id = 0;
        let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
        crate::vm::run(&PathBuf::from("compound.ppe"), &executable, &mut io, &mut state).await.unwrap();
        drop(state);
        let mut output = Vec::new();
        let mut buffer = [0; 1024];
        while let Ok(size) = peer.read(&mut buffer).await {
            if size == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..size]);
        }
        String::from_utf8(output).unwrap().replace("\r\n", "\n")
    })
}

#[test]
fn compound_assignment_module_receiver_spans_do_not_collide_at_runtime() {
    let record = "MODULE One\nTYPE Item\n INTEGER SecurityLevel\nENDTYPE\nItem items[0]\nPROCEDURE Update()\n items[0].SecurityLevel += 1\n PRINT items[0].SecurityLevel\nENDPROC\nENDMODULE\n";
    let prefix = "MODULE Two\nPROCEDURE Update()\n ";
    let padding = " ".repeat(record.find("SecurityLevel +=").unwrap() - prefix.len() - "Receiver().".len());
    let object = format!(
        "{prefix}{padding}Receiver().SecurityLevel += 1\n PRINT Session.User.SecurityLevel\nENDPROC\nFUNCTION Receiver() USER\n RETURN Session.User\nENDFUNC\nENDMODULE\n"
    );
    assert_eq!(record.find("SecurityLevel +="), object.find("SecurityLevel +="));
    for optimize in [false, true] {
        for reverse in [false, true] {
            let mut sources = vec![("one.pps", record), ("two.pps", object.as_str())];
            if reverse {
                sources.reverse();
            }
            sources.push((
                "main.pps",
                "IMPORT One AS A\nIMPORT Two AS B\nSession.User.SecurityLevel = 10\nA.Update()\nB.Update()\n",
            ));
            assert_eq!(
                "111",
                run_compound_files_with_options(&sources, optimize, 400),
                "optimize={optimize}, reverse={reverse}"
            );
        }
    }
}

#[test]
fn compound_assignment_receiver_temporaries_survive_recursion_and_skipped_branches() {
    let source = r#"
INTEGER calls
USER selected = Session.User
USER other = Board.Users[0]
Session.User.SecurityLevel = 10
IF FALSE Receiver().SecurityLevel += Change(1)
IF FALSE THEN
  Receiver().SecurityLevel += Change(1)
ENDIF
Update(1)
PRINT " ", Session.User.SecurityLevel, " ", other.SecurityLevel, " ", calls
PROCEDURE Update(INTEGER depth)
  Receiver().SecurityLevel += Change(depth)
ENDPROC
FUNCTION Receiver() USER
  PRINT "G"
  calls += 1
  RETURN selected
ENDFUNC
FUNCTION Change(INTEGER depth) INTEGER
  PRINT "R"
  IF depth > 0 THEN
    selected = other
    Update(depth - 1)
  ENDIF
  RETURN 1
ENDFUNC
"#;
    for optimize in [false, true] {
        // Board.Users is a read-only snapshot. Only the outer live receiver writes.
        assert_eq!("GRGR 11 10 2", run_compound_with_options(source, optimize, 400));
    }
}

#[test]
fn compound_assignment_module_indices_lower_to_classic_runtime() {
    let module = r#"
;$LANGVERSION 350
MODULE Counter
INTEGER calls
INTEGER values(1)
PROCEDURE Update()
  values(0) = 10
  values(1) = 20
  IF FALSE values(Index()) += 100
  values(Index()) += 1
  PRINT calls, " ", values(0), " ", values(1)
ENDPROC
FUNCTION Index() INTEGER
  calls += 1
  RETURN calls - 1
ENDFUNC
ENDMODULE
"#;
    for optimize in [false, true] {
        assert_eq!(
            "1 11 20",
            run_compound_files_with_options(
                &[("counter.pps", module), ("main.pps", ";$LANGVERSION 350\nIMPORT Counter AS C\nC.Update()\n")],
                optimize,
                340,
            )
        );
    }
}

#[test]
fn compound_assignment_earlier_indices_survive_recursive_later_indices() {
    let source = r#"
INTEGER values[2, 0]
values[0, 0] = 10
values[1, 0] = 20
values[2, 0] = 30
Update(2)
PRINT values[0, 0], " ", values[1, 0], " ", values[2, 0]
PROCEDURE Update(INTEGER depth)
  values[depth, Recurse(depth)] += 1
ENDPROC
FUNCTION Recurse(INTEGER depth) INTEGER
  IF depth > 0 Update(depth - 1)
  RETURN 0
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!("11 21 31", run_compound_with_options(source, optimize, 400));
    }
}

#[test]
fn compound_assignment_accepts_enum_indices_like_plain_assignment() {
    let source = r#"
ENUM Slot
  First = 0
  Second = 1
ENDENUM
Slot selected = Slot.First
INTEGER values[1]
values[selected] = 10
values[Slot.Second] = 20
values[selected] += 1
values[Slot.Second] += 2
PRINT values[0], " ", values[1]
"#;
    for optimize in [false, true] {
        assert_eq!("11 22", run_compound_with_options(source, optimize, 400));
    }
}

#[test]
fn compound_assignment_keeps_legacy_ppe_and_unoptimized_execution() {
    let source = r#"
;$LANGVERSION 350
INTEGER calls
INTEGER values(1)
values(0) = 10
values(1) = 20
IF FALSE values(NextIndex()) += 100
values(NextIndex()) += 1
PRINT calls, " ", values(0), " ", values(1)
FUNCTION NextIndex() INTEGER
  calls += 1
  RETURN calls - 1
ENDFUNC
"#;
    for optimize in [false, true] {
        for runtime in [340, 400] {
            assert_eq!(
                "1 11 20",
                run_compound_with_options(source, optimize, runtime),
                "optimize={optimize}, runtime={runtime}"
            );
        }
    }
}

#[test]
fn compound_assignment_captures_multidimensional_indices_left_to_right() {
    let source = r#"
;$LANGVERSION 400
INTEGER slot
INTEGER values[1, 1, 1]
values[0, 0, 0] = 10
values[slot, ChangeSlot(), LastIndex()] += ChangeValue()
PRINT " ", values[0, 0, 0], " ", values[1, 0, 0]
FUNCTION ChangeSlot() INTEGER
  PRINT "I"
  slot = 1
  RETURN 0
ENDFUNC
FUNCTION LastIndex() INTEGER
  PRINT "J"
  RETURN 0
ENDFUNC
FUNCTION ChangeValue() INTEGER
  PRINT "R"
  values[0, 0, 0] = 100
  RETURN 5
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!("IJR 15 0", run_compound_with_options(source, optimize, 400));
    }
}

#[test]
fn compound_assignment_freezes_object_identity_even_if_rhs_rebinds_it() {
    for (declaration, target) in [
        ("USER selected = Session.User", "selected"),
        ("USER selected[0]\nselected[0] = Session.User", "selected[0]"),
    ] {
        let source = format!(
            r#"
{declaration}
Session.User.SecurityLevel = 10
{target}.SecurityLevel += Rebind()
PRINT Session.User.SecurityLevel, " ", {target}.SecurityLevel
FUNCTION Rebind() INTEGER
  {target} = Board.Users[0]
  RETURN 5
ENDFUNC
"#
        );
        assert_eq!(
            "15 10",
            crate::vm::tests::run_ppl_on(&source, |board| board.config.paths.user_file = board.root_path.join("users.toml"))
        );
    }
}

#[test]
fn compound_assignment_does_not_make_read_only_members_writable() {
    for source in ["Session.User.TimesOn += 1", "Surface.New(2, 2).Width += 1"] {
        let errors = crate::vm::tests::compile_errors(source);
        assert!(!errors.is_empty(), "{source}");
    }
}

#[test]
fn compound_assignment_dynamic_array_and_nested_scalar_field() {
    let source = r#"
;$LANGVERSION 400
TYPE Leaf
  INTEGER value
ENDTYPE
TYPE Branch
  Leaf items(1)
ENDTYPE
Branch roots[1]
INTEGER calls
INTEGER values[] = { 10, 20, 30, 40 }
values[NextIndex()] += 1
roots[NextIndex()].items(NextIndex()).value += 5
PRINT calls, " ", values[0], " ", values[1], " ", roots[0].items(0).value
FUNCTION NextIndex() INTEGER
  calls += 1
  RETURN 0
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!("3 11 20 5", run_compound_with_options(source, optimize, 400));
    }
}

#[test]
fn compound_assignment_captures_scalar_indices_once() {
    for brackets in [("[", "]"), ("(", ")")] {
        let source = format!(
            r#"
;$LANGVERSION 400
INTEGER calls
INTEGER values[3]
values[0] = 10
values[1] = 20
values[2] = 30
values{}NextIndex(){} += 1
PRINT calls, " ", values[0], " ", values[1], " ", values[2]
FUNCTION NextIndex() INTEGER
  calls += 1
  RETURN calls - 1
ENDFUNC
"#,
            brackets.0, brackets.1
        );
        assert_eq!("1 11 20 30", run_ppl(&source));
    }
}

#[test]
fn compound_assignment_captures_even_plain_indices_before_rhs() {
    assert_eq!(
        "IR 15 20 1",
        run_ppl(
            r#"
;$LANGVERSION 400
INTEGER slot
INTEGER values[1]
values[0] = 10
values[1] = 20
values[Index()] += Change()
PRINT " ", values[0], " ", values[1], " ", slot
FUNCTION Index() INTEGER
  PRINT "I"
  RETURN slot
ENDFUNC
FUNCTION Change() INTEGER
  PRINT "R"
  slot = 1
  values[0] = 100
  RETURN 5
ENDFUNC
"#
        )
    );
    assert_eq!(
        "15 20",
        run_ppl(
            r#"
INTEGER slot
INTEGER values[1]
values[0] = 10
values[1] = 20
values[slot] += Change()
PRINT values[0], " ", values[1]
FUNCTION Change() INTEGER
  slot = 1
  RETURN 5
ENDFUNC
"#
        )
    );
}

#[test]
fn compound_assignment_captures_nested_record_array_paths() {
    assert_eq!(
        "1234R 15 77 0",
        run_ppl(
            r#"
;$LANGVERSION 400
TYPE Leaf
  INTEGER cells[1, 1]
  INTEGER sibling
ENDTYPE
TYPE Branch
  Leaf leaves[1]
ENDTYPE
Branch roots[1]
INTEGER calls
  roots[0].leaves(0).cells(0, 0) = 10
  roots[NextIndex()].leaves(NextIndex()).cells(NextIndex(), NextIndex()) += Change()
  PRINT " ", roots[0].leaves(0).cells(0, 0), " ", roots[0].leaves(0).sibling, " ", roots[1].leaves(0).cells(0, 0)
FUNCTION NextIndex() INTEGER
  calls += 1
  PRINT calls
  RETURN 0
ENDFUNC
FUNCTION Change() INTEGER
  PRINT "R"
      roots[0].leaves(0).sibling = 77
  RETURN 5
ENDFUNC
"#
        )
    );
}

#[test]
fn compound_assignment_temporaries_survive_recursive_rhs() {
    assert_eq!(
        "11 21 31",
        run_ppl(
            r#"
INTEGER values[2]
values[0] = 10
values[1] = 20
values[2] = 30
Update(2)
PRINT values[0], " ", values[1], " ", values[2]
PROCEDURE Update(INTEGER depth)
  values[depth] += Recurse(depth)
ENDPROC
FUNCTION Recurse(INTEGER depth) INTEGER
  IF depth > 0 Update(depth - 1)
  RETURN 1
ENDFUNC
"#
        )
    );
}

#[test]
fn compound_assignment_captures_object_receiver_once() {
    assert_eq!(
        "GR 15 1",
        crate::vm::tests::run_ppl_on(
            r#"
;$LANGVERSION 400
INTEGER calls
  Session.User.SecurityLevel = 10
  Receiver().SecurityLevel += Change()
  PRINT " ", Session.User.SecurityLevel, " ", calls
FUNCTION Receiver() USER
  PRINT "G"
  calls += 1
    RETURN Session.User
ENDFUNC
FUNCTION Change() INTEGER
  PRINT "R"
    Session.User.SecurityLevel = 100
  RETURN 5
ENDFUNC
"#,
            |board| board.config.paths.user_file = board.root_path.join("users.toml"),
        )
    );
}

#[test]
fn compound_assignment_all_operators_and_simple_targets() {
    assert_eq!(
        "1 7",
        run_ppl(
            r#"
INTEGER calls
INTEGER values[0]
INTEGER simple
values[0] = 10
values[NextIndex()] += 2
values[NextIndex()] -= 1
values[NextIndex()] *= 4
values[NextIndex()] /= 2
values[NextIndex()] %= 8
values[NextIndex()] &= 7
values[NextIndex()] |= 8
simple = 10
simple += 2
simple -= 1
simple *= 4
simple /= 2
simple %= 8
simple &= 7
simple |= 8
  IF values[0] = simple PRINT simple, " ", calls
FUNCTION NextIndex() INTEGER
  calls += 1
  RETURN 0
ENDFUNC
"#
        )
    );
}

#[test]
fn a_member_call_can_stand_on_its_own_as_a_statement() {
    assert_eq!(
        "ok",
        run_ppl(
            r#"
CONFERENCE conf = Board.Conferences[0]
conf.HasAccess()
PRINT "ok"
"#
        )
    );
}

#[test]
fn test_a_field_of_a_field_keeps_what_was_assigned_to_it() {
    assert_eq!(
        "5",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
o.i.v = 5
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_nested_record_starts_out_empty() {
    assert_eq!(
        "[0][]",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
  STRING s
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
PRINT "[", o.i.v, "][", o.i.s, "]"
"#
        )
    );
}

#[test]
fn test_three_levels_deep() {
    assert_eq!(
        "7",
        run_ppl(
            r"
TYPE Level1
  INTEGER v
ENDTYPE
TYPE Level2
  Level1 one
ENDTYPE
TYPE Level3
  Level2 two
ENDTYPE
Level3 deep
deep.two.one.v = 7
PRINT deep.two.one.v
"
        )
    );
}

#[test]
fn test_a_variable_may_take_the_name_of_its_type() {
    // Names are compared without regard to case, so `C c` leaves `c` reading like the
    // type it was declared from. A member cannot start a declaration, so it is the
    // variable that is meant.
    let source = r"
TYPE C
  INTEGER v
ENDTYPE
C c
c.v = 1
PRINTLN c.v
";
    assert!(crate::vm::tests::compile_errors(source).is_empty());
    assert_eq!(crate::vm::tests::run_ppl(source), "1\n");
}

#[test]
fn test_what_a_board_object_answers_can_be_asked_again() {
    let output = crate::vm::tests::run_ppl_on(
        r"
CONFERENCE conf = Board.Conferences[0]
PRINT conf.Areas[0].Name
",
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                name: "Main".to_string(),
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea {
                        name: "General".to_string(),
                        ..Default::default()
                    },
                ]))),
                ..Default::default()
            });
        },
    );
    assert_eq!("General", output);
}

#[test]
fn test_the_outer_fields_stay_beside_the_nested_one() {
    assert_eq!(
        "1/2/x",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  INTEGER before
  Inner i
  STRING after
ENDTYPE
Outer o
o.before = 1
o.i.v = 2
o.after = "x"
PRINT o.before, "/", o.i.v, "/", o.after
"#
        )
    );
}

#[test]
fn test_a_nested_field_takes_a_compound_assignment() {
    assert_eq!(
        "12",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
o.i.v = 2
o.i.v += 10
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_whole_nested_record_is_copied() {
    assert_eq!(
        "3/9",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer a
Outer b
a.i.v = 3
b = a
b.i.v = 9
PRINT a.i.v, "/", b.i.v
"#
        )
    );
}

#[test]
fn test_a_nested_record_survives_a_routine() {
    assert_eq!(
        "4",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Go()
PROCEDURE Go()
  Outer local
  local.i.v = 4
  PRINT local.i.v
ENDPROC
"
        )
    );
}

#[test]
fn test_an_inner_record_can_be_assigned_on_its_own() {
    assert_eq!(
        "6",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
Inner free
free.v = 6
o.i = free
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_nested_var_parameter_writes_back() {
    assert_eq!(
        "11",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper.value.v = 3
Change(wrapper)
PRINT wrapper.value.v
PROCEDURE Change(VAR Outer item)
  item.value.v = 11
ENDPROC
"
        )
    );
}

#[test]
fn test_a_nested_value_parameter_does_not_write_back() {
    assert_eq!(
        "3",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper.value.v = 3
Change(wrapper)
PRINT wrapper.value.v
PROCEDURE Change(Outer item)
  item.value.v = 11
ENDPROC
"
        )
    );
}

#[test]
fn test_a_function_can_answer_a_nested_record() {
    assert_eq!(
        "13",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper = Make()
PRINT wrapper.value.v
FUNCTION Make() Outer
  Outer made
  made.value.v = 13
  RETURN made
ENDFUNC
"
        )
    );
}

#[test]
fn test_assigning_an_inner_record_copies_it() {
    assert_eq!(
        "4/9",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Inner source
Outer wrapper
source.v = 4
wrapper.value = source
wrapper.value.v = 9
PRINT source.v, "/", wrapper.value.v
"#
        )
    );
}

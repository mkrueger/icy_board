use std::{
    hint::black_box,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use icy_board_engine::{
    ast::{GotoStatement, LabelStatement},
    compiler::{PPECompiler, optimizer::optimize_statements, workspace::Workspace},
    crypt::{decode_rle, decrypt_chunks, encode_rle, encrypt_chunks},
    executable::{Executable, VariableType, VariableValue},
    icy_board::{
        IcyBoard,
        bbs::BBS,
        conferences::Conference,
        message_area::{AreaList, MessageArea},
        state::IcyBoardState,
    },
    parser::{Encoding, ErrorReporter, UserTypeRegistry, lexer::Lexer, parse_ast},
    vm::{io::DiskIO, run},
};
use icy_net::{ConnectionType, channel::ChannelConnection};

const ARITHMETIC_SOURCE: &str = r#";$LANGVERSION 400
INTEGER i, total
FOR i = 1 TO 100000
    total = total + i
NEXT
"#;

const CALL_SOURCE: &str = r#";$LANGVERSION 400
DECLARE FUNCTION AddOne(INTEGER value) INTEGER
INTEGER i, total
FOR i = 1 TO 25000
    total = AddOne(total)
NEXT

FUNCTION AddOne(INTEGER value) INTEGER
    RETURN value + 1
ENDFUNC
"#;

const STRING_SOURCE: &str = r#";$LANGVERSION 400
STRING text = STRING.Repeat("x", 4096)
INTEGER i, total
FOR i = 1 TO 1000
    total = total + text.Find("xyz")
    text = text.Trim()
NEXT
"#;

const ARRAY_RECORD_SOURCE: &str = r#";$LANGVERSION 400
TYPE Item
    INTEGER Number
    STRING Name
ENDTYPE
Item items[1000]
INTEGER i, total
FOR i = 0 TO 999
    items[i].Number = i
    items[i].Name = "item"
NEXT
FOR i = 0 TO 999
    total = total + items[i].Number + items[i].Name.Len()
NEXT
"#;

const ARRAY_FOREACH_SOURCE: &str = r#";$LANGVERSION 400
INTEGER values[9999]
INTEGER value, total
FOREACH value IN values
    total = total + value
ENDFOREACH
"#;

const OBJECT_FOREACH_SOURCE: &str = r#";$LANGVERSION 400
AREA area
INTEGER total
FOREACH area IN Board.Conferences[0].Areas
    total = total + area.Number
ENDFOREACH
"#;

fn workspace() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("benchmark.pps")]);
    workspace
}

fn routine_source() -> String {
    let mut source = String::from(";$LANGVERSION 400\nPRINT F0(1)\n");
    for index in 0..500 {
        let next = (index + 1) % 500;
        source.push_str(&format!(
            "FUNCTION F{index}(INTEGER value) INTEGER\n  INTEGER local = value\n  IF value > 0 RETURN F{next}(value - 1)\n  RETURN local\nENDFUNC\n"
        ));
    }
    source
}

fn directive_source() -> String {
    let mut source = String::from(";$LANGVERSION 400\n;$DEFINE ENABLED = 1\n");
    for index in 0..500 {
        source.push_str(&format!(
            "; ordinary comment {index}\n;$IF ENABLED\nSTRING value{index} = \"text {index}\"\n;$ELSE\nSTRING skipped{index} = \"ignored\"\n;$ENDIF\n"
        ));
    }
    source
}

fn compile(source: &str) -> Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let workspace = workspace();
    let ast = parse_ast(PathBuf::from("benchmark.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    assert!(!errors.lock().unwrap().has_errors());
    compiler.create_executable().expect("benchmark source must compile")
}

fn parse_benchmarks(criterion: &mut Criterion) {
    let workspace = workspace();
    let routine_source = routine_source();
    let mut group = criterion.benchmark_group("parse");
    for (name, source) in [
        ("arithmetic_loop", ARITHMETIC_SOURCE),
        ("function_calls", CALL_SOURCE),
        ("string_members", STRING_SOURCE),
        ("arrays_and_records", ARRAY_RECORD_SOURCE),
        ("routines_500", routine_source.as_str()),
    ] {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_function(name, |benchmark| {
            benchmark.iter_batched(
                || (Arc::new(Mutex::new(ErrorReporter::default())), UserTypeRegistry::icy_board_registry()),
                |(errors, registry)| {
                    let ast = parse_ast(
                        PathBuf::from("benchmark.pps"),
                        errors.clone(),
                        black_box(source),
                        &registry,
                        Encoding::Utf8,
                        &workspace,
                    );
                    assert!(!errors.lock().unwrap().has_errors());
                    black_box(ast)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn lexer_benchmarks(criterion: &mut Criterion) {
    let workspace = workspace();
    let routine_source = routine_source();
    let directive_source = directive_source();
    let mut group = criterion.benchmark_group("lexer");
    for (name, source) in [("routines_500", routine_source.as_str()), ("directives_500", directive_source.as_str())] {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_function(name, |benchmark| {
            benchmark.iter(|| {
                let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                let mut lexer = Lexer::new(PathBuf::from("benchmark.pps"), &workspace, black_box(source), Encoding::Utf8, errors.clone());
                let mut token_count = 0usize;
                while let Some(token) = lexer.next_token() {
                    black_box(token);
                    token_count += 1;
                }
                assert!(!errors.lock().unwrap().has_errors());
                black_box(token_count)
            });
        });
    }
    group.finish();
}

fn compile_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("compile");
    for (name, source) in [
        ("arithmetic_loop", ARITHMETIC_SOURCE),
        ("function_calls", CALL_SOURCE),
        ("string_members", STRING_SOURCE),
        ("arrays_and_records", ARRAY_RECORD_SOURCE),
    ] {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_function(name, |benchmark| benchmark.iter(|| black_box(compile(black_box(source)))));
    }
    group.finish();
}

fn compile_from_ast_benchmarks(criterion: &mut Criterion) {
    let workspace = workspace();
    let routine_source = routine_source();
    let mut group = criterion.benchmark_group("compile_from_ast");
    for (name, source) in [
        ("arithmetic_loop", ARITHMETIC_SOURCE),
        ("function_calls", CALL_SOURCE),
        ("string_members", STRING_SOURCE),
        ("arrays_and_records", ARRAY_RECORD_SOURCE),
        ("routines_500", routine_source.as_str()),
    ] {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_function(name, |benchmark| {
            benchmark.iter_batched(
                || {
                    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                    let registry = UserTypeRegistry::icy_board_registry();
                    let ast = parse_ast(PathBuf::from("benchmark.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
                    assert!(!errors.lock().unwrap().has_errors());
                    (errors, registry, ast)
                },
                |(errors, registry, ast)| {
                    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
                    compiler.compile(&[&ast]);
                    assert!(!errors.lock().unwrap().has_errors());
                    black_box(compiler.create_executable().expect("benchmark source must compile"))
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}


fn optimizer_benchmarks(criterion: &mut Criterion) {
    const BLOCKS: usize = 500;
    let label = |index| unicase::Ascii::new(format!("L{index}"));
    let final_label = label(BLOCKS);
    let mut statements = Vec::with_capacity(BLOCKS * 2 + 2);
    statements.push(GotoStatement::create_empty_statement(final_label.clone()));
    for index in 0..BLOCKS {
        statements.push(LabelStatement::create_empty_statement(label(index)));
        statements.push(GotoStatement::create_empty_statement(final_label.clone()));
    }
    statements.push(LabelStatement::create_empty_statement(final_label));

    let mut group = criterion.benchmark_group("optimizer");
    group.throughput(Throughput::Elements(BLOCKS as u64));
    group.bench_function("cfg_cascading_dead_jumps_500", |benchmark| {
        benchmark.iter(|| black_box(optimize_statements(black_box(&statements))))
    });
    group.finish();
}

fn ppe_format_benchmarks(criterion: &mut Criterion) {
    let executable = compile(ARRAY_RECORD_SOURCE);
    let bytes = executable.to_buffer().expect("benchmark executable must serialize");
    let mut group = criterion.benchmark_group("ppe_format");

    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("serialize", |benchmark| {
        benchmark.iter(|| black_box(executable.to_buffer().expect("serialization must succeed")));
    });
    group.bench_function("deserialize", |benchmark| {
        benchmark.iter_batched(
            || bytes.clone(),
            |mut input| black_box(Executable::from_buffer(&mut input, false).expect("deserialization must succeed")),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn crypt_benchmarks(criterion: &mut Criterion) {
    const DATA_LEN: usize = 32 * 1024;
    let data: Vec<u8> = (0..DATA_LEN).map(|index| if index % 5 == 0 { index as u8 } else { 0 }).collect();
    let encoded = encode_rle(&data);
    let mut encrypted = data.clone();
    encrypt_chunks(&mut encrypted, 340, false);
    let mut encrypted_v300 = data.clone();
    encrypt_chunks(&mut encrypted_v300, 300, false);

    let mut group = criterion.benchmark_group("crypt");
    group.throughput(Throughput::Bytes(DATA_LEN as u64));
    group.bench_function("encrypt_chunks_v300_32k", |benchmark| {
        benchmark.iter_batched(
            || data.clone(),
            |mut input| {
                encrypt_chunks(&mut input, 300, false);
                black_box(input)
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("decrypt_chunks_v300_32k", |benchmark| {
        benchmark.iter_batched(
            || encrypted_v300.clone(),
            |mut input| {
                decrypt_chunks(&mut input, 300, false);
                black_box(input)
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("encrypt_chunks_v340_32k", |benchmark| {
        benchmark.iter_batched(
            || data.clone(),
            |mut input| {
                encrypt_chunks(&mut input, 340, false);
                black_box(input)
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("decrypt_chunks_v340_32k", |benchmark| {
        benchmark.iter_batched(
            || encrypted.clone(),
            |mut input| {
                decrypt_chunks(&mut input, 340, false);
                black_box(input)
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("encode_rle_32k", |benchmark| {
        benchmark.iter(|| black_box(encode_rle(black_box(&data))));
    });
    group.bench_function("decode_rle_32k", |benchmark| {
        benchmark.iter(|| black_box(decode_rle(black_box(&encoded))));
    });
    group.finish();
}

fn value_benchmarks(criterion: &mut Criterion) {
    let left = VariableValue::new_unbounded_string("ä".repeat(4096));
    let right = VariableValue::new_unbounded_string("x".repeat(4096));
    let mut array = VariableType::UnboundedString.create_empty_value();
    array.redim(1, 4096, 0, 0);
    let value = VariableValue::new_unbounded_string("value".to_string());
    let mut group = criterion.benchmark_group("values");

    group.throughput(Throughput::Bytes(16 * 1024));
    group.bench_function("string_concat_8k", |benchmark| {
        benchmark.iter(|| black_box(black_box(left.clone()) + black_box(right.clone())));
    });
    group.throughput(Throughput::Elements(4096));
    group.bench_function("array_write_4k", |benchmark| {
        benchmark.iter_batched(
            || array.clone(),
            |mut values| {
                for index in 0..4096 {
                    values.set_array_value(index, 0, 0, value.clone()).unwrap();
                }
                black_box(values)
            },
            BatchSize::SmallInput,
        );
    });
    group.throughput(Throughput::Elements(4096));
    group.bench_function("array_clone_4k", |benchmark| {
        benchmark.iter(|| black_box(black_box(&array).clone()));
    });
    group.finish();
}

async fn vm_state(root: &Path, areas: usize) -> (IcyBoardState, DiskIO) {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut board = IcyBoard::new();
    if areas > 0 {
        board.conferences.push(Conference {
            areas: Some(Arc::new(AreaList::new((0..areas).map(|_| MessageArea::default()).collect()))),
            ..Default::default()
        });
    }
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let node_state = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), node_state, node, Box::new(connection)).await;
    let io = DiskIO::new(root.to_str().expect("benchmark path must be UTF-8"), None);
    (state, io)
}

fn vm_benchmarks(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let root = tempfile::tempdir().unwrap();
    let file_name = root.path().join("benchmark.ppe");
    let mut group = criterion.benchmark_group("vm");
    group.sample_size(20);

    for (name, source, operations, areas) in [
        ("integer_loop_100k", ARITHMETIC_SOURCE, 100_000, 0),
        ("function_calls_25k", CALL_SOURCE, 25_000, 0),
        ("string_members_1k", STRING_SOURCE, 1_000, 0),
        ("array_record_access_2k", ARRAY_RECORD_SOURCE, 2_000, 0),
        ("array_foreach_10k", ARRAY_FOREACH_SOURCE, 10_000, 0),
        ("object_foreach_2k", OBJECT_FOREACH_SOURCE, 2_000, 2_000),
    ] {
        let executable = compile(source);
        group.throughput(Throughput::Elements(operations));
        group.bench_function(name, |benchmark| {
            benchmark.iter_batched(
                || runtime.block_on(vm_state(root.path(), areas)),
                |(mut state, mut io)| {
                    black_box(
                        runtime
                            .block_on(run(&file_name, &executable, &mut io, &mut state))
                            .expect("benchmark PPE must run"),
                    )
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    lexer_benchmarks,
    parse_benchmarks,
    compile_benchmarks,
    compile_from_ast_benchmarks,
    optimizer_benchmarks,
    ppe_format_benchmarks,
    crypt_benchmarks,
    value_benchmarks,
    vm_benchmarks
);
criterion_main!(benches);

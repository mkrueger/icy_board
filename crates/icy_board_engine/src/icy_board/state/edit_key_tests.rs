use super::*;

async fn input_state() -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.users.new_user(User {
        name: "EDIT KEY TEST".into(),
        security_level: 255,
        ..Default::default()
    });
    let caller = board.users[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(caller);
    state.session.cur_user_id = 0;
    (state, peer)
}

fn buffer(state: &mut IcyBoardState, source: KeySource, input: &str) {
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(source, ch)));
}

async fn next_key(state: &mut IcyBoardState) -> KeyChar {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(key) = state.get_char_edit().await.unwrap() {
                return key;
            }
            assert!(!state.session.request_logoff, "unexpected input EOF");
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("editor input stalled")
}

#[tokio::test]
async fn buffered_edit_keys_map_to_editor_controls_without_eating_the_next_key() {
    use control_codes::*;
    let cases = [
        ("\x1b[A", UP),
        ("\x1b[B", DOWN),
        ("\x1b[C", RIGHT),
        ("\x1b[D", LEFT),
        ("\x1b[H", HOME),
        ("\x1b[F", END),
        ("\x1b[1~", HOME),
        ("\x1b[7~", HOME),
        ("\x1b[4~", END),
        ("\x1b[8~", END),
        ("\x1b[2~", INS),
        ("\x1b[3~", DEL),
        ("\x1b[5~", PG_UP),
        ("\x1b[6~", PG_DN),
        ("\x1b[1;5D", CTRL_LEFT),
        ("\x1b[1;5C", CTRL_RIGHT),
        ("\x1bOP", CTRL_Z),
        ("\x1b[P", CTRL_Z),
        ("\x1b[11~", CTRL_Z),
        ("\x1b[[A", CTRL_Z),
        ("\x1bOA", UP),
        ("\x1bOB", DOWN),
        ("\x1bOC", RIGHT),
        ("\x1bOD", LEFT),
        ("\x1bOH", HOME),
        ("\x1bOF", END),
        // Retain the existing BBS terminal aliases; CSI @ has no trailing ~.
        ("\x1b[@", INS),
        ("\x1b[K", END),
        ("\x1b[V", PG_UP),
        ("\x1b[U", PG_DN),
        ("\x7f", DEL),
        ("\x08", BS),
        ("\x01", CTRL_LEFT),
        ("\x06", CTRL_RIGHT),
        ("\x1a", CTRL_Z),
        ("x", 'x'),
    ];
    let (mut state, _peer) = input_state().await;
    for source in [
        KeySource::User,
        KeySource::Sysop,
        KeySource::StuffedVisible,
        KeySource::StuffedHidden,
        KeySource::StuffedFile,
    ] {
        for (input, expected) in cases {
            buffer(&mut state, source, input);
            buffer(&mut state, source, "!");
            let key = next_key(&mut state).await;
            assert_eq!(key.ch, expected, "{source:?}: {input:?}");
            assert_eq!(key.source, source);
            assert_eq!(next_key(&mut state).await.ch, '!', "{input:?} consumed the next key");
            assert!(state.char_buffer.is_empty());
        }
    }
}

#[tokio::test]
async fn remote_sequences_decode_across_delayed_channel_fragments() {
    for (input, expected) in [
        ("\x1b[5~", control_codes::PG_UP),
        ("\x1b[6~", control_codes::PG_DN),
        ("\x1b[3~", control_codes::DEL),
        ("\x1b[1;5D", control_codes::CTRL_LEFT),
        ("\x1b[1;5C", control_codes::CTRL_RIGHT),
        ("\x1bOP", control_codes::CTRL_Z),
        ("\x1b[11~", control_codes::CTRL_Z),
    ] {
        let (mut state, mut peer) = input_state().await;
        let send = async {
            for byte in input.bytes().chain([b'!']) {
                sleep(Duration::from_millis(10)).await;
                peer.send(&[byte]).await.unwrap();
            }
        };
        let read = async {
            let key = next_key(&mut state).await;
            assert_eq!(key.ch, expected, "{input:?}");
            assert_eq!(key.source, KeySource::User);
            assert_eq!(next_key(&mut state).await.ch, '!');
        };
        tokio::join!(send, read);
        assert!(!state.session.request_logoff);
    }
}

#[tokio::test]
async fn sysop_sequences_fragment_and_bare_escape_keeps_both_node_channels() {
    let (mut state, _user_peer) = input_state().await;
    let (mut peer, connection) = ChannelConnection::create_pair();
    state.node_state.lock().await[state.node].as_mut().unwrap().sysop_connection = Some(connection);
    let send = async {
        for byte in b"\x1b[1;5D" {
            sleep(Duration::from_millis(10)).await;
            peer.send(&[*byte]).await.unwrap();
        }
    };
    let read = async {
        let key = next_key(&mut state).await;
        assert_eq!(key.ch, control_codes::CTRL_LEFT);
        assert_eq!(key.source, KeySource::Sysop);
    };
    tokio::join!(send, read);
    peer.send(b"\x1b").await.unwrap();
    let key = next_key(&mut state).await;
    assert_eq!(key.ch, control_codes::ESC);
    assert_eq!(key.source, KeySource::Sysop);
    {
        let nodes = state.node_state.lock().await;
        let node = nodes[state.node].as_ref().unwrap();
        assert!(node.bbs_channel.is_some());
        assert!(node.sysop_connection.is_some());
    }
    peer.send(b"z").await.unwrap();
    assert_eq!(next_key(&mut state).await.ch, 'z');
}

#[tokio::test]
async fn escape_preserves_a_following_remote_printable_character() {
    let (mut state, mut peer) = input_state().await;
    peer.send(b"\x1b").await.unwrap();
    let send = async {
        sleep(Duration::from_millis(20)).await;
        peer.send(b"x!").await.unwrap();
    };
    let read = async {
        assert_eq!(next_key(&mut state).await.ch, control_codes::ESC);
        assert_eq!(next_key(&mut state).await.ch, 'x');
        assert_eq!(next_key(&mut state).await.ch, '!');
    };
    tokio::join!(send, read);
}

#[tokio::test]
async fn incomplete_sequences_time_out_without_losing_suffixes_or_node_channels() {
    for input in ["\x1b", "\x1b[", "\x1b[2", "\x1b[1;5", "\x1bO"] {
        let (mut state, mut peer) = input_state().await;
        peer.send(input.as_bytes()).await.unwrap();
        assert_eq!(next_key(&mut state).await.ch, control_codes::ESC, "{input:?}");
        for expected in input.chars().skip(1) {
            assert_eq!(next_key(&mut state).await.ch, expected);
        }
        assert!(!state.session.request_logoff);
        assert!(state.node_state.lock().await[state.node].as_ref().unwrap().bbs_channel.is_some());
        peer.send(b"z").await.unwrap();
        assert_eq!(next_key(&mut state).await.ch, 'z');
    }
}

#[tokio::test]
async fn unknown_complete_sequences_do_not_leak_parameters_or_eat_printable_input() {
    let (mut state, _peer) = input_state().await;
    for input in ["\x1b[99~", "\x1b[1;2C", "\x1bOQ", "\x1b[2x"] {
        buffer(&mut state, KeySource::User, input);
        buffer(&mut state, KeySource::User, "!");
        assert!(state.get_char_edit().await.unwrap().is_none(), "{input:?}");
        assert_eq!(next_key(&mut state).await.ch, '!');
    }
}

#[tokio::test]
async fn malformed_and_overlong_sequences_are_bounded_and_preserve_the_suffix() {
    let (mut state, _peer) = input_state().await;
    for input in ["\x1b[2\r", "\x1b[12345678901234567890"] {
        buffer(&mut state, KeySource::StuffedHidden, input);
        assert_eq!(next_key(&mut state).await.ch, control_codes::ESC);
        for expected in input.chars().skip(1) {
            assert_eq!(next_key(&mut state).await.ch, expected);
        }
    }
}

#[tokio::test]
async fn sequence_continuations_never_cross_input_sources() {
    let (mut state, _peer) = input_state().await;
    for (first, second) in [
        (KeySource::User, KeySource::Sysop),
        (KeySource::Sysop, KeySource::User),
        (KeySource::StuffedHidden, KeySource::User),
    ] {
        buffer(&mut state, first, "\x1b[");
        buffer(&mut state, second, "A");
        assert_eq!(next_key(&mut state).await.ch, control_codes::ESC);
        let literal = next_key(&mut state).await;
        assert_eq!(literal.ch, '[');
        assert_eq!(literal.source, first);
        let other = next_key(&mut state).await;
        assert_eq!(other.ch, 'A');
        assert_eq!(other.source, second);
    }
}

#[tokio::test]
async fn channel_close_during_escape_input_sets_logoff_without_spinning() {
    for input in ["", "\x1b", "\x1b[", "\x1b[1;5"] {
        let (mut state, mut peer) = input_state().await;
        peer.send(input.as_bytes()).await.unwrap();
        peer.shutdown().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            while !state.session.request_logoff {
                assert!(state.get_char_edit().await.unwrap().is_none(), "{input:?}");
            }
            assert!(state.get_char_edit().await.unwrap().is_none());
        })
        .await
        .expect("closed input must terminate promptly");
        assert!(state.node_state.lock().await[state.node].as_ref().unwrap().bbs_channel.is_some());
    }
}

#[tokio::test]
async fn a_complete_key_is_delivered_before_channel_close_is_reported() {
    let (mut state, mut peer) = input_state().await;
    peer.send(b"\x1b[3~").await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(next_key(&mut state).await.ch, control_codes::DEL);
    assert!(state.get_char_edit().await.unwrap().is_none());
    assert!(state.session.request_logoff);
}

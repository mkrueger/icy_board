//! Source-to-VM coverage for S1 layouts, without crossing the deferred C1/C2 PPE boundary.

use std::sync::Arc;

use crate::{
    executable::ExecutableError,
    icy_board::{conferences::Conference, message_area::AreaList, message_area::MessageArea},
};

use super::{compile_errors, compile_in_memory, run_ppl, run_ppl_in_memory, run_ppl_in_memory_collecting};

const RANKS: [(&str, &str, usize); 3] = [("[]", "1", 2), ("[,]", "1, 2", 6), ("[,,]", "1, 2, 3", 24)];
const TONE: &[u8] = b"RIFFxxxxWAVEfmt ";
const AUDIO_TERMINAL: &[u8] = b"\x1b[=7;100;1n\x1b[=7;101;1;2;1n";

#[test]
fn s1_review_string_array_conversions_survive_ppe_roundtrip() {
    for (_, bounds, length) in RANKS {
        for (source_type, target_type) in [("STRING", "BIGSTR"), ("BIGSTR", "STRING")] {
            let source = format!(
                r#";$LANGVERSION 400
TYPE Item
 {target_type} Words[{bounds}]
ENDTYPE
TYPE Box
 Item Child
ENDTYPE
{source_type} supplied[{bounds}]
supplied[{bounds}] = "before"
Item literal = Item {{ Words = supplied }}
Box item
item.Child.Words = supplied
supplied[{bounds}] = "after"
PRINTLN literal.Words[{bounds}], ":", item.Child.Words[{bounds}], ":", item.Child.Words.Len()
Item copied = item.Child
item.Child.Words[{bounds}] = "changed"
PRINTLN copied.Words[{bounds}], ":", supplied[{bounds}]
"#
            );
            assert_eq!(run_ppl(&source), format!("before:before:{length}\nbefore:after\n"), "{source}");
        }
    }
}

#[test]
fn s1_review_dynamic_string_array_conversions_include_empty_values() {
    for (rank, bounds, length) in RANKS {
        for (source_type, target_type) in [("STRING", "BIGSTR"), ("BIGSTR", "STRING")] {
            let source = format!(
                r#";$LANGVERSION 400
TYPE Item
 {target_type} Words{rank}
ENDTYPE
{source_type} supplied[{bounds}], empty{rank}
supplied[{bounds}] = "before"
Item literal = Item {{ Words = supplied }}
Item item
item.Words = supplied
supplied[{bounds}] = "after"
PRINTLN literal.Words[{bounds}], ":", item.Words[{bounds}], ":", item.Words.Len()
item.Words = empty
literal = Item {{ Words = empty }}
PRINTLN item.Words.Len(), ":", literal.Words.Len()
"#
            );
            assert_eq!(run_ppl_in_memory(&source), format!("before:before:{length}\n0:0\n"), "{source}");
        }
    }
}

#[test]
fn s1_review_empty_host_and_record_fields_have_defaults_in_every_rank() {
    for (rank, bounds, _) in RANKS {
        let zeros = bounds.split(", ").map(|_| "0").collect::<Vec<_>>().join(", ");
        let negative = bounds.split(", ").map(|_| "-1").collect::<Vec<_>>().join(", ");
        for indices in [&zeros, &negative, bounds] {
            let source = format!(
                r#";$LANGVERSION 400
ENUM Choice
 First = 7
ENDENUM
TYPE Leaf
 INTEGER Values[]
 INTEGER Fixed[0]
 SURFACE Image
 Choice Selected
ENDTYPE
TYPE Box
 SURFACE Images{rank}
 AUDIO Sounds{rank}
 Leaf Children{rank}
ENDTYPE
Box item
PRINTLN item.Images[{indices}].Valid, ":", item.Sounds[{indices}].Valid
PRINTLN item.Children[{indices}].Values.Len(), ":", item.Children[{indices}].Fixed.Len(), ":", item.Children[{indices}].Image.Valid, ":", TOINTEGER(item.Children[{indices}].Selected)
PRINTLN item.Images.Len(), ":", item.Sounds.Len(), ":", item.Children.Len()
"#
            );
            assert_eq!(run_ppl_in_memory(&source), "0:0\n0:1:0:7\n0:0:0\n", "{source}");
        }
    }
}

#[test]
fn s1_review_missing_elements_work_in_variable_and_legacy_index_paths() {
    for (rank, bounds, _) in RANKS {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Fixed[0]
 SURFACE Image
ENDTYPE
TYPE Box
 SURFACE Images{rank}
 Leaf Children{rank}
ENDTYPE
Box item
SURFACE images{rank}
Leaf children{rank}
PRINTLN images[{bounds}].Valid, ":", images({bounds}).Valid, ":", (images)[{bounds}].Valid
PRINTLN children[{bounds}].Fixed.Len(), ":", children({bounds}).Image.Valid, ":", (children)[{bounds}].Fixed.Len()
PRINTLN item.Images({bounds}).Valid, ":", item.Children({bounds}).Fixed.Len()
"#
        );
        assert_eq!(run_ppl_in_memory(&source), "0:0:0\n1:0:1\n0:1\n", "{source}");
    }
}

#[test]
fn s1_review_parenthesized_redim_executes_with_captured_indices() {
    for (rank, bounds, length) in RANKS {
        let zeros = bounds.split(", ").map(|_| "0").collect::<Vec<_>>().join(", ");
        let indices = bounds.split(", ").map(|_| "Index()").collect::<Vec<_>>().join(", ");
        let bound_calls = bounds.split(", ").map(|bound| format!("Bound({bound})")).collect::<Vec<_>>().join(", ");
        let calls = bounds.split(", ").count();
        let source = format!(
            r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Children[{zeros}]
ENDTYPE
Branch roots[{zeros}]
INTEGER indexCalls, boundCalls
REDIM (((((roots)))[{indices}].Children))[{indices}].Values, {bound_calls}
PRINTLN roots[{zeros}].Children[{zeros}].Values.Len(), ":", indexCalls, ":", boundCalls
FUNCTION Index() INTEGER
 indexCalls += 1
 RETURN 0
ENDFUNC
FUNCTION Bound(INTEGER upper) INTEGER
 boundCalls += 1
 RETURN upper
ENDFUNC
"#
        );
        assert_eq!(run_ppl_in_memory(&source), format!("{length}:{}:{calls}\n", 2 * calls), "{source}");
    }
}

#[test]
fn s1_in_memory_execution_does_not_fall_back_through_ppe_serialization() {
    for (field, expression) in [("INTEGER Values[]", "value.Values.Len()"), ("AUDIO Sound", "value.Sound.Valid")] {
        let source = format!(";$LANGVERSION 400\nTYPE Item\n {field}\nENDTYPE\nItem value\nPRINTLN {expression}\n");
        assert!(matches!(
            compile_in_memory(&source).to_buffer(),
            Err(ExecutableError::UnsupportedRecordFieldEncoding { type_id: 100, field_index: 0 })
        ));
        assert_eq!(run_ppl_in_memory(&source), "0\n", "{source}");
    }
}

#[test]
fn s1_nested_dynamic_rank_one_two_three_assignments() {
    for (rank, bounds, length) in RANKS {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
Branch roots[1]
roots[1].Leaves.Redim(1)
INTEGER supplied[{bounds}]
supplied[{bounds}] = 7
roots[1].Leaves[1].Values = supplied
supplied[{bounds}] = 99
PRINTLN roots[0].Leaves.Len(), ":", roots[1].Leaves[0].Values.Len(), ":", roots[1].Leaves[1].Values.Len(), ":", roots[1].Leaves[1].Values[{bounds}]
INTEGER empty{rank}
roots[1].Leaves[1].Values = empty
PRINTLN roots[1].Leaves[1].Values.Len(), ":", roots[1].Leaves.Len()
"#
        );
        assert_eq!(run_ppl_in_memory(&source), format!("0:0:{length}:7\n0:2\n"), "{source}");
    }
}

#[test]
fn s1_source_redim_forms_capture_nested_indices_and_bounds_once() {
    for (rank, bounds, length) in RANKS {
        let bound_calls = bounds.split(", ").map(|bound| format!("Bound({bound})")).collect::<Vec<_>>().join(", ");
        let dimensions = bounds.split(", ").count();
        let trace = bounds.replace(", ", "");
        for resize in [
            format!("roots[OuterIndex()].Leaves[InnerIndex()].Values.Redim({bound_calls})"),
            format!("REDIM roots[OuterIndex()].Leaves[InnerIndex()].Values, {bound_calls}"),
        ] {
            let source = format!(
                r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
Branch roots[1]
roots[1].Leaves.Redim(1)
INTEGER outerCalls, innerCalls, boundCalls
{resize}
roots[1].Leaves[1].Values[{bounds}] = 9
PRINTLN ":", outerCalls, ":", innerCalls, ":", boundCalls, ":", roots[1].Leaves[1].Values.Len(), ":", roots[1].Leaves[1].Values[{bounds}], ":", roots[0].Leaves.Len(), ":", roots[1].Leaves[0].Values.Len()
FUNCTION OuterIndex() INTEGER
 PRINT "O"
 outerCalls += 1
 RETURN 1
ENDFUNC
FUNCTION InnerIndex() INTEGER
 PRINT "I"
 innerCalls += 1
 RETURN 1
ENDFUNC
FUNCTION Bound(INTEGER upper) INTEGER
 PRINT upper
 boundCalls += 1
 RETURN upper
ENDFUNC
"#
            );
            assert_eq!(run_ppl_in_memory(&source), format!("OI{trace}:1:1:{dimensions}:{length}:9:0:0\n"), "{source}");
        }
    }
}

#[test]
fn s1_nested_dynamic_array_copies_detach_at_every_rank() {
    for (rank, bounds, length) in RANKS {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
Branch original
original.Leaves.Redim(0)
original.Leaves[0].Values.Redim({bounds})
original.Leaves[0].Values[{bounds}] = 7
Branch copied = original
PRINTLN original = copied
copied.Leaves[0].Values[{bounds}] = 9
PRINTLN original.Leaves[0].Values[{bounds}], ":", copied.Leaves[0].Values[{bounds}], ":", original <> copied
INTEGER detached{rank} = copied.Leaves[0].Values
detached[{bounds}] = 11
PRINTLN copied.Leaves[0].Values[{bounds}], ":", detached[{bounds}]
copied.Leaves[0].Values.Redim({bounds})
PRINTLN original.Leaves[0].Values[{bounds}], ":", copied.Leaves[0].Values[{bounds}], ":", copied.Leaves[0].Values.Len()
copied.Leaves.Redim(1)
PRINTLN original.Leaves.Len(), ":", original.Leaves[0].Values[{bounds}], ":", copied.Leaves[0].Values.Len(), ":", copied.Leaves[1].Values.Len()
"#
        );
        assert_eq!(run_ppl_in_memory(&source), format!("1\n7:9:1\n9:11\n7:0:{length}\n1:7:0:0\n"), "{source}");
    }
}

#[test]
fn s1_var_dynamic_fields_copy_back_values_and_shapes_through_nested_paths() {
    for (rank, bounds, length) in RANKS {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
Branch roots[1]
roots[1].Leaves.Redim(1)
Change(roots[1].Leaves[1].Values)
PRINTLN roots[1].Leaves[1].Values.Len(), ":", roots[1].Leaves[1].Values[{bounds}], ":", roots[1].Leaves[0].Values.Len()
TouchCopy(roots[1].Leaves[1].Values)
PRINTLN roots[1].Leaves[1].Values.Len(), ":", roots[1].Leaves[1].Values[{bounds}]
PROCEDURE Change(VAR INTEGER values{rank})
 values.Redim({bounds})
 values[{bounds}] = 17
ENDPROC
PROCEDURE TouchCopy(INTEGER values{rank})
 values[{bounds}] = 99
 INTEGER empty{rank}
 values = empty
ENDPROC
"#
        );
        assert_eq!(run_ppl_in_memory(&source), format!("{length}:17:0\n{length}:17\n"), "{source}");
    }
}

#[test]
fn s1_var_whole_records_and_scalar_fields_copy_back_through_nested_paths() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Values[]
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
TYPE Tree
 Branch Branches[1]
ENDTYPE
Tree roots[1]
roots[1].Branches[1].Leaves.Redim(1)
ReplaceLeaf(roots[1].Branches[1].Leaves[1])
Increment(roots[1].Branches[1].Leaves[1].Values[2])
PRINTLN roots[1].Branches[1].Leaves[1].Values.Len(), ":", roots[1].Branches[1].Leaves[1].Values[2], ":", roots[1].Branches[1].Leaves[0].Values.Len()
Tree saved = roots[1]
ReplaceBranch(roots[1].Branches[1])
PRINTLN roots[1].Branches[1].Leaves.Len(), ":", roots[1].Branches[1].Leaves[0].Values[0], ":", saved.Branches[1].Leaves.Len(), ":", saved.Branches[1].Leaves[1].Values[2], ":", roots[0].Branches[1].Leaves.Len()
PROCEDURE ReplaceLeaf(VAR Leaf item)
 INTEGER numbers[] = { 4, 5, 6 }
 item = Leaf { Values = numbers }
ENDPROC
PROCEDURE Increment(VAR INTEGER number)
 number += 1
ENDPROC
PROCEDURE ReplaceBranch(VAR Branch item)
 Branch replacement
 replacement.Leaves.Redim(0)
 replacement.Leaves[0].Values.Redim(0)
 replacement.Leaves[0].Values[0] = 21
 item = replacement
ENDPROC
"#,
    );
    assert_eq!(output, "3:7:0\n1:21:2:7:0\n");
}

#[test]
fn s1_call_local_record_defaults_are_fresh_on_repeated_calls() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Payload
 INTEGER Vector[]
 INTEGER Matrix[,]
 INTEGER Cube[,,]
 AUDIO Sound
 SURFACE Image
ENDTYPE
TYPE Wrapper
 Payload Item
 Payload Fixed[0]
ENDTYPE
Work(TRUE)
Work(FALSE)
Work(TRUE)
PROCEDURE Work(BOOLEAN grow)
 Wrapper local
 PRINTLN "in:", local.Item.Vector.Len(), local.Item.Matrix.Len(), local.Item.Cube.Len(), local.Fixed[0].Vector.Len(), local.Item.Sound.Valid, local.Item.Image.Valid
 IF grow THEN
  local.Item.Vector.Redim(1)
  REDIM local.Item.Matrix, 1, 2
  local.Item.Cube.Redim(1, 2, 3)
  local.Fixed[0].Vector.Redim(2)
 ENDIF
 PRINTLN "out:", local.Item.Vector.Len(), ":", local.Item.Matrix.Len(), ":", local.Item.Cube.Len(), ":", local.Fixed[0].Vector.Len()
ENDPROC
"#,
    );
    assert_eq!(output, "in:000000\nout:2:6:24:3\nin:000000\nout:0:0:0:0\nin:000000\nout:2:6:24:3\n");
}

#[test]
fn s1_recursive_routines_restore_nonrecursive_record_local_arrays() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Payload
 INTEGER Vector[]
 INTEGER Matrix[,]
 INTEGER Cube[,,]
ENDTYPE
TYPE Wrapper
 Payload Item
ENDTYPE
Visit(2)
PROCEDURE Visit(INTEGER depth)
 Wrapper local
 PRINTLN "in:", depth, ":", local.Item.Vector.Len(), local.Item.Matrix.Len(), local.Item.Cube.Len()
 local.Item.Vector.Redim(depth)
 local.Item.Matrix.Redim(depth, 0)
 REDIM local.Item.Cube, depth, 0, 0
 local.Item.Vector[depth] = depth + 10
 local.Item.Matrix[depth, 0] = depth + 20
 local.Item.Cube[depth, 0, 0] = depth + 30
 IF depth > 0 Visit(depth - 1)
 PRINTLN "out:", depth, ":", local.Item.Vector.Len(), ":", local.Item.Matrix.Len(), ":", local.Item.Cube.Len(), ":", local.Item.Vector[depth], ":", local.Item.Matrix[depth, 0], ":", local.Item.Cube[depth, 0, 0]
ENDPROC
"#,
    );
    assert_eq!(
        output,
        "in:2:000\nin:1:000\nin:0:000\nout:0:1:1:1:10:20:30\nout:1:2:2:2:11:21:31\nout:2:3:3:3:12:22:32\n"
    );
}

#[test]
fn s1_record_function_return_defaults_do_not_retain_previous_arrays() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Payload
 INTEGER Values[]
 AUDIO Sound
 SURFACE Image
ENDTYPE
Payload first = Make(TRUE)
Payload second = Make(FALSE)
PRINTLN first.Values.Len(), ":", first.Values[1], ":", second.Values.Len(), ":", second.Sound.Valid, second.Image.Valid
FUNCTION Make(BOOLEAN populated) Payload
 Payload local
 IF populated THEN
  local.Values.Redim(1)
  local.Values[1] = 8
  RETURN local
 ENDIF
ENDFUNC
"#,
    );
    assert_eq!(output, "2:8:0:00\n");
}

#[test]
fn s1_partial_record_literals_bind_nested_host_and_dynamic_defaults() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Media
 AUDIO Sound
 SURFACE Image
 AREA Destination
 CONTACT Person
 INTEGER Values[]
ENDTYPE
TYPE Wrapper
 INTEGER Number
 Media Item
 Media Fixed[0]
 Media Dynamic[]
ENDTYPE
Wrapper value = Wrapper { Number = 7 }
PRINTLN value.Number, ":", value.Item.Sound.Valid, ":", value.Item.Sound.Playing, ":", value.Item.Sound.Channel
PRINTLN value.Item.Image.Valid, ":", value.Item.Image.Width, ":", value.Item.Image.Height, ":", value.Item.Destination.Valid, ":", value.Item.Person.Account.Len()
PRINTLN value.Item.Values.Len(), ":", value.Fixed.Len(), ":", value.Fixed[0].Values.Len(), ":", value.Dynamic.Len()
PRINTLN value.Item.Sound = value.Fixed[0].Sound, ":", value.Item.Image = value.Fixed[0].Image
value.Item.Values.Redim(1)
value = Wrapper { Number = 9 }
PRINTLN value.Number, ":", value.Item.Values.Len(), ":", value.Item.Sound.Valid, ":", value.Item.Image.Valid
"#,
    );
    assert_eq!(output, "7:0:0:-1\n0:0:0:0:0\n0:1:0:0\n1:1\n9:0:0:0\n");
}

#[test]
fn s1_redim_record_arrays_binds_new_host_leaves_at_every_rank() {
    for (rank, bounds, length) in RANKS {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Media
 AUDIO Sound
 SURFACE Image
 INTEGER Values[]
ENDTYPE
TYPE Wrapper
 Media Items{rank}
ENDTYPE
Wrapper value
value.Items.Redim({bounds})
PRINTLN value.Items.Len(), ":", value.Items[{bounds}].Sound.Valid, ":", value.Items[{bounds}].Image.Valid, ":", value.Items[{bounds}].Values.Len()
value.Items[{bounds}].Values.Redim(1)
REDIM value.Items, {bounds}
PRINTLN value.Items[{bounds}].Values.Len(), ":", value.Items[{bounds}].Sound.Channel, ":", value.Items[{bounds}].Image.Width
"#
        );
        assert_eq!(run_ppl_in_memory(&source), format!("{length}:0:0:0\n0:-1:0\n"), "{source}");
    }
}

#[test]
fn s1_sprite_record_copies_keep_surface_identity_after_free_and_reuse() {
    let output = run_ppl_in_memory(
        r#";$LANGVERSION 400
TYPE Sprite
 SURFACE Image
ENDTYPE
TYPE Scene
 Sprite Sprites[]
ENDTYPE
Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
Scene original
original.Sprites.Redim(0)
original.Sprites[0].Image = Surface.New(2, 2)
Scene copied = original
copied.Sprites[0].Image.SetPixel(0, 0, Rgb(1, 2, 3))
PRINTLN "live:", original = copied, original.Sprites[0].Image.Valid, original.Sprites[0].Image.GetPixel(0, 0) = Rgb(1, 2, 3)
copied.Sprites[0].Image.Free()
Sprite replacement = Sprite { Image = Surface.New(2, 2) }
replacement.Image.Clear(Rgb(10, 20, 30))
PRINTLN "stale:", original.Sprites[0].Image.Valid, copied.Sprites[0].Image.Valid, ":", original = copied, ":", original.Sprites[0] <> replacement
PRINTLN "ops:", original.Sprites[0].Image.Clear(0), copied.Sprites[0].Image.Free()
PRINTLN "new:", replacement.Image.Valid, replacement.Image.GetPixel(0, 0) = Rgb(10, 20, 30)
"#,
    );
    assert_eq!(output, "live:111\nstale:00:1:1\nops:00\nnew:11\n");
}

#[test]
fn s1_sprite_record_copies_stay_stale_after_graphics_reinitialization() {
    for shutdown in ["", "Terminal.Gfx.Shutdown()"] {
        let source = format!(
            r#";$LANGVERSION 400
TYPE Sprite
 SURFACE Image
ENDTYPE
Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
Sprite original = Sprite {{ Image = Surface.New(2, 2) }}
Sprite copied = original
{shutdown}
Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
Sprite replacement = Sprite {{ Image = Surface.New(3, 4) }}
replacement.Image.SetPixel(0, 0, Rgb(1, 2, 3))
PRINTLN "stale:", original.Image.Valid, copied.Image.Valid, ":", copied.Image.Width, ":", copied.Image.Height, ":", original = copied, ":", original <> replacement
PRINTLN "ops:", copied.Image.Clear(0), copied.Image.Free()
PRINTLN "new:", replacement.Image.Valid, ":", replacement.Image.Width, ":", replacement.Image.Height, ":", replacement.Image.GetPixel(0, 0) = Rgb(1, 2, 3)
"#
        );
        let output = run_ppl_in_memory(&source);
        let prefix = if shutdown.is_empty() { "" } else { "\x1b[0m" };
        assert_eq!(output, format!("{prefix}stale:00:0:0:1:1\nops:00\nnew:1:3:4:1\n"), "{source}");
    }
}

#[test]
fn s1_audio_record_copies_keep_identity_after_free_and_channel_reuse() {
    let (_, output) = run_ppl_in_memory_collecting(
        r#";$LANGVERSION 400
TYPE Track
 AUDIO Sound
ENDTYPE
TYPE Playlist
 Track Tracks[]
ENDTYPE
Playlist original
original.Tracks.Redim(0)
original.Tracks[0].Sound = Audio.Load("tone.wav")
Playlist copied = original
copied.Tracks[0].Sound.Play(TRUE)
PRINTLN "live:", original = copied, original.Tracks[0].Sound.Valid, original.Tracks[0].Sound.Playing
original.Tracks[0].Sound.Free()
Track replacement = Track { Sound = Audio.Load("tone.wav") }
replacement.Sound.Play(TRUE)
PRINTLN "stale:", copied.Tracks[0].Sound.Valid, copied.Tracks[0].Sound.Playing, ":", original = copied, ":", original.Tracks[0] <> replacement
PRINTLN "ops:", copied.Tracks[0].Sound.Play(), copied.Tracks[0].Sound.Stop(), copied.Tracks[0].Sound.Free()
PRINTLN "new:", replacement.Sound.Valid, replacement.Sound.Playing, ":", replacement.Sound.Channel
"#,
        |_| {},
        &[("tone.wav", TONE)],
        AUDIO_TERMINAL,
    );
    assert!(output.contains("live:111\n"), "{output:?}");
    assert!(output.ends_with("stale:00:1:1\nops:000\nnew:11:0\n"), "{output:?}");
    assert_eq!(output.matches("Queue;C=2;S=2").count(), 2, "{output:?}");
    assert_eq!(output.matches("Flush;C=2;O=0").count(), 1, "{output:?}");
    assert_eq!(output.matches("SyncTERM:C;S;").count(), 1, "{output:?}");
}

#[test]
fn s1_menu_area_assignment_preserves_readable_snapshot_and_permissions() {
    let (_, output) = run_ppl_in_memory_collecting(
        r#";$LANGVERSION 400
TYPE MenuEntry
 STRING Label
 AREA TargetArea
ENDTYPE
MenuEntry entries[]
entries.Redim(1)
AREA selected = Board.Conferences[0].Areas[0]
entries[0] = MenuEntry { Label = "first", TargetArea = selected }
MenuEntry copied = entries[0]
selected = Board.Conferences[0].Areas[1]
entries[1].TargetArea = selected
entries[0].Label = "renamed"
PRINTLN entries[0].Label, ":", entries[0].TargetArea.Name, ":", copied.Label, ":", copied.TargetArea.Name
PRINTLN entries[0].TargetArea.Valid, entries[0].TargetArea.HasAccess(), entries[0].TargetArea.CanEnter(), entries[0].TargetArea.CanAttach()
PRINTLN copied.TargetArea.Valid, copied.TargetArea.HasAccess(), copied.TargetArea.CanEnter(), copied.TargetArea.CanAttach()
PRINTLN entries[1].TargetArea.Name, ":", entries[1].TargetArea.Valid, entries[1].TargetArea.HasAccess(), entries[1].TargetArea.CanEnter(), entries[1].TargetArea.CanAttach()
PRINTLN Board.Conferences[0].Areas[0].Name, ":", Board.Conferences[0].Areas[0].CanEnter(), Board.Conferences[0].Areas[1].HasAccess()
"#,
        |board| {
            board.conferences.clear();
            board.conferences.push(Conference {
                name: "Main Board".to_string(),
                areas: Some(Arc::new(AreaList::new(vec![
                    MessageArea {
                        name: "Listed".to_string(),
                        req_level_to_list: "TRUE".parse().unwrap(),
                        req_level_to_enter: "FALSE".parse().unwrap(),
                        req_level_to_save_attach: "FALSE".parse().unwrap(),
                        ..Default::default()
                    },
                    MessageArea {
                        name: "Restricted".to_string(),
                        req_level_to_list: "FALSE".parse().unwrap(),
                        req_level_to_enter: "FALSE".parse().unwrap(),
                        req_level_to_save_attach: "FALSE".parse().unwrap(),
                        ..Default::default()
                    },
                ]))),
                ..Default::default()
            });
        },
        &[],
        &[],
    );
    assert_eq!(output, "renamed:Listed:first:Listed\n1100\n1100\nRestricted:1000\nListed:00\n");
}

#[test]
fn s1_menu_area_record_fields_do_not_make_snapshot_properties_writable() {
    for assignment in ["entries[0].TargetArea.Name = \"changed\"", "entries[0].TargetArea.Name += \"changed\""] {
        let source = format!(";$LANGVERSION 400\nTYPE MenuEntry\n AREA TargetArea\nENDTYPE\nMenuEntry entries[0]\n{assignment}\n");
        assert_eq!(compile_errors(&source), vec!["'Name' can only be read"], "{source}");
    }
}

#[test]
fn s1_fixed_nested_shapes_match_existing_serialized_execution() {
    let source = r#";$LANGVERSION 400
TYPE Leaf
 INTEGER Vector[0]
 INTEGER Matrix[1, 2]
 INTEGER Cube[1, 2, 3]
ENDTYPE
TYPE Branch
 Leaf Leaves[1]
ENDTYPE
Branch original
INTEGER singleton[0]
singleton[0] = 7
original.Leaves[1].Vector = singleton
original.Leaves[1].Matrix[1, 2] = 42
original.Leaves[1].Cube[1, 2, 3] = 77
Branch copied = original
copied.Leaves[1].Cube[1, 2, 3] = 88
PRINTLN original.Leaves.Len(), ":", original.Leaves[0].Vector.Len(), ":", original.Leaves[0].Matrix.Len(), ":", original.Leaves[0].Cube.Len()
PRINTLN original.Leaves[1].Vector[0], ":", original.Leaves[1].Matrix[1, 2], ":", original.Leaves[1].Cube[1, 2, 3], ":", copied.Leaves[1].Cube[1, 2, 3]
PRINTLN original.Leaves[0].Vector[0], ":", original.Leaves[0].Matrix[1, 2], ":", original.Leaves[0].Cube[1, 2, 3]
"#;
    let expected = "2:1:6:24\n7:42:77:88\n0:0:0\n";
    assert_eq!(run_ppl_in_memory(source), expected);
    assert_eq!(run_ppl(source), expected);
}

#[test]
fn s1_fixed_nested_fields_still_reject_resizing_and_shape_changes() {
    let header = ";$LANGVERSION 400\nTYPE Leaf\n INTEGER Values[1, 2]\nENDTYPE\nTYPE Branch\n Leaf Leaves[0]\nENDTYPE\nBranch value\n";
    for resize in ["value.Leaves[0].Values.Redim(2, 3)", "REDIM value.Leaves[0].Values, 2, 3"] {
        let errors = compile_errors(&format!("{header}{resize}\n"));
        assert!(
            errors.iter().any(|error| error.contains("fixed size and cannot be redimensioned")),
            "{errors:?}"
        );
    }
    let errors = compile_errors(&format!("{header}INTEGER other[2, 3]\nvalue.Leaves[0].Values = other\n"));
    assert!(errors.iter().any(|error| error.contains("Record array field 'Values' expects")), "{errors:?}");
}

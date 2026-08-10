//! The bodies behind PPL's dBase opcodes.
//!
//! Most of these opcodes exist twice, once as a statement and once as a function that
//! reports success, so each operation lives here once and both forms call it. The `bool`
//! that comes back is PPL's own convention: `true` means the operation failed.

use std::path::PathBuf;

use crate::{
    Res,
    executable::{GenericVariableData, PPEExpr, VariableValue},
    vm::{VirtualMachine, io::MAX_FILE_CHANNELS},
};

use super::{file::parse_field_info, index, table_path};

async fn channel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr], at: usize) -> Res<i32> {
    Ok(vm.eval_expr(&args[at]).await?.as_int())
}

async fn text(vm: &mut VirtualMachine<'_>, args: &[PPEExpr], at: usize) -> Res<String> {
    Ok(vm.eval_expr(&args[at]).await?.as_string())
}

// -- opening and closing ----------------------------------------------------------------

pub async fn dcreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;

    let PPEExpr::Value(id) = args[3] else {
        return Ok(true);
    };
    let GenericVariableData::Dim1(specs) = vm.variable_table.get_value(id).generic_data.clone() else {
        return Ok(true);
    };

    let mut fields = Vec::new();
    for spec in &specs {
        let spec = spec.as_string();
        if spec.trim().is_empty() {
            continue;
        }
        let Some(field) = parse_field_info(&spec) else {
            return Ok(true);
        };
        fields.push(field);
    }
    if fields.is_empty() {
        return Ok(true);
    }

    let path = vm.resolve_file(&table_path(&name)).await;
    Ok(vm.dbase.create(channel, &path, &fields))
}

pub async fn dopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    let path = vm.resolve_file(&table_path(&name)).await;
    // PCBoard reports success even when the table is not there, so a miss only shows up
    // on the first access.
    vm.dbase.open(channel, &path);
    Ok(false)
}

pub async fn dclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.close(channel))
}

pub async fn dcloseall(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<bool> {
    Ok(vm.dbase.close_all())
}

pub async fn dsetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let alias = text(vm, args, 1).await?;
    Ok(vm.dbase.set_alias(channel, &alias))
}

pub async fn dgetalias(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<String> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.alias(channel))
}

pub async fn dselect(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let alias = text(vm, args, 0).await?;
    Ok(vm.dbase.select(&alias))
}

pub async fn dnext(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<i32> {
    Ok(vm.dbase.next_free())
}

pub async fn dchkstat(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    Ok(i32::from(!vm.dbase.is_open(channel)))
}

pub async fn derr(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.error(channel))
}

// -- navigation -------------------------------------------------------------------------

pub async fn dtop(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.top(channel))
}

pub async fn dbottom(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.bottom(channel))
}

pub async fn dgo(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let record_no = vm.eval_expr(&args[1]).await?.as_int();
    Ok(vm.dbase.go(channel, record_no))
}

pub async fn dskip(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let count = vm.eval_expr(&args[1]).await?.as_int();
    Ok(vm.dbase.skip(channel, count))
}

pub async fn dbof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.bof(channel))
}

pub async fn deof(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.eof(channel))
}

pub async fn dreccount(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    if !vm.dbase.is_open(channel) {
        return Ok(0);
    }
    Ok(vm.dbase.record_count(channel))
}

pub async fn drecno(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.record_no(channel))
}

// -- field metadata ---------------------------------------------------------------------

pub async fn dfields(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.field_count(channel))
}

pub async fn dname(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<String> {
    let channel = channel(vm, args, 0).await?;
    let number = vm.eval_expr(&args[1]).await?.as_int();
    Ok(vm.dbase.field_name(channel, number))
}

pub async fn dtype(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<String> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.field_type(channel, &name))
}

pub async fn dlength(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.field_length(channel, &name))
}

pub async fn ddecimals(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.field_decimals(channel, &name))
}

// -- field values -----------------------------------------------------------------------

/// The raw padded field bytes, or the previous result when the field is unknown.
pub async fn dget(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<String> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.get_field(channel, &name))
}

pub async fn dput(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    let value = vm.eval_expr(&args[2]).await?;
    Ok(vm.dbase.put_field(channel, &name, &value))
}

pub async fn dfblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.blank_field(channel, &name))
}

pub async fn dfcopy(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let from = channel(vm, args, 0).await?;
    let from_name = text(vm, args, 1).await?;
    let to = channel(vm, args, 2).await?;
    let to_name = text(vm, args, 3).await?;
    Ok(vm.dbase.copy_field(from, &from_name, to, &to_name))
}

// -- record lifecycle -------------------------------------------------------------------

pub async fn dnew(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.new_record(channel))
}

pub async fn dadd(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.add_record(channel))
}

pub async fn dappend(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.append_record(channel))
}

pub async fn dblank(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.blank_record(channel))
}

pub async fn ddelete(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.set_deleted(channel, true))
}

pub async fn drecall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.set_deleted(channel, false))
}

pub async fn ddeleted(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.deleted(channel))
}

pub async fn dchanged(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.changed(channel))
}

pub async fn dpack(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.pack(channel))
}

// -- indexes ----------------------------------------------------------------------------

async fn index_file(vm: &mut VirtualMachine<'_>, name: &str) -> PathBuf {
    vm.resolve_file(&index::index_path(name)).await
}

pub async fn dncreate(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    let expression = text(vm, args, 2).await?;
    let path = index_file(vm, &name).await;
    Ok(vm.dbase.create_index(channel, &name, path, &expression))
}

pub async fn dnopen(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    let path = index_file(vm, &name).await;
    Ok(vm.dbase.open_index(channel, &name, path))
}

pub async fn dnclose(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.close_index(channel, &name))
}

pub async fn dncloseall(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    Ok(vm.dbase.close_all_indexes(channel))
}

pub async fn dtag(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<bool> {
    let channel = channel(vm, args, 0).await?;
    let name = text(vm, args, 1).await?;
    Ok(vm.dbase.tag(channel, &name))
}

pub async fn dseek(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = channel(vm, args, 0).await?;
    let search = text(vm, args, 1).await?;
    Ok(vm.dbase.seek(channel, &search))
}

// -- locking ----------------------------------------------------------------------------

/// Every lock succeeds. PCBoard's locks only mean anything against other DOS nodes
/// sharing the file, which is not a situation this engine can be in.
pub async fn dlock(_vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<bool> {
    Ok(false)
}

/// The lowest unused FOPEN channel, or -1 when all are busy. Like DNEXT it does not reserve anything.
pub async fn fnext(vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<i32> {
    Ok((0..MAX_FILE_CHANNELS).find(|c| !vm.io.is_open(*c)).unwrap_or(-1))
}

/// PCBoard has no message for any of its dBase error codes, and neither do we.
pub async fn derrmsg(_vm: &mut VirtualMachine<'_>, _args: &[PPEExpr]) -> Res<String> {
    Ok(String::new())
}

/// Writes DGET's result into the variable the statement form was handed.
pub async fn dget_stmt(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<()> {
    let value = dget(vm, args).await?;
    vm.set_variable(&args[2], VariableValue::new_string(value)).await
}

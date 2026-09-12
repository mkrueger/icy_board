use async_trait::async_trait;
use dizbase::file_base::{
    FileBase,
    reader::{self, FileEntry, FilePage},
};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    datetime::IcbDate,
    executable::{VariableData, VariableType, VariableValue},
    icy_board::file_directory::FileDirectory,
    parser::{FILE_ENTRY_ID, FILE_PAGE_ID},
    vm::VirtualMachine,
};

use super::ppl_error::{ERR_DENIED, ERR_INVALID, ERR_IO, ERR_KIND_FILE, ERR_LIMIT, ERR_UNAVAILABLE, PplError};

#[derive(Clone, Default)]
pub struct PplFileEntry(Option<FileEntry>);

impl UserData for PplFileEntry {
    const TYPE_NAME: &'static str = "FileEntry";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(Self::default(), FILE_ENTRY_ID));

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(FILE_ENTRY_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplFileEntry {
    fn get_property_value(&self, _vm: &VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let entry = self.0.as_ref();
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "valid" => VariableValue::new_bool(entry.is_some()),
            "id" => VariableValue::new_long(entry.map_or(0, |entry| entry.id)),
            "name" => VariableValue::new_unbounded_string(entry.map_or_else(String::new, |entry| entry.name.clone())),
            "description" => VariableValue::new_unbounded_string(entry.map_or_else(String::new, |entry| entry.description.clone())),
            "size" => VariableValue::new_long(entry.map_or(0, |entry| entry.size)),
            "date" => VariableValue::new(
                VariableType::Date,
                VariableData::from_int(entry.map_or(0, |entry| IcbDate::from_utc(&entry.date).to_pcboard_date())),
            ),
            "descriptiontruncated" => VariableValue::new_bool(entry.is_some_and(|entry| entry.description_truncated)),
            _ => return Err(format!("Unknown FILEENTRY property {name}").into()),
        })
    }

    async fn set_property_value(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("FILEENTRY property {name} is read-only").into())
    }

    async fn call_function(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<VariableValue> {
        Err(format!("Unknown FILEENTRY function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown FILEENTRY method {name}").into())
    }
}

#[derive(Clone, Default)]
pub struct PplFilePage(Option<FilePage>);

impl PplFilePage {
    fn value(self) -> VariableValue {
        user_data_value(self, FILE_PAGE_ID)
    }
}

impl UserData for PplFilePage {
    const TYPE_NAME: &'static str = "FilePage";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| Self::default().value());

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(FILE_PAGE_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplFilePage {
    fn get_property_value(&self, _vm: &VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let page = self.0.as_ref();
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "valid" => VariableValue::new_bool(page.is_some()),
            "nextafter" => VariableValue::new_long(page.map_or(0, |page| page.next_after)),
            "hasmore" => VariableValue::new_bool(page.is_some_and(|page| page.has_more)),
            "entries" => VariableValue::new_vector(
                VariableType::UserData(FILE_ENTRY_ID as u32),
                page.into_iter()
                    .flat_map(|page| page.entries.iter())
                    .map(|entry| user_data_value(PplFileEntry(Some(entry.clone())), FILE_ENTRY_ID))
                    .collect(),
            ),
            _ => return Err(format!("Unknown FILEPAGE property {name}").into()),
        })
    }

    async fn set_property_value(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("FILEPAGE property {name} is read-only").into())
    }

    async fn call_function(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<VariableValue> {
        Err(format!("Unknown FILEPAGE function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown FILEPAGE method {name}").into())
    }
}

pub async fn find(directory: &FileDirectory, vm: &mut VirtualMachine<'_>, arguments: &[VariableValue]) -> crate::Res<VariableValue> {
    let Some(query) = arguments.first() else {
        vm.set_error(PplError::new(ERR_KIND_FILE, ERR_INVALID, "missing file search query"));
        return Ok(PplFilePage::default().value());
    };
    let query = query.as_string();
    let current = {
        let board = vm.icy_board_state.get_board().await;
        board.conferences.get(directory.conference_number).and_then(|conference| {
            let current = conference.directories.as_ref()?.get(directory.number)?;
            Some((
                current.clone(),
                conference.required_security.session_can_access(&vm.icy_board_state.session)
                    && current.list_security.session_can_access(&vm.icy_board_state.session),
            ))
        })
    };
    let after = arguments.get(1).map_or(0, VariableValue::as_long);
    let limit = arguments.get(2).map_or(15, VariableValue::as_int);
    let error = if !directory.valid
        || current
            .as_ref()
            .is_none_or(|(current, _)| current.path != directory.path || current.metadata_path != directory.metadata_path)
    {
        Some((ERR_INVALID, "invalid directory"))
    } else if !current.as_ref().is_some_and(|(_, allowed)| *allowed) {
        Some((ERR_DENIED, "directory access denied"))
    } else if after < 0 || limit < 1 {
        Some((ERR_INVALID, "invalid file search cursor or page size"))
    } else if limit as usize > reader::MAX_PAGE_SIZE || query.len() > 1024 {
        Some((ERR_LIMIT, "file search limit exceeded"))
    } else if !FileBase::database_path(&directory.metadata_path).is_file() {
        Some((ERR_UNAVAILABLE, "filebase index is not initialized"))
    } else {
        None
    };
    if let Some((code, message)) = error {
        vm.set_error(PplError::new(ERR_KIND_FILE, code, message));
        return Ok(PplFilePage::default().value());
    }
    let path = directory.path.clone();
    let metadata = directory.metadata_path.clone();
    let result = tokio::task::spawn_blocking(move || reader::read_page(&path, &metadata, &query, after, limit as usize)).await;
    match result {
        Ok(Ok(page)) => {
            vm.clear_error();
            Ok(PplFilePage(Some(page)).value())
        }
        error => {
            log::warn!("PPL filebase search failed: {error:?}");
            vm.set_error(PplError::new(ERR_KIND_FILE, ERR_IO, "unable to read filebase index"));
            Ok(PplFilePage::default().value())
        }
    }
}

pub async fn flag(directory: &FileDirectory, vm: &mut VirtualMachine<'_>, arguments: &[VariableValue]) -> crate::Res<VariableValue> {
    let result = flag_file(directory, vm, arguments).await;
    match result {
        Ok(()) => {
            vm.clear_error();
            Ok(VariableValue::new_bool(true))
        }
        Err(error) => {
            vm.set_error(error);
            Ok(VariableValue::new_bool(false))
        }
    }
}

async fn flag_file(directory: &FileDirectory, vm: &mut VirtualMachine<'_>, arguments: &[VariableValue]) -> Result<(), PplError> {
    let failure = |code, message| PplError::new(ERR_KIND_FILE, code, message);
    let name = arguments.first().map(VariableValue::as_string).unwrap_or_default();
    if arguments.len() != 1 || !reader::valid_file_name(&name) {
        return Err(failure(ERR_INVALID, "invalid file name"));
    }
    let path = directory.path.clone();
    let metadata = directory.metadata_path.clone();
    check_flag_access(directory, vm.icy_board_state).await?;
    if !FileBase::database_path(&metadata).is_file() {
        return Err(failure(ERR_UNAVAILABLE, "filebase index is not initialized"));
    }
    let file = match tokio::task::spawn_blocking(move || reader::resolve_file(&path, &metadata, &name)).await {
        Ok(Ok(Some(file))) => file,
        Ok(Ok(None)) => return Err(failure(ERR_UNAVAILABLE, "file is not available")),
        error => {
            log::warn!("PPL filebase flag lookup failed: {error:?}");
            return Err(failure(ERR_IO, "unable to read filebase index"));
        }
    };
    let state = &mut vm.icy_board_state;
    check_flag_access(directory, state).await?;
    if state.session.flagged_files.contains(&file) {
        return Ok(());
    }
    if state.session.flagged_files.len() >= state.session.batch_limit {
        return Err(failure(ERR_LIMIT, "batch limit reached"));
    }
    let io_error = |error| {
        log::warn!("PPL file flag failed: {error}");
        failure(ERR_IO, "unable to check file transfer cost")
    };
    let size = file.metadata().map_err(|_| failure(ERR_UNAVAILABLE, "file is not available"))?.len();
    let charge = state.accounting_download_estimate(&file, size).await.map_err(&io_error)?;
    let reserved = state.accounting_queued_download_cost(&file).await.map_err(&io_error)?;
    if state.accounting_credit_insufficient(charge, reserved).map_err(&io_error)? {
        return Err(failure(ERR_DENIED, "insufficient download credit"));
    }
    check_flag_access(directory, state).await?;
    state.session.flagged_files.push(file);
    Ok(())
}

async fn check_flag_access(directory: &FileDirectory, state: &super::IcyBoardState) -> Result<(), PplError> {
    let failure = |code, message| PplError::new(ERR_KIND_FILE, code, message);
    let board = state.get_board().await;
    let conference = board
        .conferences
        .get(directory.conference_number)
        .ok_or_else(|| failure(ERR_INVALID, "invalid directory"))?;
    let current = conference
        .directories
        .as_ref()
        .and_then(|directories| directories.get(directory.number))
        .ok_or_else(|| failure(ERR_INVALID, "invalid directory"))?;
    if !directory.valid || current.path != directory.path || current.metadata_path != directory.metadata_path {
        return Err(failure(ERR_INVALID, "invalid directory"));
    }
    if state.session.current_user.is_none()
        || !conference.required_security.session_can_access(&state.session)
        || !current.list_security.session_can_access(&state.session)
        || !current.download_security.session_can_access(&state.session)
    {
        return Err(failure(ERR_DENIED, "file access denied"));
    }
    Ok(())
}

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use async_trait::async_trait;
use icy_board_engine::{
    datetime::{IcbDoW, IcbTime},
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        conferences::{Conference, ConferenceBase, ConferenceType},
        icb_config::{DisplayNewsBehavior, IcbConfig, PasswordStorageMethod},
        security_expr::SecurityExpression,
        user_base::Password,
    },
};
use tokio::sync::Mutex;

use crate::{
    backup::{self, BoardLock},
    dto::*,
    error::{AdminError, Result},
};

/// Everything the web layer is allowed to do with a board. All configuration access
/// goes through the engine types; this layer only adds locking, backups and auditing.
pub struct AdminService {
    board_file: PathBuf,
    root_path: PathBuf,
}

#[async_trait]
pub trait AdminBackend: Send + Sync {
    fn board_file(&self) -> &Path;
    fn root_path(&self) -> &Path;
    async fn overview(&self) -> OverviewDto;

    async fn get_general_settings(&self) -> Result<GeneralSettingsResponse>;
    async fn preview_general_settings(&self, patch: &GeneralSettingsDto) -> Result<DiffDto>;
    async fn update_general_settings(&self, patch: &GeneralSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_message_settings(&self) -> Result<MessageSettingsResponse>;
    async fn preview_message_settings(&self, patch: &MessageSettingsDto) -> Result<DiffDto>;
    async fn update_message_settings(&self, patch: &MessageSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_file_transfer_settings(&self) -> Result<FileTransferSettingsResponse>;
    async fn preview_file_transfer_settings(&self, patch: &FileTransferSettingsDto) -> Result<DiffDto>;
    async fn update_file_transfer_settings(&self, patch: &FileTransferSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_system_control_settings(&self) -> Result<SystemControlSettingsResponse>;
    async fn preview_system_control_settings(&self, patch: &SystemControlSettingsDto) -> Result<DiffDto>;
    async fn update_system_control_settings(&self, patch: &SystemControlSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_switches_settings(&self) -> Result<SwitchesSettingsResponse>;
    async fn preview_switches_settings(&self, patch: &SwitchesSettingsDto) -> Result<DiffDto>;
    async fn update_switches_settings(&self, patch: &SwitchesSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_limits_settings(&self) -> Result<LimitsSettingsResponse>;
    async fn preview_limits_settings(&self, patch: &LimitsSettingsDto) -> Result<DiffDto>;
    async fn update_limits_settings(&self, patch: &LimitsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_new_user_settings(&self) -> Result<NewUserSettingsResponse>;
    async fn preview_new_user_settings(&self, patch: &NewUserSettingsDto) -> Result<DiffDto>;
    async fn update_new_user_settings(&self, patch: &NewUserSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_event_settings(&self) -> Result<EventSettingsResponse>;
    async fn preview_event_settings(&self, patch: &EventSettingsDto) -> Result<DiffDto>;
    async fn update_event_settings(&self, patch: &EventSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_subscription_settings(&self) -> Result<SubscriptionSettingsResponse>;
    async fn preview_subscription_settings(&self, patch: &SubscriptionSettingsDto) -> Result<DiffDto>;
    async fn update_subscription_settings(&self, patch: &SubscriptionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_connection_settings(&self) -> Result<ConnectionSettingsResponse>;
    async fn preview_connection_settings(&self, patch: &ConnectionSettingsDto) -> Result<DiffDto>;
    async fn update_connection_settings(&self, patch: &ConnectionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_paths_settings(&self) -> Result<PathsSettingsResponse>;
    async fn preview_paths_settings(&self, patch: &PathsSettingsDto) -> Result<DiffDto>;
    async fn update_paths_settings(&self, patch: &PathsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_accounting_settings(&self) -> Result<AccountingSettingsResponse>;
    async fn preview_accounting_settings(&self, patch: &AccountingSettingsDto) -> Result<DiffDto>;
    async fn update_accounting_settings(&self, patch: &AccountingSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn get_function_keys_settings(&self) -> Result<FunctionKeysSettingsResponse>;
    async fn preview_function_keys_settings(&self, patch: &FunctionKeysSettingsDto) -> Result<DiffDto>;
    async fn update_function_keys_settings(&self, patch: &FunctionKeysSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;

    async fn list_conferences(&self) -> Result<ConferenceListResponse>;
    async fn get_conference(&self, index: usize) -> Result<ConferenceResponse>;
    async fn create_conference(&self, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;
    async fn update_conference(&self, index: usize, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;
    async fn delete_conference(&self, index: usize, fingerprint: &str, actor: &str) -> Result<ApplyResultDto>;
}

impl AdminService {
    pub fn open<P: AsRef<Path>>(board_file: P) -> Result<Self> {
        let board_file = absolute(board_file.as_ref());
        if !board_file.is_file() {
            return Err(AdminError::NotFound(board_file));
        }
        let root_path = board_file.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        Ok(Self { board_file, root_path })
    }

    pub fn board_file(&self) -> &Path {
        &self.board_file
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn overview(&self) -> OverviewDto {
        let board = match IcyBoard::load(&self.board_file) {
            Ok(board) => board,
            Err(e) => {
                let mut dto = empty_overview(&self.board_file, &self.root_path);
                dto.load_error = Some(e.to_string());
                return dto;
            }
        };
        overview_from_board(&board, &self.board_file, &self.root_path)
    }

    fn load_config(&self) -> Result<IcbConfig> {
        IcbConfig::load(&self.board_file).map_err(|e| AdminError::Load(e.to_string()))
    }

    fn fingerprint(&self) -> Result<String> {
        backup::fingerprint(&self.board_file)
    }

    fn mutate_config<F>(&self, fingerprint: &str, actor: &str, action: &str, mutator: F) -> Result<ApplyResultDto>
    where
        F: FnOnce(&mut IcbConfig) -> Result<Vec<FieldChangeDto>>,
    {
        let _lock = BoardLock::acquire(&self.root_path)?;
        backup::check_fingerprint(&self.board_file, fingerprint)?;

        let mut config = self.load_config()?;
        let changes = mutator(&mut config)?;
        if changes.is_empty() {
            return Ok(ApplyResultDto {
                changed_fields: Vec::new(),
                backup: None,
                fingerprint: self.fingerprint()?,
            });
        }

        let backup_path = backup::create_backup(&self.root_path, &self.board_file)?;
        config.save_atomic(&self.board_file).map_err(|e| AdminError::Save(e.to_string()))?;

        if let Err(e) = IcbConfig::load(&self.board_file) {
            let _ = std::fs::copy(&backup_path, &self.board_file);
            return Err(AdminError::Save(format!(
                "written configuration could not be read back ({e}), the backup was restored"
            )));
        }

        let changed_fields: Vec<String> = changes.iter().map(|c| c.field.clone()).collect();
        backup::append_audit(
            &self.root_path,
            &serde_json::json!({
                "time": chrono::Utc::now().to_rfc3339(),
                "actor": actor,
                "action": action,
                "file": self.board_file.display().to_string(),
                "backup": backup_path.display().to_string(),
                "changes": changes.iter().map(|c| serde_json::json!({ "field": c.field, "old": c.old, "new": c.new })).collect::<Vec<_>>(),
            }),
        );

        Ok(ApplyResultDto {
            changed_fields,
            backup: Some(backup_path.display().to_string()),
            fingerprint: self.fingerprint()?,
        })
    }
}

// ---------------------------------------------------------------- section helpers (offline)

macro_rules! impl_offline_section {
    ($get:ident, $preview:ident, $update:ident, $dto:ident, $resp:ident, $to:ident, $apply:ident, $validate:ident, $diff:ident, $action:literal $(, $extra_field:ident = $extra_expr:expr )* ) => {
        impl AdminService {
            pub fn $get(&self) -> Result<$resp> {
                let config = self.load_config()?;
                Ok($resp {
                    settings: $to(&config),
                    $( $extra_field: $extra_expr, )*
                    fingerprint: self.fingerprint()?,
                })
            }

            pub fn $preview(&self, patch: &$dto) -> Result<DiffDto> {
                $validate(patch)?;
                let current = $to(&self.load_config()?);
                Ok(DiffDto {
                    changes: $diff(&current, patch),
                })
            }

            pub fn $update(&self, patch: &$dto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
                $validate(patch)?;
                self.mutate_config(fingerprint, actor, $action, |config| {
                    let current = $to(config);
                    let changes = $diff(&current, patch);
                    if !changes.is_empty() {
                        $apply(config, patch)?;
                    }
                    Ok(changes)
                })
            }
        }
    };
}

impl AdminService {
    pub fn get_general_settings(&self) -> Result<GeneralSettingsResponse> {
        let config = self.load_config()?;
        Ok(GeneralSettingsResponse {
            settings: to_general_dto(&config),
            sysop_password_set: !config.sysop.password.is_empty(),
            fingerprint: self.fingerprint()?,
        })
    }

    pub fn preview_general_settings(&self, patch: &GeneralSettingsDto) -> Result<DiffDto> {
        validate_general(patch)?;
        let current = to_general_dto(&self.load_config()?);
        Ok(DiffDto {
            changes: diff_general(&current, patch),
        })
    }

    pub fn update_general_settings(&self, patch: &GeneralSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_general(patch)?;
        self.mutate_config(fingerprint, actor, "update_general_settings", |config| {
            let current = to_general_dto(config);
            let changes = diff_general(&current, patch);
            if !changes.is_empty() {
                apply_general_dto(config, patch)?;
            }
            Ok(changes)
        })
    }
}

impl_offline_section!(
    get_message_settings,
    preview_message_settings,
    update_message_settings,
    MessageSettingsDto,
    MessageSettingsResponse,
    to_message_dto,
    apply_message_dto,
    validate_message,
    diff_message,
    "update_message_settings"
);

impl_offline_section!(
    get_file_transfer_settings,
    preview_file_transfer_settings,
    update_file_transfer_settings,
    FileTransferSettingsDto,
    FileTransferSettingsResponse,
    to_file_transfer_dto,
    apply_file_transfer_dto,
    validate_file_transfer,
    diff_file_transfer,
    "update_file_transfer_settings"
);

impl_offline_section!(
    get_system_control_settings,
    preview_system_control_settings,
    update_system_control_settings,
    SystemControlSettingsDto,
    SystemControlSettingsResponse,
    to_system_control_dto,
    apply_system_control_dto,
    validate_system_control,
    diff_system_control,
    "update_system_control_settings"
);

impl_offline_section!(
    get_switches_settings,
    preview_switches_settings,
    update_switches_settings,
    SwitchesSettingsDto,
    SwitchesSettingsResponse,
    to_switches_dto,
    apply_switches_dto,
    validate_switches,
    diff_switches,
    "update_switches_settings"
);

impl_offline_section!(
    get_limits_settings,
    preview_limits_settings,
    update_limits_settings,
    LimitsSettingsDto,
    LimitsSettingsResponse,
    to_limits_dto,
    apply_limits_dto,
    validate_limits,
    diff_limits,
    "update_limits_settings"
);

impl_offline_section!(
    get_new_user_settings,
    preview_new_user_settings,
    update_new_user_settings,
    NewUserSettingsDto,
    NewUserSettingsResponse,
    to_new_user_dto,
    apply_new_user_dto,
    validate_new_user,
    diff_new_user,
    "update_new_user_settings"
);

impl_offline_section!(
    get_event_settings,
    preview_event_settings,
    update_event_settings,
    EventSettingsDto,
    EventSettingsResponse,
    to_event_dto,
    apply_event_dto,
    validate_event,
    diff_event,
    "update_event_settings"
);

impl_offline_section!(
    get_subscription_settings,
    preview_subscription_settings,
    update_subscription_settings,
    SubscriptionSettingsDto,
    SubscriptionSettingsResponse,
    to_subscription_dto,
    apply_subscription_dto,
    validate_subscription,
    diff_subscription,
    "update_subscription_settings"
);

impl_offline_section!(
    get_connection_settings,
    preview_connection_settings,
    update_connection_settings,
    ConnectionSettingsDto,
    ConnectionSettingsResponse,
    to_connection_dto,
    apply_connection_dto,
    validate_connection,
    diff_connection,
    "update_connection_settings"
);

impl_offline_section!(
    get_paths_settings,
    preview_paths_settings,
    update_paths_settings,
    PathsSettingsDto,
    PathsSettingsResponse,
    to_paths_dto,
    apply_paths_dto,
    validate_paths,
    diff_paths,
    "update_paths_settings"
);

impl_offline_section!(
    get_accounting_settings,
    preview_accounting_settings,
    update_accounting_settings,
    AccountingSettingsDto,
    AccountingSettingsResponse,
    to_accounting_dto,
    apply_accounting_dto,
    validate_accounting,
    diff_accounting,
    "update_accounting_settings"
);

impl_offline_section!(
    get_function_keys_settings,
    preview_function_keys_settings,
    update_function_keys_settings,
    FunctionKeysSettingsDto,
    FunctionKeysSettingsResponse,
    to_function_keys_dto,
    apply_function_keys_dto,
    validate_function_keys,
    diff_function_keys,
    "update_function_keys_settings"
);

#[async_trait]
impl AdminBackend for AdminService {
    fn board_file(&self) -> &Path {
        AdminService::board_file(self)
    }
    fn root_path(&self) -> &Path {
        AdminService::root_path(self)
    }
    async fn overview(&self) -> OverviewDto {
        AdminService::overview(self)
    }

    async fn get_general_settings(&self) -> Result<GeneralSettingsResponse> {
        AdminService::get_general_settings(self)
    }
    async fn preview_general_settings(&self, patch: &GeneralSettingsDto) -> Result<DiffDto> {
        AdminService::preview_general_settings(self, patch)
    }
    async fn update_general_settings(&self, patch: &GeneralSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_general_settings(self, patch, fingerprint, actor)
    }

    async fn get_message_settings(&self) -> Result<MessageSettingsResponse> {
        AdminService::get_message_settings(self)
    }
    async fn preview_message_settings(&self, patch: &MessageSettingsDto) -> Result<DiffDto> {
        AdminService::preview_message_settings(self, patch)
    }
    async fn update_message_settings(&self, patch: &MessageSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_message_settings(self, patch, fingerprint, actor)
    }

    async fn get_file_transfer_settings(&self) -> Result<FileTransferSettingsResponse> {
        AdminService::get_file_transfer_settings(self)
    }
    async fn preview_file_transfer_settings(&self, patch: &FileTransferSettingsDto) -> Result<DiffDto> {
        AdminService::preview_file_transfer_settings(self, patch)
    }
    async fn update_file_transfer_settings(&self, patch: &FileTransferSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_file_transfer_settings(self, patch, fingerprint, actor)
    }

    async fn get_system_control_settings(&self) -> Result<SystemControlSettingsResponse> {
        AdminService::get_system_control_settings(self)
    }
    async fn preview_system_control_settings(&self, patch: &SystemControlSettingsDto) -> Result<DiffDto> {
        AdminService::preview_system_control_settings(self, patch)
    }
    async fn update_system_control_settings(&self, patch: &SystemControlSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_system_control_settings(self, patch, fingerprint, actor)
    }

    async fn get_switches_settings(&self) -> Result<SwitchesSettingsResponse> {
        AdminService::get_switches_settings(self)
    }
    async fn preview_switches_settings(&self, patch: &SwitchesSettingsDto) -> Result<DiffDto> {
        AdminService::preview_switches_settings(self, patch)
    }
    async fn update_switches_settings(&self, patch: &SwitchesSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_switches_settings(self, patch, fingerprint, actor)
    }

    async fn get_limits_settings(&self) -> Result<LimitsSettingsResponse> {
        AdminService::get_limits_settings(self)
    }
    async fn preview_limits_settings(&self, patch: &LimitsSettingsDto) -> Result<DiffDto> {
        AdminService::preview_limits_settings(self, patch)
    }
    async fn update_limits_settings(&self, patch: &LimitsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_limits_settings(self, patch, fingerprint, actor)
    }

    async fn get_new_user_settings(&self) -> Result<NewUserSettingsResponse> {
        AdminService::get_new_user_settings(self)
    }
    async fn preview_new_user_settings(&self, patch: &NewUserSettingsDto) -> Result<DiffDto> {
        AdminService::preview_new_user_settings(self, patch)
    }
    async fn update_new_user_settings(&self, patch: &NewUserSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_new_user_settings(self, patch, fingerprint, actor)
    }

    async fn get_event_settings(&self) -> Result<EventSettingsResponse> {
        AdminService::get_event_settings(self)
    }
    async fn preview_event_settings(&self, patch: &EventSettingsDto) -> Result<DiffDto> {
        AdminService::preview_event_settings(self, patch)
    }
    async fn update_event_settings(&self, patch: &EventSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_event_settings(self, patch, fingerprint, actor)
    }

    async fn get_subscription_settings(&self) -> Result<SubscriptionSettingsResponse> {
        AdminService::get_subscription_settings(self)
    }
    async fn preview_subscription_settings(&self, patch: &SubscriptionSettingsDto) -> Result<DiffDto> {
        AdminService::preview_subscription_settings(self, patch)
    }
    async fn update_subscription_settings(&self, patch: &SubscriptionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_subscription_settings(self, patch, fingerprint, actor)
    }

    async fn get_connection_settings(&self) -> Result<ConnectionSettingsResponse> {
        AdminService::get_connection_settings(self)
    }
    async fn preview_connection_settings(&self, patch: &ConnectionSettingsDto) -> Result<DiffDto> {
        AdminService::preview_connection_settings(self, patch)
    }
    async fn update_connection_settings(&self, patch: &ConnectionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_connection_settings(self, patch, fingerprint, actor)
    }

    async fn get_paths_settings(&self) -> Result<PathsSettingsResponse> {
        AdminService::get_paths_settings(self)
    }
    async fn preview_paths_settings(&self, patch: &PathsSettingsDto) -> Result<DiffDto> {
        AdminService::preview_paths_settings(self, patch)
    }
    async fn update_paths_settings(&self, patch: &PathsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_paths_settings(self, patch, fingerprint, actor)
    }

    async fn get_accounting_settings(&self) -> Result<AccountingSettingsResponse> {
        AdminService::get_accounting_settings(self)
    }
    async fn preview_accounting_settings(&self, patch: &AccountingSettingsDto) -> Result<DiffDto> {
        AdminService::preview_accounting_settings(self, patch)
    }
    async fn update_accounting_settings(&self, patch: &AccountingSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_accounting_settings(self, patch, fingerprint, actor)
    }

    async fn get_function_keys_settings(&self) -> Result<FunctionKeysSettingsResponse> {
        AdminService::get_function_keys_settings(self)
    }
    async fn preview_function_keys_settings(&self, patch: &FunctionKeysSettingsDto) -> Result<DiffDto> {
        AdminService::preview_function_keys_settings(self, patch)
    }
    async fn update_function_keys_settings(&self, patch: &FunctionKeysSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_function_keys_settings(self, patch, fingerprint, actor)
    }

    async fn list_conferences(&self) -> Result<ConferenceListResponse> {
        AdminService::list_conferences(self)
    }
    async fn get_conference(&self, index: usize) -> Result<ConferenceResponse> {
        AdminService::get_conference(self, index)
    }
    async fn create_conference(&self, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::create_conference(self, patch, fingerprint, actor)
    }
    async fn update_conference(&self, index: usize, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::update_conference(self, index, patch, fingerprint, actor)
    }
    async fn delete_conference(&self, index: usize, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        AdminService::delete_conference(self, index, fingerprint, actor)
    }
}

// ---------------------------------------------------------------- live backend

pub struct LiveAdminBackend {
    board: Arc<Mutex<IcyBoard>>,
    board_file: PathBuf,
    root_path: PathBuf,
}

impl LiveAdminBackend {
    pub fn new<P: AsRef<Path>>(board_file: P, board: Arc<Mutex<IcyBoard>>) -> Result<Self> {
        let board_file = absolute(board_file.as_ref());
        if !board_file.is_file() {
            return Err(AdminError::NotFound(board_file));
        }
        let root_path = board_file.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        Ok(Self {
            board,
            board_file,
            root_path,
        })
    }

    async fn mutate_live_fn<F>(&self, fingerprint: &str, actor: &str, action: &str, mutator: F) -> Result<ApplyResultDto>
    where
        F: Fn(&mut IcbConfig) -> Result<Vec<FieldChangeDto>> + Send,
    {
        let _file_lock = BoardLock::acquire(&self.root_path)?;
        backup::check_fingerprint(&self.board_file, fingerprint)?;
        let mut board = self.board.lock().await;

        let changes = mutator(&mut board.config)?;
        if changes.is_empty() {
            return Ok(ApplyResultDto {
                changed_fields: Vec::new(),
                backup: None,
                fingerprint: backup::fingerprint(&self.board_file)?,
            });
        }

        let mut disk_config = IcbConfig::load(&self.board_file).map_err(|e| AdminError::Load(e.to_string()))?;
        // Apply the same section mutation onto the disk image.
        let _ = mutator(&mut disk_config)?;

        let backup_path = backup::create_backup(&self.root_path, &self.board_file)?;
        disk_config
            .save_atomic(&self.board_file)
            .map_err(|e| AdminError::Save(e.to_string()))?;
        if let Err(e) = IcbConfig::load(&self.board_file) {
            let _ = std::fs::copy(&backup_path, &self.board_file);
            return Err(AdminError::Save(format!(
                "written configuration could not be read back ({e}), the backup was restored"
            )));
        }

        let changed_fields: Vec<String> = changes.iter().map(|c| c.field.clone()).collect();
        backup::append_audit(
            &self.root_path,
            &serde_json::json!({
                "time": chrono::Utc::now().to_rfc3339(),
                "actor": actor,
                "action": action,
                "mode": "live",
                "file": self.board_file.display().to_string(),
                "backup": backup_path.display().to_string(),
                "changes": changes.iter().map(|c| serde_json::json!({ "field": c.field, "old": c.old, "new": c.new })).collect::<Vec<_>>(),
            }),
        );

        Ok(ApplyResultDto {
            changed_fields,
            backup: Some(backup_path.display().to_string()),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
}

#[async_trait]
impl AdminBackend for LiveAdminBackend {
    fn board_file(&self) -> &Path {
        &self.board_file
    }
    fn root_path(&self) -> &Path {
        &self.root_path
    }
    async fn overview(&self) -> OverviewDto {
        let board = self.board.lock().await;
        overview_from_board(&board, &self.board_file, &self.root_path)
    }

    async fn get_general_settings(&self) -> Result<GeneralSettingsResponse> {
        let board = self.board.lock().await;
        Ok(GeneralSettingsResponse {
            settings: to_general_dto(&board.config),
            sysop_password_set: !board.config.sysop.password.is_empty(),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_general_settings(&self, patch: &GeneralSettingsDto) -> Result<DiffDto> {
        validate_general(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_general(&to_general_dto(&board.config), patch),
        })
    }
    async fn update_general_settings(&self, patch: &GeneralSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_general(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_general_settings", move |config| {
            let changes = diff_general(&to_general_dto(config), &patch);
            if !changes.is_empty() {
                apply_general_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_message_settings(&self) -> Result<MessageSettingsResponse> {
        let board = self.board.lock().await;
        Ok(MessageSettingsResponse {
            settings: to_message_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_message_settings(&self, patch: &MessageSettingsDto) -> Result<DiffDto> {
        validate_message(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_message(&to_message_dto(&board.config), patch),
        })
    }
    async fn update_message_settings(&self, patch: &MessageSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_message(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_message_settings", move |config| {
            let changes = diff_message(&to_message_dto(config), &patch);
            if !changes.is_empty() {
                apply_message_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_file_transfer_settings(&self) -> Result<FileTransferSettingsResponse> {
        let board = self.board.lock().await;
        Ok(FileTransferSettingsResponse {
            settings: to_file_transfer_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_file_transfer_settings(&self, patch: &FileTransferSettingsDto) -> Result<DiffDto> {
        validate_file_transfer(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_file_transfer(&to_file_transfer_dto(&board.config), patch),
        })
    }
    async fn update_file_transfer_settings(&self, patch: &FileTransferSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_file_transfer(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_file_transfer_settings", move |config| {
            let changes = diff_file_transfer(&to_file_transfer_dto(config), &patch);
            if !changes.is_empty() {
                apply_file_transfer_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_system_control_settings(&self) -> Result<SystemControlSettingsResponse> {
        let board = self.board.lock().await;
        Ok(SystemControlSettingsResponse {
            settings: to_system_control_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_system_control_settings(&self, patch: &SystemControlSettingsDto) -> Result<DiffDto> {
        validate_system_control(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_system_control(&to_system_control_dto(&board.config), patch),
        })
    }
    async fn update_system_control_settings(&self, patch: &SystemControlSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_system_control(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_system_control_settings", move |config| {
            let changes = diff_system_control(&to_system_control_dto(config), &patch);
            if !changes.is_empty() {
                apply_system_control_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_switches_settings(&self) -> Result<SwitchesSettingsResponse> {
        let board = self.board.lock().await;
        Ok(SwitchesSettingsResponse {
            settings: to_switches_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_switches_settings(&self, patch: &SwitchesSettingsDto) -> Result<DiffDto> {
        validate_switches(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_switches(&to_switches_dto(&board.config), patch),
        })
    }
    async fn update_switches_settings(&self, patch: &SwitchesSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_switches(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_switches_settings", move |config| {
            let changes = diff_switches(&to_switches_dto(config), &patch);
            if !changes.is_empty() {
                apply_switches_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_limits_settings(&self) -> Result<LimitsSettingsResponse> {
        let board = self.board.lock().await;
        Ok(LimitsSettingsResponse {
            settings: to_limits_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_limits_settings(&self, patch: &LimitsSettingsDto) -> Result<DiffDto> {
        validate_limits(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_limits(&to_limits_dto(&board.config), patch),
        })
    }
    async fn update_limits_settings(&self, patch: &LimitsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_limits(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_limits_settings", move |config| {
            let changes = diff_limits(&to_limits_dto(config), &patch);
            if !changes.is_empty() {
                apply_limits_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_new_user_settings(&self) -> Result<NewUserSettingsResponse> {
        let board = self.board.lock().await;
        Ok(NewUserSettingsResponse {
            settings: to_new_user_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_new_user_settings(&self, patch: &NewUserSettingsDto) -> Result<DiffDto> {
        validate_new_user(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_new_user(&to_new_user_dto(&board.config), patch),
        })
    }
    async fn update_new_user_settings(&self, patch: &NewUserSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_new_user(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_new_user_settings", move |config| {
            let changes = diff_new_user(&to_new_user_dto(config), &patch);
            if !changes.is_empty() {
                apply_new_user_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_event_settings(&self) -> Result<EventSettingsResponse> {
        let board = self.board.lock().await;
        Ok(EventSettingsResponse {
            settings: to_event_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_event_settings(&self, patch: &EventSettingsDto) -> Result<DiffDto> {
        validate_event(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_event(&to_event_dto(&board.config), patch),
        })
    }
    async fn update_event_settings(&self, patch: &EventSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_event(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_event_settings", move |config| {
            let changes = diff_event(&to_event_dto(config), &patch);
            if !changes.is_empty() {
                apply_event_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_subscription_settings(&self) -> Result<SubscriptionSettingsResponse> {
        let board = self.board.lock().await;
        Ok(SubscriptionSettingsResponse {
            settings: to_subscription_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_subscription_settings(&self, patch: &SubscriptionSettingsDto) -> Result<DiffDto> {
        validate_subscription(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_subscription(&to_subscription_dto(&board.config), patch),
        })
    }
    async fn update_subscription_settings(&self, patch: &SubscriptionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_subscription(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_subscription_settings", move |config| {
            let changes = diff_subscription(&to_subscription_dto(config), &patch);
            if !changes.is_empty() {
                apply_subscription_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_connection_settings(&self) -> Result<ConnectionSettingsResponse> {
        let board = self.board.lock().await;
        Ok(ConnectionSettingsResponse {
            settings: to_connection_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_connection_settings(&self, patch: &ConnectionSettingsDto) -> Result<DiffDto> {
        validate_connection(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_connection(&to_connection_dto(&board.config), patch),
        })
    }
    async fn update_connection_settings(&self, patch: &ConnectionSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_connection(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_connection_settings", move |config| {
            let changes = diff_connection(&to_connection_dto(config), &patch);
            if !changes.is_empty() {
                apply_connection_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_paths_settings(&self) -> Result<PathsSettingsResponse> {
        let board = self.board.lock().await;
        Ok(PathsSettingsResponse {
            settings: to_paths_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_paths_settings(&self, patch: &PathsSettingsDto) -> Result<DiffDto> {
        validate_paths(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_paths(&to_paths_dto(&board.config), patch),
        })
    }
    async fn update_paths_settings(&self, patch: &PathsSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_paths(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_paths_settings", move |config| {
            let changes = diff_paths(&to_paths_dto(config), &patch);
            if !changes.is_empty() {
                apply_paths_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_accounting_settings(&self) -> Result<AccountingSettingsResponse> {
        let board = self.board.lock().await;
        Ok(AccountingSettingsResponse {
            settings: to_accounting_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_accounting_settings(&self, patch: &AccountingSettingsDto) -> Result<DiffDto> {
        validate_accounting(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_accounting(&to_accounting_dto(&board.config), patch),
        })
    }
    async fn update_accounting_settings(&self, patch: &AccountingSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_accounting(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_accounting_settings", move |config| {
            let changes = diff_accounting(&to_accounting_dto(config), &patch);
            if !changes.is_empty() {
                apply_accounting_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn get_function_keys_settings(&self) -> Result<FunctionKeysSettingsResponse> {
        let board = self.board.lock().await;
        Ok(FunctionKeysSettingsResponse {
            settings: to_function_keys_dto(&board.config),
            fingerprint: backup::fingerprint(&self.board_file)?,
        })
    }
    async fn preview_function_keys_settings(&self, patch: &FunctionKeysSettingsDto) -> Result<DiffDto> {
        validate_function_keys(patch)?;
        let board = self.board.lock().await;
        Ok(DiffDto {
            changes: diff_function_keys(&to_function_keys_dto(&board.config), patch),
        })
    }
    async fn update_function_keys_settings(&self, patch: &FunctionKeysSettingsDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        validate_function_keys(patch)?;
        let patch = patch.clone();
        self.mutate_live_fn(fingerprint, actor, "update_function_keys_settings", move |config| {
            let changes = diff_function_keys(&to_function_keys_dto(config), &patch);
            if !changes.is_empty() {
                apply_function_keys_dto(config, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn list_conferences(&self) -> Result<ConferenceListResponse> {
        let board = self.board.lock().await;
        let path = self.live_conferences_path(&board.config)?;
        Ok(ConferenceListResponse {
            conferences: conference_summaries(&board.conferences),
            file: path.display().to_string(),
            fingerprint: backup::fingerprint(&path)?,
        })
    }

    async fn get_conference(&self, index: usize) -> Result<ConferenceResponse> {
        let board = self.board.lock().await;
        let path = self.live_conferences_path(&board.config)?;
        let conf = conference_at(&board.conferences, index)?;
        Ok(ConferenceResponse {
            index,
            settings: to_conference_dto(conf),
            password_set: !conf.password.is_empty(),
            file: path.display().to_string(),
            fingerprint: backup::fingerprint(&path)?,
        })
    }

    async fn create_conference(&self, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        let patch = normalize_conference(patch)?;
        self.mutate_conferences_live(fingerprint, actor, "create_conference", move |base| {
            let mut conf = Conference::default();
            apply_conference_dto(&mut conf, &patch)?;
            let index = base.len();
            base.push(conf);
            Ok(vec![FieldChangeDto {
                field: format!("conference[{index}]"),
                old: String::new(),
                new: patch.name.clone(),
            }])
        })
        .await
    }

    async fn update_conference(&self, index: usize, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        let patch = normalize_conference(patch)?;
        self.mutate_conferences_live(fingerprint, actor, "update_conference", move |base| {
            let conf = base
                .get_mut(index)
                .ok_or_else(|| AdminError::Missing(format!("conference {index} does not exist")))?;
            let changes = diff_conference(&to_conference_dto(conf), &patch);
            if !changes.is_empty() {
                apply_conference_dto(conf, &patch)?;
            }
            Ok(changes)
        })
        .await
    }

    async fn delete_conference(&self, index: usize, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        self.mutate_conferences_live(fingerprint, actor, "delete_conference", move |base| {
            if index >= base.len() {
                return Err(AdminError::Missing(format!("conference {index} does not exist")));
            }
            if base.len() == 1 {
                return Err(AdminError::Validation(vec!["The last conference cannot be deleted".to_string()]));
            }
            let removed = base.remove(index);
            Ok(vec![FieldChangeDto {
                field: format!("conference[{index}]"),
                old: removed.name.clone(),
                new: String::new(),
            }])
        })
        .await
    }
}

// ---------------------------------------------------------------- overview helpers

fn empty_overview(board_file: &Path, root_path: &Path) -> OverviewDto {
    OverviewDto {
        board_file: board_file.display().to_string(),
        root_path: root_path.display().to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        config_loaded: false,
        load_error: None,
        board_name: None,
        sysop_name: None,
        num_nodes: None,
        counts: None,
        statistics: None,
        paths: Vec::new(),
        warnings: Vec::new(),
    }
}

fn overview_from_board(board: &IcyBoard, board_file: &Path, root_path: &Path) -> OverviewDto {
    let mut dto = empty_overview(board_file, root_path);
    dto.config_loaded = true;
    dto.board_name = Some(board.config.board.name.clone());
    dto.sysop_name = Some(board.config.sysop.name.clone());
    dto.num_nodes = Some(board.config.board.num_nodes);
    dto.counts = Some(CountsDto {
        conferences: board.conferences.len(),
        users: board.users.len(),
        security_levels: board.sec_levels.len(),
        commands: board.commands.commands.len(),
        languages: board.languages.len(),
        protocols: board.protocols.len(),
    });
    dto.statistics = Some(StatisticsDto {
        today_calls: board.statistics.today.calls,
        today_messages: board.statistics.today.messages,
        today_uploads: board.statistics.today.uploads,
        today_downloads: board.statistics.today.downloads,
        total_calls: board.statistics.total.calls,
        total_messages: board.statistics.total.messages,
        total_uploads: board.statistics.total.uploads,
        total_downloads: board.statistics.total.downloads,
    });
    dto.paths = path_checks(&board.config, root_path);

    for check in &dto.paths {
        if !check.exists && check.expected != PathKind::Unset {
            dto.warnings.push(format!("{} is missing: {}", check.label, check.path));
        }
    }
    if board.config.board.name.trim().is_empty() {
        dto.warnings.push("The board has no name.".to_string());
    }
    if board.config.sysop.password.is_empty() {
        dto.warnings.push("No sysop password is set.".to_string());
    }
    if board.conferences.is_empty() {
        dto.warnings.push("No conferences are defined.".to_string());
    }
    dto
}

fn path_checks(config: &IcbConfig, root_path: &Path) -> Vec<PathCheckDto> {
    let paths = &config.paths;
    let entries: Vec<(&str, &PathBuf, PathKind)> = vec![
        ("Conferences", &paths.conferences, PathKind::File),
        ("User file", &paths.user_file, PathKind::File),
        ("Display text", &paths.icbtext, PathKind::File),
        ("Command file", &paths.command_file, PathKind::File),
        ("Language file", &paths.language_file, PathKind::File),
        ("Protocol file", &paths.protocol_data_file, PathKind::File),
        ("Security levels", &paths.pwrd_sec_level_file, PathKind::File),
        ("Group file", &paths.group_file, PathKind::File),
        ("Statistics", &paths.statistics_file, PathKind::File),
        ("Help files", &paths.help_path, PathKind::Directory),
        ("Security messages", &paths.security_file_path, PathKind::Directory),
        ("Command display files", &paths.command_display_path, PathKind::Directory),
        ("Temporary work directory", &paths.tmp_work_path, PathKind::Directory),
        ("E-Mail message base", &paths.email_msgbase, PathKind::Directory),
    ];

    entries
        .into_iter()
        .map(|(label, path, kind)| {
            if path.as_os_str().is_empty() {
                return PathCheckDto {
                    label: label.to_string(),
                    path: "<not set>".to_string(),
                    exists: false,
                    expected: PathKind::Unset,
                };
            }
            let resolved = if path.is_absolute() {
                path.clone()
            } else {
                root_path.join(path)
            };
            let exists = match kind {
                PathKind::File => resolved.is_file(),
                PathKind::Directory => resolved.is_dir(),
                PathKind::Unset => true,
            };
            PathCheckDto {
                label: label.to_string(),
                path: resolved.display().to_string(),
                exists,
                expected: kind,
            }
        })
        .collect()
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cur) => cur.join(path),
        Err(_) => path.to_path_buf(),
    }
}

// ---------------------------------------------------------------- conversion helpers

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn set_path(target: &mut PathBuf, value: &str) {
    *target = PathBuf::from(value.trim());
}

fn time_string(time: &IcbTime) -> String {
    if time.is_empty() {
        String::new()
    } else {
        time.to_string()
    }
}

fn parse_time_field(label: &str, value: &str, errors: &mut Vec<String>) -> IcbTime {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return IcbTime::default();
    }
    let normalized = if trimmed.matches(':').count() == 1 {
        format!("{trimmed}:00")
    } else {
        trimmed.to_string()
    };
    let parsed = IcbTime::parse(&normalized);
    // IcbTime::parse returns 00:00:00 for bad input - distinguish empty already handled.
    if normalized != parsed.to_string() && !(parsed.is_empty() && normalized == "00:00:00") {
        // Accept parse if components look valid.
        let parts: Vec<_> = normalized.split(':').collect();
        if parts.len() != 3
            || parts[0].parse::<u8>().ok().filter(|h| *h <= 23).is_none()
            || parts[1].parse::<u8>().ok().filter(|m| *m <= 59).is_none()
            || parts[2].parse::<u8>().ok().filter(|s| *s <= 59).is_none()
        {
            errors.push(format!("{label} must be HH:MM or HH:MM:SS."));
        }
    }
    parsed
}

fn password_storage_to_str(method: PasswordStorageMethod) -> String {
    match method {
        PasswordStorageMethod::BCrypt => "bcrypt".to_string(),
        PasswordStorageMethod::Argon2 => "argon2".to_string(),
        PasswordStorageMethod::PlainText => "plain".to_string(),
    }
}

fn password_storage_from_str(value: &str) -> Option<PasswordStorageMethod> {
    match value {
        "bcrypt" => Some(PasswordStorageMethod::BCrypt),
        "argon2" => Some(PasswordStorageMethod::Argon2),
        "plain" => Some(PasswordStorageMethod::PlainText),
        _ => None,
    }
}

fn news_to_str(behavior: DisplayNewsBehavior) -> String {
    behavior.to_pcb_char().to_string()
}

fn news_from_str(value: &str) -> Option<DisplayNewsBehavior> {
    let c = value.chars().next()?;
    if !matches!(c, 'Y' | 'N' | 'A' | 'X') {
        return None;
    }
    Some(DisplayNewsBehavior::from_pcb_char(c))
}

fn push_change(changes: &mut Vec<FieldChangeDto>, field: &str, old: String, new: String) {
    if old != new {
        changes.push(FieldChangeDto {
            field: field.to_string(),
            old,
            new,
        });
    }
}

fn check_text(label: &str, value: &str, max_len: usize, required: bool, errors: &mut Vec<String>) {
    let trimmed = value.trim();
    if required && trimmed.is_empty() {
        errors.push(format!("{label} must not be empty."));
    }
    if trimmed.chars().count() > max_len {
        errors.push(format!("{label} must not be longer than {max_len} characters."));
    }
    if value.chars().any(|c| c.is_control()) {
        errors.push(format!("{label} must not contain control characters."));
    }
}

fn check_path_text(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.chars().any(|c| c.is_control()) {
        errors.push(format!("{label} must not contain control characters."));
    }
    if value.trim().chars().count() > 512 {
        errors.push(format!("{label} must not be longer than 512 characters."));
    }
}

// ---- general ----

fn to_general_dto(config: &IcbConfig) -> GeneralSettingsDto {
    GeneralSettingsDto {
        board_name: config.board.name.clone(),
        location: config.board.location.clone(),
        operator: config.board.operator.clone(),
        notice: config.board.notice.clone(),
        capabilities: config.board.capabilities.clone(),
        date_format: config.board.date_format.clone(),
        num_nodes: config.board.num_nodes,
        allow_iemsi: config.board.allow_iemsi,
        who_include_city: config.board.who_include_city,
        who_show_alias: config.board.who_show_alias,
        sysop_name: config.sysop.name.clone(),
        sysop_use_real_name: config.sysop.use_real_name,
        sysop_require_password_to_exit: config.sysop.require_password_to_exit,
        sysop_external_editor: config.sysop.external_editor.clone(),
        sysop_config_color_theme: config.sysop.config_color_theme.clone(),
        web_admin_enabled: config.board.web_admin.enabled,
        web_admin_address: config.board.web_admin.address.clone(),
        web_admin_port: config.board.web_admin.port,
        web_admin_allow_remote: config.board.web_admin.allow_remote,
    }
}

fn apply_general_dto(config: &mut IcbConfig, dto: &GeneralSettingsDto) -> Result<()> {
    config.board.name = dto.board_name.trim().to_string();
    config.board.location = dto.location.trim().to_string();
    config.board.operator = dto.operator.trim().to_string();
    config.board.notice = dto.notice.trim().to_string();
    config.board.capabilities = dto.capabilities.trim().to_string();
    config.board.date_format = dto.date_format.clone();
    config.board.num_nodes = dto.num_nodes;
    config.board.allow_iemsi = dto.allow_iemsi;
    config.board.who_include_city = dto.who_include_city;
    config.board.who_show_alias = dto.who_show_alias;
    config.sysop.name = dto.sysop_name.trim().to_string();
    config.sysop.use_real_name = dto.sysop_use_real_name;
    config.sysop.require_password_to_exit = dto.sysop_require_password_to_exit;
    config.sysop.external_editor = dto.sysop_external_editor.trim().to_string();
    config.sysop.config_color_theme = dto.sysop_config_color_theme.trim().to_string();
    config.board.web_admin.enabled = dto.web_admin_enabled;
    config.board.web_admin.address = dto.web_admin_address.trim().to_string();
    config.board.web_admin.port = dto.web_admin_port;
    config.board.web_admin.allow_remote = dto.web_admin_allow_remote;
    Ok(())
}

fn validate_general(dto: &GeneralSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    check_text("Board name", &dto.board_name, 45, true, &mut errors);
    check_text("Location", &dto.location, 54, false, &mut errors);
    check_text("Operator", &dto.operator, 30, false, &mut errors);
    check_text("Notice", &dto.notice, 30, false, &mut errors);
    check_text("Capabilities", &dto.capabilities, 30, false, &mut errors);
    check_text("Sysop name", &dto.sysop_name, 30, true, &mut errors);
    check_text("External editor", &dto.sysop_external_editor, 128, false, &mut errors);
    check_text("Color theme", &dto.sysop_config_color_theme, 64, false, &mut errors);
    check_text("Web admin address", &dto.web_admin_address, 64, true, &mut errors);
    if !DATE_FORMATS.iter().any(|(value, _)| *value == dto.date_format) {
        errors.push(format!("Date format '{}' is not one of the supported formats.", dto.date_format));
    }
    if !(1..=256).contains(&dto.num_nodes) {
        errors.push("Number of nodes must be between 1 and 256.".to_string());
    }
    if dto.web_admin_port == 0 {
        errors.push("Web admin port must be between 1 and 65535.".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_general(old: &GeneralSettingsDto, new: &GeneralSettingsDto) -> Vec<FieldChangeDto> {
    let mut changes = Vec::new();
    push_change(&mut changes, "board_name", old.board_name.clone(), new.board_name.trim().to_string());
    push_change(&mut changes, "location", old.location.clone(), new.location.trim().to_string());
    push_change(&mut changes, "operator", old.operator.clone(), new.operator.trim().to_string());
    push_change(&mut changes, "notice", old.notice.clone(), new.notice.trim().to_string());
    push_change(
        &mut changes,
        "capabilities",
        old.capabilities.clone(),
        new.capabilities.trim().to_string(),
    );
    push_change(&mut changes, "date_format", old.date_format.clone(), new.date_format.clone());
    push_change(&mut changes, "num_nodes", old.num_nodes.to_string(), new.num_nodes.to_string());
    push_change(&mut changes, "allow_iemsi", old.allow_iemsi.to_string(), new.allow_iemsi.to_string());
    push_change(
        &mut changes,
        "who_include_city",
        old.who_include_city.to_string(),
        new.who_include_city.to_string(),
    );
    push_change(
        &mut changes,
        "who_show_alias",
        old.who_show_alias.to_string(),
        new.who_show_alias.to_string(),
    );
    push_change(&mut changes, "sysop_name", old.sysop_name.clone(), new.sysop_name.trim().to_string());
    push_change(
        &mut changes,
        "sysop_use_real_name",
        old.sysop_use_real_name.to_string(),
        new.sysop_use_real_name.to_string(),
    );
    push_change(
        &mut changes,
        "sysop_require_password_to_exit",
        old.sysop_require_password_to_exit.to_string(),
        new.sysop_require_password_to_exit.to_string(),
    );
    push_change(
        &mut changes,
        "sysop_external_editor",
        old.sysop_external_editor.clone(),
        new.sysop_external_editor.trim().to_string(),
    );
    push_change(
        &mut changes,
        "sysop_config_color_theme",
        old.sysop_config_color_theme.clone(),
        new.sysop_config_color_theme.trim().to_string(),
    );
    push_change(
        &mut changes,
        "web_admin_enabled",
        old.web_admin_enabled.to_string(),
        new.web_admin_enabled.to_string(),
    );
    push_change(
        &mut changes,
        "web_admin_address",
        old.web_admin_address.clone(),
        new.web_admin_address.trim().to_string(),
    );
    push_change(
        &mut changes,
        "web_admin_port",
        old.web_admin_port.to_string(),
        new.web_admin_port.to_string(),
    );
    push_change(
        &mut changes,
        "web_admin_allow_remote",
        old.web_admin_allow_remote.to_string(),
        new.web_admin_allow_remote.to_string(),
    );
    changes
}

// ---- message ----

fn to_message_dto(config: &IcbConfig) -> MessageSettingsDto {
    MessageSettingsDto {
        max_msg_lines: config.message.max_msg_lines,
        scan_all_mail_at_login: config.message.scan_all_mail_at_login,
        disable_message_scan_prompt: config.message.disable_message_scan_prompt,
        allow_esc_codes: config.message.allow_esc_codes,
        allow_carbon_copy: config.message.allow_carbon_copy,
        validate_to_name: config.message.validate_to_name,
        default_quick_personal_scan: config.message.default_quick_personal_scan,
        default_scan_all_selected_confs_at_login: config.message.default_scan_all_selected_confs_at_login,
        prompt_to_read_mail: config.message.prompt_to_read_mail,
        force_comments_to_main: config.message.force_comments_to_main,
        update_last_read_pointer: config.message.update_last_read_pointer,
    }
}

fn apply_message_dto(config: &mut IcbConfig, dto: &MessageSettingsDto) -> Result<()> {
    config.message.max_msg_lines = dto.max_msg_lines;
    config.message.scan_all_mail_at_login = dto.scan_all_mail_at_login;
    config.message.disable_message_scan_prompt = dto.disable_message_scan_prompt;
    config.message.allow_esc_codes = dto.allow_esc_codes;
    config.message.allow_carbon_copy = dto.allow_carbon_copy;
    config.message.validate_to_name = dto.validate_to_name;
    config.message.default_quick_personal_scan = dto.default_quick_personal_scan;
    config.message.default_scan_all_selected_confs_at_login = dto.default_scan_all_selected_confs_at_login;
    config.message.prompt_to_read_mail = dto.prompt_to_read_mail;
    config.message.force_comments_to_main = dto.force_comments_to_main;
    config.message.update_last_read_pointer = dto.update_last_read_pointer;
    Ok(())
}

fn validate_message(dto: &MessageSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    if !(1..=500).contains(&dto.max_msg_lines) {
        errors.push("Max message lines must be between 1 and 500.".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_message(old: &MessageSettingsDto, new: &MessageSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(&mut c, "max_msg_lines", old.max_msg_lines.to_string(), new.max_msg_lines.to_string());
    push_change(
        &mut c,
        "scan_all_mail_at_login",
        old.scan_all_mail_at_login.to_string(),
        new.scan_all_mail_at_login.to_string(),
    );
    push_change(
        &mut c,
        "disable_message_scan_prompt",
        old.disable_message_scan_prompt.to_string(),
        new.disable_message_scan_prompt.to_string(),
    );
    push_change(&mut c, "allow_esc_codes", old.allow_esc_codes.to_string(), new.allow_esc_codes.to_string());
    push_change(
        &mut c,
        "allow_carbon_copy",
        old.allow_carbon_copy.to_string(),
        new.allow_carbon_copy.to_string(),
    );
    push_change(
        &mut c,
        "validate_to_name",
        old.validate_to_name.to_string(),
        new.validate_to_name.to_string(),
    );
    push_change(
        &mut c,
        "default_quick_personal_scan",
        old.default_quick_personal_scan.to_string(),
        new.default_quick_personal_scan.to_string(),
    );
    push_change(
        &mut c,
        "default_scan_all_selected_confs_at_login",
        old.default_scan_all_selected_confs_at_login.to_string(),
        new.default_scan_all_selected_confs_at_login.to_string(),
    );
    push_change(
        &mut c,
        "prompt_to_read_mail",
        old.prompt_to_read_mail.to_string(),
        new.prompt_to_read_mail.to_string(),
    );
    push_change(
        &mut c,
        "force_comments_to_main",
        old.force_comments_to_main.to_string(),
        new.force_comments_to_main.to_string(),
    );
    push_change(
        &mut c,
        "update_last_read_pointer",
        old.update_last_read_pointer.to_string(),
        new.update_last_read_pointer.to_string(),
    );
    c
}

// ---- file transfer ----

fn to_file_transfer_dto(config: &IcbConfig) -> FileTransferSettingsDto {
    FileTransferSettingsDto {
        disallow_batch_uploads: config.file_transfer.disallow_batch_uploads,
        promote_to_batch_transfers: config.file_transfer.promote_to_batch_transfers,
        upload_credit_time: config.file_transfer.upload_credit_time,
        upload_credit_bytes: config.file_transfer.upload_credit_bytes,
        verify_files_uploaded: config.file_transfer.verify_files_uploaded,
        upload_descr_lines: config.file_transfer.upload_descr_lines,
        display_uploader: config.file_transfer.display_uploader,
        disable_drive_size_check: config.file_transfer.disable_drive_size_check,
        stop_uploads_free_space: config.file_transfer.stop_uploads_free_space,
    }
}

fn apply_file_transfer_dto(config: &mut IcbConfig, dto: &FileTransferSettingsDto) -> Result<()> {
    config.file_transfer.disallow_batch_uploads = dto.disallow_batch_uploads;
    config.file_transfer.promote_to_batch_transfers = dto.promote_to_batch_transfers;
    config.file_transfer.upload_credit_time = dto.upload_credit_time;
    config.file_transfer.upload_credit_bytes = dto.upload_credit_bytes;
    config.file_transfer.verify_files_uploaded = dto.verify_files_uploaded;
    config.file_transfer.upload_descr_lines = dto.upload_descr_lines;
    config.file_transfer.display_uploader = dto.display_uploader;
    config.file_transfer.disable_drive_size_check = dto.disable_drive_size_check;
    config.file_transfer.stop_uploads_free_space = dto.stop_uploads_free_space;
    Ok(())
}

fn validate_file_transfer(dto: &FileTransferSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    if dto.upload_descr_lines == 0 || dto.upload_descr_lines > 99 {
        errors.push("Upload description lines must be between 1 and 99.".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_file_transfer(old: &FileTransferSettingsDto, new: &FileTransferSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(
        &mut c,
        "disallow_batch_uploads",
        old.disallow_batch_uploads.to_string(),
        new.disallow_batch_uploads.to_string(),
    );
    push_change(
        &mut c,
        "promote_to_batch_transfers",
        old.promote_to_batch_transfers.to_string(),
        new.promote_to_batch_transfers.to_string(),
    );
    push_change(
        &mut c,
        "upload_credit_time",
        old.upload_credit_time.to_string(),
        new.upload_credit_time.to_string(),
    );
    push_change(
        &mut c,
        "upload_credit_bytes",
        old.upload_credit_bytes.to_string(),
        new.upload_credit_bytes.to_string(),
    );
    push_change(
        &mut c,
        "verify_files_uploaded",
        old.verify_files_uploaded.to_string(),
        new.verify_files_uploaded.to_string(),
    );
    push_change(
        &mut c,
        "upload_descr_lines",
        old.upload_descr_lines.to_string(),
        new.upload_descr_lines.to_string(),
    );
    push_change(
        &mut c,
        "display_uploader",
        old.display_uploader.to_string(),
        new.display_uploader.to_string(),
    );
    push_change(
        &mut c,
        "disable_drive_size_check",
        old.disable_drive_size_check.to_string(),
        new.disable_drive_size_check.to_string(),
    );
    push_change(
        &mut c,
        "stop_uploads_free_space",
        old.stop_uploads_free_space.to_string(),
        new.stop_uploads_free_space.to_string(),
    );
    c
}

// ---- system control ----

fn to_system_control_dto(config: &IcbConfig) -> SystemControlSettingsDto {
    SystemControlSettingsDto {
        disable_ns_logon: config.system_control.disable_ns_logon,
        disable_full_record_updating: config.system_control.disable_full_record_updating,
        allow_alias_change: config.system_control.allow_alias_change,
        is_multi_lingual: config.system_control.is_multi_lingual,
        is_closed_board: config.system_control.is_closed_board,
        enforce_daily_time_limit: config.system_control.enforce_daily_time_limit,
        allow_password_failure_comment: config.system_control.allow_password_failure_comment,
        guard_logoff: config.system_control.guard_logoff,
        password_storage_method: password_storage_to_str(config.system_control.password_storage_method),
        confirm_caller_name: config.system_control.confirm_caller_name,
        reread_sec_level_on_join: config.system_control.reread_sec_level_on_join,
    }
}

fn apply_system_control_dto(config: &mut IcbConfig, dto: &SystemControlSettingsDto) -> Result<()> {
    config.system_control.disable_ns_logon = dto.disable_ns_logon;
    config.system_control.disable_full_record_updating = dto.disable_full_record_updating;
    config.system_control.allow_alias_change = dto.allow_alias_change;
    config.system_control.is_multi_lingual = dto.is_multi_lingual;
    config.system_control.is_closed_board = dto.is_closed_board;
    config.system_control.enforce_daily_time_limit = dto.enforce_daily_time_limit;
    config.system_control.allow_password_failure_comment = dto.allow_password_failure_comment;
    config.system_control.guard_logoff = dto.guard_logoff;
    config.system_control.password_storage_method =
        password_storage_from_str(&dto.password_storage_method).ok_or_else(|| AdminError::Validation(vec!["Invalid password storage method.".into()]))?;
    config.system_control.confirm_caller_name = dto.confirm_caller_name;
    config.system_control.reread_sec_level_on_join = dto.reread_sec_level_on_join;
    Ok(())
}

fn validate_system_control(dto: &SystemControlSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    if password_storage_from_str(&dto.password_storage_method).is_none() {
        errors.push("Password storage method must be bcrypt, argon2 or plain.".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_system_control(old: &SystemControlSettingsDto, new: &SystemControlSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(
        &mut c,
        "disable_ns_logon",
        old.disable_ns_logon.to_string(),
        new.disable_ns_logon.to_string(),
    );
    push_change(
        &mut c,
        "disable_full_record_updating",
        old.disable_full_record_updating.to_string(),
        new.disable_full_record_updating.to_string(),
    );
    push_change(
        &mut c,
        "allow_alias_change",
        old.allow_alias_change.to_string(),
        new.allow_alias_change.to_string(),
    );
    push_change(
        &mut c,
        "is_multi_lingual",
        old.is_multi_lingual.to_string(),
        new.is_multi_lingual.to_string(),
    );
    push_change(
        &mut c,
        "is_closed_board",
        old.is_closed_board.to_string(),
        new.is_closed_board.to_string(),
    );
    push_change(
        &mut c,
        "enforce_daily_time_limit",
        old.enforce_daily_time_limit.to_string(),
        new.enforce_daily_time_limit.to_string(),
    );
    push_change(
        &mut c,
        "allow_password_failure_comment",
        old.allow_password_failure_comment.to_string(),
        new.allow_password_failure_comment.to_string(),
    );
    push_change(&mut c, "guard_logoff", old.guard_logoff.to_string(), new.guard_logoff.to_string());
    push_change(
        &mut c,
        "password_storage_method",
        old.password_storage_method.clone(),
        new.password_storage_method.clone(),
    );
    push_change(
        &mut c,
        "confirm_caller_name",
        old.confirm_caller_name.to_string(),
        new.confirm_caller_name.to_string(),
    );
    push_change(
        &mut c,
        "reread_sec_level_on_join",
        old.reread_sec_level_on_join.to_string(),
        new.reread_sec_level_on_join.to_string(),
    );
    c
}

// ---- switches + board options ----

fn to_switches_dto(config: &IcbConfig) -> SwitchesSettingsDto {
    SwitchesSettingsDto {
        default_graphics_at_login: config.switches.default_graphics_at_login,
        non_graphics: config.switches.non_graphics,
        exclude_local_calls_stats: config.switches.exclude_local_calls_stats,
        display_news_behavior: news_to_str(config.switches.display_news_behavior),
        disable_registration_edits: config.switches.disable_registration_edits,
        disable_high_ascii_filter: config.switches.disable_high_ascii_filter,
        display_userinfo_at_login: config.switches.display_userinfo_at_login,
        force_intro_on_join: config.switches.force_intro_on_join,
        scan_new_blt: config.switches.scan_new_blt,
        capture_grp_chat_session: config.switches.capture_grp_chat_session,
        allow_handle_in_grpchat: config.switches.allow_handle_in_grpchat,
        give_user_password_to_doors: config.options.give_user_password_to_doors,
        call_log: config.options.call_log,
        page_bell: config.options.page_bell,
        alarm: config.options.alarm,
        log_caller_number: config.options.log_caller_number,
        log_connect_string: config.options.log_connect_string,
        log_security_level: config.options.log_security_level,
    }
}

fn apply_switches_dto(config: &mut IcbConfig, dto: &SwitchesSettingsDto) -> Result<()> {
    config.switches.default_graphics_at_login = dto.default_graphics_at_login;
    config.switches.non_graphics = dto.non_graphics;
    config.switches.exclude_local_calls_stats = dto.exclude_local_calls_stats;
    config.switches.display_news_behavior =
        news_from_str(&dto.display_news_behavior).ok_or_else(|| AdminError::Validation(vec!["Invalid display news behavior.".into()]))?;
    config.switches.disable_registration_edits = dto.disable_registration_edits;
    config.switches.disable_high_ascii_filter = dto.disable_high_ascii_filter;
    config.switches.display_userinfo_at_login = dto.display_userinfo_at_login;
    config.switches.force_intro_on_join = dto.force_intro_on_join;
    config.switches.scan_new_blt = dto.scan_new_blt;
    config.switches.capture_grp_chat_session = dto.capture_grp_chat_session;
    config.switches.allow_handle_in_grpchat = dto.allow_handle_in_grpchat;
    config.options.give_user_password_to_doors = dto.give_user_password_to_doors;
    config.options.call_log = dto.call_log;
    config.options.page_bell = dto.page_bell;
    config.options.alarm = dto.alarm;
    config.options.log_caller_number = dto.log_caller_number;
    config.options.log_connect_string = dto.log_connect_string;
    config.options.log_security_level = dto.log_security_level;
    Ok(())
}

fn validate_switches(dto: &SwitchesSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    if news_from_str(&dto.display_news_behavior).is_none() {
        errors.push("Display news behavior must be Y, N, A or X.".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_switches(old: &SwitchesSettingsDto, new: &SwitchesSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(
        &mut c,
        "default_graphics_at_login",
        old.default_graphics_at_login.to_string(),
        new.default_graphics_at_login.to_string(),
    );
    push_change(&mut c, "non_graphics", old.non_graphics.to_string(), new.non_graphics.to_string());
    push_change(
        &mut c,
        "exclude_local_calls_stats",
        old.exclude_local_calls_stats.to_string(),
        new.exclude_local_calls_stats.to_string(),
    );
    push_change(
        &mut c,
        "display_news_behavior",
        old.display_news_behavior.clone(),
        new.display_news_behavior.clone(),
    );
    push_change(
        &mut c,
        "disable_registration_edits",
        old.disable_registration_edits.to_string(),
        new.disable_registration_edits.to_string(),
    );
    push_change(
        &mut c,
        "disable_high_ascii_filter",
        old.disable_high_ascii_filter.to_string(),
        new.disable_high_ascii_filter.to_string(),
    );
    push_change(
        &mut c,
        "display_userinfo_at_login",
        old.display_userinfo_at_login.to_string(),
        new.display_userinfo_at_login.to_string(),
    );
    push_change(
        &mut c,
        "force_intro_on_join",
        old.force_intro_on_join.to_string(),
        new.force_intro_on_join.to_string(),
    );
    push_change(&mut c, "scan_new_blt", old.scan_new_blt.to_string(), new.scan_new_blt.to_string());
    push_change(
        &mut c,
        "capture_grp_chat_session",
        old.capture_grp_chat_session.to_string(),
        new.capture_grp_chat_session.to_string(),
    );
    push_change(
        &mut c,
        "allow_handle_in_grpchat",
        old.allow_handle_in_grpchat.to_string(),
        new.allow_handle_in_grpchat.to_string(),
    );
    push_change(
        &mut c,
        "give_user_password_to_doors",
        old.give_user_password_to_doors.to_string(),
        new.give_user_password_to_doors.to_string(),
    );
    push_change(&mut c, "call_log", old.call_log.to_string(), new.call_log.to_string());
    push_change(&mut c, "page_bell", old.page_bell.to_string(), new.page_bell.to_string());
    push_change(&mut c, "alarm", old.alarm.to_string(), new.alarm.to_string());
    push_change(
        &mut c,
        "log_caller_number",
        old.log_caller_number.to_string(),
        new.log_caller_number.to_string(),
    );
    push_change(
        &mut c,
        "log_connect_string",
        old.log_connect_string.to_string(),
        new.log_connect_string.to_string(),
    );
    push_change(
        &mut c,
        "log_security_level",
        old.log_security_level.to_string(),
        new.log_security_level.to_string(),
    );
    c
}

// ---- limits ----

fn to_limits_dto(config: &IcbConfig) -> LimitsSettingsDto {
    LimitsSettingsDto {
        keyboard_timeout: config.limits.keyboard_timeout,
        max_number_upload_descr_lines: config.limits.max_number_upload_descr_lines,
        min_pwd_length: config.limits.min_pwd_length,
        password_expire_days: config.limits.password_expire_days,
        password_expire_warn_days: config.limits.password_expire_warn_days,
        sysop_start: time_string(&config.limits.sysop_start),
        sysop_stop: time_string(&config.limits.sysop_stop),
    }
}

fn apply_limits_dto(config: &mut IcbConfig, dto: &LimitsSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    let start = parse_time_field("Sysop page start", &dto.sysop_start, &mut errors);
    let stop = parse_time_field("Sysop page stop", &dto.sysop_stop, &mut errors);
    if !errors.is_empty() {
        return Err(AdminError::Validation(errors));
    }
    config.limits.keyboard_timeout = dto.keyboard_timeout;
    config.limits.max_number_upload_descr_lines = dto.max_number_upload_descr_lines;
    config.limits.min_pwd_length = dto.min_pwd_length;
    config.limits.password_expire_days = dto.password_expire_days;
    config.limits.password_expire_warn_days = dto.password_expire_warn_days;
    config.limits.sysop_start = start;
    config.limits.sysop_stop = stop;
    Ok(())
}

fn validate_limits(dto: &LimitsSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    if dto.max_number_upload_descr_lines > 99 {
        errors.push("Max upload description lines must be at most 99.".to_string());
    }
    if dto.min_pwd_length > 64 {
        errors.push("Minimum password length must be at most 64.".to_string());
    }
    let _ = parse_time_field("Sysop page start", &dto.sysop_start, &mut errors);
    let _ = parse_time_field("Sysop page stop", &dto.sysop_stop, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_limits(old: &LimitsSettingsDto, new: &LimitsSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(
        &mut c,
        "keyboard_timeout",
        old.keyboard_timeout.to_string(),
        new.keyboard_timeout.to_string(),
    );
    push_change(
        &mut c,
        "max_number_upload_descr_lines",
        old.max_number_upload_descr_lines.to_string(),
        new.max_number_upload_descr_lines.to_string(),
    );
    push_change(&mut c, "min_pwd_length", old.min_pwd_length.to_string(), new.min_pwd_length.to_string());
    push_change(
        &mut c,
        "password_expire_days",
        old.password_expire_days.to_string(),
        new.password_expire_days.to_string(),
    );
    push_change(
        &mut c,
        "password_expire_warn_days",
        old.password_expire_warn_days.to_string(),
        new.password_expire_warn_days.to_string(),
    );
    push_change(&mut c, "sysop_start", old.sysop_start.clone(), new.sysop_start.trim().to_string());
    push_change(&mut c, "sysop_stop", old.sysop_stop.clone(), new.sysop_stop.trim().to_string());
    c
}

// ---- new user ----

fn to_new_user_dto(config: &IcbConfig) -> NewUserSettingsDto {
    let n = &config.new_user_settings;
    NewUserSettingsDto {
        sec_level: n.sec_level,
        new_user_groups: n.new_user_groups.clone(),
        allow_one_name_users: n.allow_one_name_users,
        use_newask_and_builtin: n.use_newask_and_builtin,
        ask_city_or_state: n.ask_city_or_state,
        ask_address: n.ask_address,
        ask_verification: n.ask_verification,
        ask_business_phone: n.ask_business_phone,
        ask_home_phone: n.ask_home_phone,
        ask_comment: n.ask_comment,
        ask_clr_msg: n.ask_clr_msg,
        ask_xfer_protocol: n.ask_xfer_protocol,
        ask_date_format: n.ask_date_format,
        ask_fse: n.ask_fse,
        ask_alias: n.ask_alias,
        ask_gender: n.ask_gender,
        ask_birthdate: n.ask_birthdate,
        ask_email: n.ask_email,
        ask_web_address: n.ask_web_address,
        ask_use_short_descr: n.ask_use_short_descr,
        auto_register_conferences: n.auto_register_conferences,
    }
}

fn apply_new_user_dto(config: &mut IcbConfig, dto: &NewUserSettingsDto) -> Result<()> {
    let n = &mut config.new_user_settings;
    n.sec_level = dto.sec_level;
    n.new_user_groups = dto.new_user_groups.trim().to_string();
    n.allow_one_name_users = dto.allow_one_name_users;
    n.use_newask_and_builtin = dto.use_newask_and_builtin;
    n.ask_city_or_state = dto.ask_city_or_state;
    n.ask_address = dto.ask_address;
    n.ask_verification = dto.ask_verification;
    n.ask_business_phone = dto.ask_business_phone;
    n.ask_home_phone = dto.ask_home_phone;
    n.ask_comment = dto.ask_comment;
    n.ask_clr_msg = dto.ask_clr_msg;
    n.ask_xfer_protocol = dto.ask_xfer_protocol;
    n.ask_date_format = dto.ask_date_format;
    n.ask_fse = dto.ask_fse;
    n.ask_alias = dto.ask_alias;
    n.ask_gender = dto.ask_gender;
    n.ask_birthdate = dto.ask_birthdate;
    n.ask_email = dto.ask_email;
    n.ask_web_address = dto.ask_web_address;
    n.ask_use_short_descr = dto.ask_use_short_descr;
    n.auto_register_conferences = dto.auto_register_conferences;
    Ok(())
}

fn validate_new_user(dto: &NewUserSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    check_text("New user groups", &dto.new_user_groups, 128, false, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_new_user(old: &NewUserSettingsDto, new: &NewUserSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(&mut c, "sec_level", old.sec_level.to_string(), new.sec_level.to_string());
    push_change(
        &mut c,
        "new_user_groups",
        old.new_user_groups.clone(),
        new.new_user_groups.trim().to_string(),
    );
    push_change(
        &mut c,
        "allow_one_name_users",
        old.allow_one_name_users.to_string(),
        new.allow_one_name_users.to_string(),
    );
    push_change(
        &mut c,
        "use_newask_and_builtin",
        old.use_newask_and_builtin.to_string(),
        new.use_newask_and_builtin.to_string(),
    );
    push_change(
        &mut c,
        "ask_city_or_state",
        old.ask_city_or_state.to_string(),
        new.ask_city_or_state.to_string(),
    );
    push_change(&mut c, "ask_address", old.ask_address.to_string(), new.ask_address.to_string());
    push_change(
        &mut c,
        "ask_verification",
        old.ask_verification.to_string(),
        new.ask_verification.to_string(),
    );
    push_change(
        &mut c,
        "ask_business_phone",
        old.ask_business_phone.to_string(),
        new.ask_business_phone.to_string(),
    );
    push_change(
        &mut c,
        "ask_home_phone",
        old.ask_home_phone.to_string(),
        new.ask_home_phone.to_string(),
    );
    push_change(&mut c, "ask_comment", old.ask_comment.to_string(), new.ask_comment.to_string());
    push_change(&mut c, "ask_clr_msg", old.ask_clr_msg.to_string(), new.ask_clr_msg.to_string());
    push_change(
        &mut c,
        "ask_xfer_protocol",
        old.ask_xfer_protocol.to_string(),
        new.ask_xfer_protocol.to_string(),
    );
    push_change(
        &mut c,
        "ask_date_format",
        old.ask_date_format.to_string(),
        new.ask_date_format.to_string(),
    );
    push_change(&mut c, "ask_fse", old.ask_fse.to_string(), new.ask_fse.to_string());
    push_change(&mut c, "ask_alias", old.ask_alias.to_string(), new.ask_alias.to_string());
    push_change(&mut c, "ask_gender", old.ask_gender.to_string(), new.ask_gender.to_string());
    push_change(&mut c, "ask_birthdate", old.ask_birthdate.to_string(), new.ask_birthdate.to_string());
    push_change(&mut c, "ask_email", old.ask_email.to_string(), new.ask_email.to_string());
    push_change(
        &mut c,
        "ask_web_address",
        old.ask_web_address.to_string(),
        new.ask_web_address.to_string(),
    );
    push_change(
        &mut c,
        "ask_use_short_descr",
        old.ask_use_short_descr.to_string(),
        new.ask_use_short_descr.to_string(),
    );
    push_change(
        &mut c,
        "auto_register_conferences",
        old.auto_register_conferences.to_string(),
        new.auto_register_conferences.to_string(),
    );
    c
}

// ---- event ----

fn to_event_dto(config: &IcbConfig) -> EventSettingsDto {
    EventSettingsDto {
        enabled: config.event.enabled,
        event_file: path_string(&config.event.event_file),
        suspend_minutes: config.event.suspend_minutes,
        disallow_uploads: config.event.disallow_uploads,
        minutes_uploads_disallowed: config.event.minutes_uploads_disallowed,
    }
}

fn apply_event_dto(config: &mut IcbConfig, dto: &EventSettingsDto) -> Result<()> {
    config.event.enabled = dto.enabled;
    set_path(&mut config.event.event_file, &dto.event_file);
    config.event.suspend_minutes = dto.suspend_minutes;
    config.event.disallow_uploads = dto.disallow_uploads;
    config.event.minutes_uploads_disallowed = dto.minutes_uploads_disallowed;
    Ok(())
}

fn validate_event(dto: &EventSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    check_path_text("Event file", &dto.event_file, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_event(old: &EventSettingsDto, new: &EventSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(&mut c, "enabled", old.enabled.to_string(), new.enabled.to_string());
    push_change(&mut c, "event_file", old.event_file.clone(), new.event_file.trim().to_string());
    push_change(
        &mut c,
        "suspend_minutes",
        old.suspend_minutes.to_string(),
        new.suspend_minutes.to_string(),
    );
    push_change(
        &mut c,
        "disallow_uploads",
        old.disallow_uploads.to_string(),
        new.disallow_uploads.to_string(),
    );
    push_change(
        &mut c,
        "minutes_uploads_disallowed",
        old.minutes_uploads_disallowed.to_string(),
        new.minutes_uploads_disallowed.to_string(),
    );
    c
}

// ---- subscription ----

fn to_subscription_dto(config: &IcbConfig) -> SubscriptionSettingsDto {
    SubscriptionSettingsDto {
        is_enabled: config.subscription_info.is_enabled,
        subscription_length: config.subscription_info.subscription_length,
        default_expired_level: config.subscription_info.default_expired_level,
        warning_days: config.subscription_info.warning_days,
    }
}

fn apply_subscription_dto(config: &mut IcbConfig, dto: &SubscriptionSettingsDto) -> Result<()> {
    config.subscription_info.is_enabled = dto.is_enabled;
    config.subscription_info.subscription_length = dto.subscription_length;
    config.subscription_info.default_expired_level = dto.default_expired_level;
    config.subscription_info.warning_days = dto.warning_days;
    Ok(())
}

fn validate_subscription(_dto: &SubscriptionSettingsDto) -> Result<()> {
    Ok(())
}

fn diff_subscription(old: &SubscriptionSettingsDto, new: &SubscriptionSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(&mut c, "is_enabled", old.is_enabled.to_string(), new.is_enabled.to_string());
    push_change(
        &mut c,
        "subscription_length",
        old.subscription_length.to_string(),
        new.subscription_length.to_string(),
    );
    push_change(
        &mut c,
        "default_expired_level",
        old.default_expired_level.to_string(),
        new.default_expired_level.to_string(),
    );
    push_change(&mut c, "warning_days", old.warning_days.to_string(), new.warning_days.to_string());
    c
}

// ---- connection ----

fn to_connection_dto(config: &IcbConfig) -> ConnectionSettingsDto {
    ConnectionSettingsDto {
        telnet: ListenerDto {
            is_enabled: config.login_server.telnet.is_enabled,
            port: config.login_server.telnet.port,
            address: config.login_server.telnet.address.clone(),
            display_file: path_string(&config.login_server.telnet.display_file),
        },
        ssh: ListenerDto {
            is_enabled: config.login_server.ssh.is_enabled,
            port: config.login_server.ssh.port,
            address: config.login_server.ssh.address.clone(),
            display_file: path_string(&config.login_server.ssh.display_file),
        },
        secure_websocket: SecureWebsocketDto {
            is_enabled: config.login_server.secure_websocket.is_enabled,
            port: config.login_server.secure_websocket.port,
            address: config.login_server.secure_websocket.address.clone(),
            display_file: path_string(&config.login_server.secure_websocket.display_file),
            cert_pem: path_string(&config.login_server.secure_websocket.cert_pem),
            key_pem: path_string(&config.login_server.secure_websocket.key_pem),
        },
    }
}

fn apply_connection_dto(config: &mut IcbConfig, dto: &ConnectionSettingsDto) -> Result<()> {
    config.login_server.telnet.is_enabled = dto.telnet.is_enabled;
    config.login_server.telnet.port = dto.telnet.port;
    config.login_server.telnet.address = dto.telnet.address.trim().to_string();
    set_path(&mut config.login_server.telnet.display_file, &dto.telnet.display_file);

    config.login_server.ssh.is_enabled = dto.ssh.is_enabled;
    config.login_server.ssh.port = dto.ssh.port;
    config.login_server.ssh.address = dto.ssh.address.trim().to_string();
    set_path(&mut config.login_server.ssh.display_file, &dto.ssh.display_file);

    config.login_server.secure_websocket.is_enabled = dto.secure_websocket.is_enabled;
    config.login_server.secure_websocket.port = dto.secure_websocket.port;
    config.login_server.secure_websocket.address = dto.secure_websocket.address.trim().to_string();
    set_path(
        &mut config.login_server.secure_websocket.display_file,
        &dto.secure_websocket.display_file,
    );
    set_path(&mut config.login_server.secure_websocket.cert_pem, &dto.secure_websocket.cert_pem);
    set_path(&mut config.login_server.secure_websocket.key_pem, &dto.secure_websocket.key_pem);
    Ok(())
}

fn validate_connection(dto: &ConnectionSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    for (label, listener) in [("Telnet", &dto.telnet), ("SSH", &dto.ssh)] {
        if listener.port == 0 {
            errors.push(format!("{label} port must be between 1 and 65535."));
        }
        check_text(&format!("{label} address"), &listener.address, 64, false, &mut errors);
        check_path_text(&format!("{label} display file"), &listener.display_file, &mut errors);
    }
    if dto.secure_websocket.port == 0 {
        errors.push("Secure WebSocket port must be between 1 and 65535.".to_string());
    }
    check_text("Secure WebSocket address", &dto.secure_websocket.address, 64, false, &mut errors);
    check_path_text("Secure WebSocket display file", &dto.secure_websocket.display_file, &mut errors);
    check_path_text("Certificate PEM", &dto.secure_websocket.cert_pem, &mut errors);
    check_path_text("Key PEM", &dto.secure_websocket.key_pem, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_connection(old: &ConnectionSettingsDto, new: &ConnectionSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(
        &mut c,
        "telnet.is_enabled",
        old.telnet.is_enabled.to_string(),
        new.telnet.is_enabled.to_string(),
    );
    push_change(&mut c, "telnet.port", old.telnet.port.to_string(), new.telnet.port.to_string());
    push_change(
        &mut c,
        "telnet.address",
        old.telnet.address.clone(),
        new.telnet.address.trim().to_string(),
    );
    push_change(
        &mut c,
        "telnet.display_file",
        old.telnet.display_file.clone(),
        new.telnet.display_file.trim().to_string(),
    );
    push_change(&mut c, "ssh.is_enabled", old.ssh.is_enabled.to_string(), new.ssh.is_enabled.to_string());
    push_change(&mut c, "ssh.port", old.ssh.port.to_string(), new.ssh.port.to_string());
    push_change(&mut c, "ssh.address", old.ssh.address.clone(), new.ssh.address.trim().to_string());
    push_change(
        &mut c,
        "ssh.display_file",
        old.ssh.display_file.clone(),
        new.ssh.display_file.trim().to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.is_enabled",
        old.secure_websocket.is_enabled.to_string(),
        new.secure_websocket.is_enabled.to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.port",
        old.secure_websocket.port.to_string(),
        new.secure_websocket.port.to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.address",
        old.secure_websocket.address.clone(),
        new.secure_websocket.address.trim().to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.display_file",
        old.secure_websocket.display_file.clone(),
        new.secure_websocket.display_file.trim().to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.cert_pem",
        old.secure_websocket.cert_pem.clone(),
        new.secure_websocket.cert_pem.trim().to_string(),
    );
    push_change(
        &mut c,
        "secure_websocket.key_pem",
        old.secure_websocket.key_pem.clone(),
        new.secure_websocket.key_pem.trim().to_string(),
    );
    c
}

// ---- paths ----

fn to_paths_dto(config: &IcbConfig) -> PathsSettingsDto {
    let p = &config.paths;
    PathsSettingsDto {
        help_path: path_string(&p.help_path),
        security_file_path: path_string(&p.security_file_path),
        email_msgbase: path_string(&p.email_msgbase),
        command_display_path: path_string(&p.command_display_path),
        tmp_work_path: path_string(&p.tmp_work_path),
        icbtext: path_string(&p.icbtext),
        conferences: path_string(&p.conferences),
        welcome: path_string(&p.welcome),
        newuser: path_string(&p.newuser),
        closed: path_string(&p.closed),
        expire_warning: path_string(&p.expire_warning),
        expired: path_string(&p.expired),
        conf_join_menu: path_string(&p.conf_join_menu),
        chat_intro_file: path_string(&p.chat_intro_file),
        chat_menu: path_string(&p.chat_menu),
        chat_actions_menu: path_string(&p.chat_actions_menu),
        no_ansi: path_string(&p.no_ansi),
        trashcan_upload_files: path_string(&p.trashcan_upload_files),
        trashcan_user: path_string(&p.trashcan_user),
        trashcan_email: path_string(&p.trashcan_email),
        trashcan_passwords: path_string(&p.trashcan_passwords),
        vip_users: path_string(&p.vip_users),
        protocol_data_file: path_string(&p.protocol_data_file),
        pwrd_sec_level_file: path_string(&p.pwrd_sec_level_file),
        command_file: path_string(&p.command_file),
        statistics_file: path_string(&p.statistics_file),
        language_file: path_string(&p.language_file),
        group_file: path_string(&p.group_file),
        ftn_file: path_string(&p.ftn_file),
        user_file: path_string(&p.user_file),
        caller_log: path_string(&p.caller_log),
        logon_survey: path_string(&p.logon_survey),
        logon_answer: path_string(&p.logon_answer),
        logoff_survey: path_string(&p.logoff_survey),
        logoff_answer: path_string(&p.logoff_answer),
        newask_survey: path_string(&p.newask_survey),
        newask_answer: path_string(&p.newask_answer),
    }
}

fn apply_paths_dto(config: &mut IcbConfig, dto: &PathsSettingsDto) -> Result<()> {
    let p = &mut config.paths;
    set_path(&mut p.help_path, &dto.help_path);
    set_path(&mut p.security_file_path, &dto.security_file_path);
    set_path(&mut p.email_msgbase, &dto.email_msgbase);
    set_path(&mut p.command_display_path, &dto.command_display_path);
    set_path(&mut p.tmp_work_path, &dto.tmp_work_path);
    set_path(&mut p.icbtext, &dto.icbtext);
    set_path(&mut p.conferences, &dto.conferences);
    set_path(&mut p.welcome, &dto.welcome);
    set_path(&mut p.newuser, &dto.newuser);
    set_path(&mut p.closed, &dto.closed);
    set_path(&mut p.expire_warning, &dto.expire_warning);
    set_path(&mut p.expired, &dto.expired);
    set_path(&mut p.conf_join_menu, &dto.conf_join_menu);
    set_path(&mut p.chat_intro_file, &dto.chat_intro_file);
    set_path(&mut p.chat_menu, &dto.chat_menu);
    set_path(&mut p.chat_actions_menu, &dto.chat_actions_menu);
    set_path(&mut p.no_ansi, &dto.no_ansi);
    set_path(&mut p.trashcan_upload_files, &dto.trashcan_upload_files);
    set_path(&mut p.trashcan_user, &dto.trashcan_user);
    set_path(&mut p.trashcan_email, &dto.trashcan_email);
    set_path(&mut p.trashcan_passwords, &dto.trashcan_passwords);
    set_path(&mut p.vip_users, &dto.vip_users);
    set_path(&mut p.protocol_data_file, &dto.protocol_data_file);
    set_path(&mut p.pwrd_sec_level_file, &dto.pwrd_sec_level_file);
    set_path(&mut p.command_file, &dto.command_file);
    set_path(&mut p.statistics_file, &dto.statistics_file);
    set_path(&mut p.language_file, &dto.language_file);
    set_path(&mut p.group_file, &dto.group_file);
    set_path(&mut p.ftn_file, &dto.ftn_file);
    set_path(&mut p.user_file, &dto.user_file);
    set_path(&mut p.caller_log, &dto.caller_log);
    set_path(&mut p.logon_survey, &dto.logon_survey);
    set_path(&mut p.logon_answer, &dto.logon_answer);
    set_path(&mut p.logoff_survey, &dto.logoff_survey);
    set_path(&mut p.logoff_answer, &dto.logoff_answer);
    set_path(&mut p.newask_survey, &dto.newask_survey);
    set_path(&mut p.newask_answer, &dto.newask_answer);
    Ok(())
}

fn validate_paths(dto: &PathsSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    let fields = [
        ("help_path", &dto.help_path),
        ("security_file_path", &dto.security_file_path),
        ("email_msgbase", &dto.email_msgbase),
        ("command_display_path", &dto.command_display_path),
        ("tmp_work_path", &dto.tmp_work_path),
        ("icbtext", &dto.icbtext),
        ("conferences", &dto.conferences),
        ("welcome", &dto.welcome),
        ("newuser", &dto.newuser),
        ("closed", &dto.closed),
        ("expire_warning", &dto.expire_warning),
        ("expired", &dto.expired),
        ("conf_join_menu", &dto.conf_join_menu),
        ("chat_intro_file", &dto.chat_intro_file),
        ("chat_menu", &dto.chat_menu),
        ("chat_actions_menu", &dto.chat_actions_menu),
        ("no_ansi", &dto.no_ansi),
        ("trashcan_upload_files", &dto.trashcan_upload_files),
        ("trashcan_user", &dto.trashcan_user),
        ("trashcan_email", &dto.trashcan_email),
        ("trashcan_passwords", &dto.trashcan_passwords),
        ("vip_users", &dto.vip_users),
        ("protocol_data_file", &dto.protocol_data_file),
        ("pwrd_sec_level_file", &dto.pwrd_sec_level_file),
        ("command_file", &dto.command_file),
        ("statistics_file", &dto.statistics_file),
        ("language_file", &dto.language_file),
        ("group_file", &dto.group_file),
        ("ftn_file", &dto.ftn_file),
        ("user_file", &dto.user_file),
        ("caller_log", &dto.caller_log),
        ("logon_survey", &dto.logon_survey),
        ("logon_answer", &dto.logon_answer),
        ("logoff_survey", &dto.logoff_survey),
        ("logoff_answer", &dto.logoff_answer),
        ("newask_survey", &dto.newask_survey),
        ("newask_answer", &dto.newask_answer),
    ];
    for (label, value) in fields {
        check_path_text(label, value, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_paths(old: &PathsSettingsDto, new: &PathsSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    let pairs = [
        ("help_path", &old.help_path, new.help_path.trim()),
        ("security_file_path", &old.security_file_path, new.security_file_path.trim()),
        ("email_msgbase", &old.email_msgbase, new.email_msgbase.trim()),
        ("command_display_path", &old.command_display_path, new.command_display_path.trim()),
        ("tmp_work_path", &old.tmp_work_path, new.tmp_work_path.trim()),
        ("icbtext", &old.icbtext, new.icbtext.trim()),
        ("conferences", &old.conferences, new.conferences.trim()),
        ("welcome", &old.welcome, new.welcome.trim()),
        ("newuser", &old.newuser, new.newuser.trim()),
        ("closed", &old.closed, new.closed.trim()),
        ("expire_warning", &old.expire_warning, new.expire_warning.trim()),
        ("expired", &old.expired, new.expired.trim()),
        ("conf_join_menu", &old.conf_join_menu, new.conf_join_menu.trim()),
        ("chat_intro_file", &old.chat_intro_file, new.chat_intro_file.trim()),
        ("chat_menu", &old.chat_menu, new.chat_menu.trim()),
        ("chat_actions_menu", &old.chat_actions_menu, new.chat_actions_menu.trim()),
        ("no_ansi", &old.no_ansi, new.no_ansi.trim()),
        ("trashcan_upload_files", &old.trashcan_upload_files, new.trashcan_upload_files.trim()),
        ("trashcan_user", &old.trashcan_user, new.trashcan_user.trim()),
        ("trashcan_email", &old.trashcan_email, new.trashcan_email.trim()),
        ("trashcan_passwords", &old.trashcan_passwords, new.trashcan_passwords.trim()),
        ("vip_users", &old.vip_users, new.vip_users.trim()),
        ("protocol_data_file", &old.protocol_data_file, new.protocol_data_file.trim()),
        ("pwrd_sec_level_file", &old.pwrd_sec_level_file, new.pwrd_sec_level_file.trim()),
        ("command_file", &old.command_file, new.command_file.trim()),
        ("statistics_file", &old.statistics_file, new.statistics_file.trim()),
        ("language_file", &old.language_file, new.language_file.trim()),
        ("group_file", &old.group_file, new.group_file.trim()),
        ("ftn_file", &old.ftn_file, new.ftn_file.trim()),
        ("user_file", &old.user_file, new.user_file.trim()),
        ("caller_log", &old.caller_log, new.caller_log.trim()),
        ("logon_survey", &old.logon_survey, new.logon_survey.trim()),
        ("logon_answer", &old.logon_answer, new.logon_answer.trim()),
        ("logoff_survey", &old.logoff_survey, new.logoff_survey.trim()),
        ("logoff_answer", &old.logoff_answer, new.logoff_answer.trim()),
        ("newask_survey", &old.newask_survey, new.newask_survey.trim()),
        ("newask_answer", &old.newask_answer, new.newask_answer.trim()),
    ];
    for (field, old_v, new_v) in pairs {
        push_change(&mut c, field, old_v.clone(), new_v.to_string());
    }
    c
}

// ---- accounting ----

fn to_accounting_dto(config: &IcbConfig) -> AccountingSettingsDto {
    let a = &config.accounting;
    AccountingSettingsDto {
        enabled: a.enabled,
        use_money: a.use_money,
        concurrent_tracking: a.concurrent_tracking,
        ignore_empty_sec_level: a.ignore_empty_sec_level,
        peak_usage_start: time_string(&a.peak_usage_start),
        peak_usage_end: time_string(&a.peak_usage_end),
        peak_days_of_week: a.peak_days_of_week.to_string(),
        peak_holiday_list_file: path_string(&a.peak_holiday_list_file),
        cfg_file: path_string(&a.cfg_file),
        tracking_file: path_string(&a.tracking_file),
        info_file: path_string(&a.info_file),
        warning_file: path_string(&a.warning_file),
        logoff_file: path_string(&a.logoff_file),
    }
}

fn apply_accounting_dto(config: &mut IcbConfig, dto: &AccountingSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    let start = parse_time_field("Peak usage start", &dto.peak_usage_start, &mut errors);
    let end = parse_time_field("Peak usage end", &dto.peak_usage_end, &mut errors);
    let dow = dto.peak_days_of_week.trim();
    if !dow.is_empty() && (dow.len() != 7 || !dow.chars().all(|ch| ch == 'Y' || ch == 'N')) {
        errors.push("Peak days of week must be seven Y/N characters (Sunday first).".to_string());
    }
    if !errors.is_empty() {
        return Err(AdminError::Validation(errors));
    }

    let a = &mut config.accounting;
    a.enabled = dto.enabled;
    a.use_money = dto.use_money;
    a.concurrent_tracking = dto.concurrent_tracking;
    a.ignore_empty_sec_level = dto.ignore_empty_sec_level;
    a.peak_usage_start = start;
    a.peak_usage_end = end;
    a.peak_days_of_week = if dow.is_empty() {
        IcbDoW::default()
    } else {
        IcbDoW::from_str(dow).unwrap_or_default()
    };
    set_path(&mut a.peak_holiday_list_file, &dto.peak_holiday_list_file);
    set_path(&mut a.cfg_file, &dto.cfg_file);
    set_path(&mut a.tracking_file, &dto.tracking_file);
    set_path(&mut a.info_file, &dto.info_file);
    set_path(&mut a.warning_file, &dto.warning_file);
    set_path(&mut a.logoff_file, &dto.logoff_file);
    Ok(())
}

fn validate_accounting(dto: &AccountingSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    let _ = parse_time_field("Peak usage start", &dto.peak_usage_start, &mut errors);
    let _ = parse_time_field("Peak usage end", &dto.peak_usage_end, &mut errors);
    let dow = dto.peak_days_of_week.trim();
    if !dow.is_empty() && (dow.len() != 7 || !dow.chars().all(|ch| ch == 'Y' || ch == 'N')) {
        errors.push("Peak days of week must be seven Y/N characters (Sunday first).".to_string());
    }
    for (label, value) in [
        ("peak_holiday_list_file", &dto.peak_holiday_list_file),
        ("cfg_file", &dto.cfg_file),
        ("tracking_file", &dto.tracking_file),
        ("info_file", &dto.info_file),
        ("warning_file", &dto.warning_file),
        ("logoff_file", &dto.logoff_file),
    ] {
        check_path_text(label, value, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_accounting(old: &AccountingSettingsDto, new: &AccountingSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    push_change(&mut c, "enabled", old.enabled.to_string(), new.enabled.to_string());
    push_change(&mut c, "use_money", old.use_money.to_string(), new.use_money.to_string());
    push_change(
        &mut c,
        "concurrent_tracking",
        old.concurrent_tracking.to_string(),
        new.concurrent_tracking.to_string(),
    );
    push_change(
        &mut c,
        "ignore_empty_sec_level",
        old.ignore_empty_sec_level.to_string(),
        new.ignore_empty_sec_level.to_string(),
    );
    push_change(
        &mut c,
        "peak_usage_start",
        old.peak_usage_start.clone(),
        new.peak_usage_start.trim().to_string(),
    );
    push_change(
        &mut c,
        "peak_usage_end",
        old.peak_usage_end.clone(),
        new.peak_usage_end.trim().to_string(),
    );
    push_change(
        &mut c,
        "peak_days_of_week",
        old.peak_days_of_week.clone(),
        new.peak_days_of_week.trim().to_string(),
    );
    push_change(
        &mut c,
        "peak_holiday_list_file",
        old.peak_holiday_list_file.clone(),
        new.peak_holiday_list_file.trim().to_string(),
    );
    push_change(&mut c, "cfg_file", old.cfg_file.clone(), new.cfg_file.trim().to_string());
    push_change(
        &mut c,
        "tracking_file",
        old.tracking_file.clone(),
        new.tracking_file.trim().to_string(),
    );
    push_change(&mut c, "info_file", old.info_file.clone(), new.info_file.trim().to_string());
    push_change(
        &mut c,
        "warning_file",
        old.warning_file.clone(),
        new.warning_file.trim().to_string(),
    );
    push_change(&mut c, "logoff_file", old.logoff_file.clone(), new.logoff_file.trim().to_string());
    c
}

// ---- function keys ----

fn to_function_keys_dto(config: &IcbConfig) -> FunctionKeysSettingsDto {
    FunctionKeysSettingsDto {
        keys: config.func_keys.clone(),
    }
}

fn apply_function_keys_dto(config: &mut IcbConfig, dto: &FunctionKeysSettingsDto) -> Result<()> {
    for (i, key) in dto.keys.iter().enumerate() {
        config.func_keys[i] = key.to_string();
    }
    Ok(())
}

fn validate_function_keys(dto: &FunctionKeysSettingsDto) -> Result<()> {
    let mut errors = Vec::new();
    for (i, key) in dto.keys.iter().enumerate() {
        if key.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
            errors.push(format!("F{} must not contain control characters.", i + 1));
        }
        if key.chars().count() > 256 {
            errors.push(format!("F{} must not be longer than 256 characters.", i + 1));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AdminError::Validation(errors))
    }
}

fn diff_function_keys(old: &FunctionKeysSettingsDto, new: &FunctionKeysSettingsDto) -> Vec<FieldChangeDto> {
    let mut c = Vec::new();
    for i in 0..10 {
        push_change(&mut c, &format!("f{}", i + 1), old.keys[i].clone(), new.keys[i].clone());
    }
    c
}

// ---------------------------------------------------------------- conferences

fn conferences_path(root_path: &Path, config: &IcbConfig) -> PathBuf {
    let configured = &config.paths.conferences;
    if configured.as_os_str().is_empty() {
        return PathBuf::new();
    }
    if configured.is_absolute() {
        configured.clone()
    } else {
        root_path.join(configured)
    }
}

fn conference_type_to_string(kind: &ConferenceType) -> String {
    match kind {
        ConferenceType::Normal => "Normal",
        ConferenceType::InternetEmail => "InternetEmail",
        ConferenceType::InternetUsenet => "InternetUsenet",
        ConferenceType::UsnetModeratedNewsgroup => "UsnetModeratedNewsgroup",
        ConferenceType::UsnetPublicNewsgroup => "UsnetPublicNewsgroup",
        ConferenceType::FidoConference => "FidoConference",
    }
    .to_string()
}

fn conference_type_from_str(value: &str) -> Option<ConferenceType> {
    Some(match value {
        "Normal" => ConferenceType::Normal,
        "InternetEmail" => ConferenceType::InternetEmail,
        "InternetUsenet" => ConferenceType::InternetUsenet,
        "UsnetModeratedNewsgroup" => ConferenceType::UsnetModeratedNewsgroup,
        "UsnetPublicNewsgroup" => ConferenceType::UsnetPublicNewsgroup,
        "FidoConference" => ConferenceType::FidoConference,
        _ => return None,
    })
}

fn security_to_string(expr: &SecurityExpression) -> String {
    if expr.is_empty() { String::new() } else { expr.to_string() }
}

fn parse_security(label: &str, value: &str, errors: &mut Vec<String>) -> SecurityExpression {
    match SecurityExpression::from_str(value.trim()) {
        Ok(expr) => expr,
        Err(e) => {
            errors.push(format!("{label}: {e}"));
            SecurityExpression::default()
        }
    }
}

fn to_conference_dto(conf: &Conference) -> ConferenceDto {
    let path = |p: &PathBuf| p.display().to_string();
    ConferenceDto {
        name: conf.name.clone(),
        conference_type: conference_type_to_string(&conf.conference_type),
        is_public: conf.is_public,
        is_read_only: conf.is_read_only,
        echo_mail_in_conference: conf.echo_mail_in_conference,
        force_echomail: conf.force_echomail,
        auto_rejoin: conf.auto_rejoin,
        allow_view_conf_members: conf.allow_view_conf_members,
        private_uploads: conf.private_uploads,
        private_msgs: conf.private_msgs,
        disallow_private_msgs: conf.disallow_private_msgs,
        allow_aliases: conf.allow_aliases,
        show_intro_in_scan: conf.show_intro_in_scan,
        use_main_commands: conf.use_main_commands,
        record_origin: conf.record_origin,
        prompt_for_routing: conf.prompt_for_routing,
        long_to_names: conf.long_to_names,
        required_security: security_to_string(&conf.required_security),
        sec_attachments: security_to_string(&conf.sec_attachments),
        sec_write_message: security_to_string(&conf.sec_write_message),
        sec_request_rr: security_to_string(&conf.sec_request_rr),
        sec_carbon_copy: security_to_string(&conf.sec_carbon_copy),
        carbon_list_limit: conf.carbon_list_limit,
        add_conference_security: conf.add_conference_security,
        add_conference_time: conf.add_conference_time,
        pub_upload_sort: conf.pub_upload_sort,
        private_upload_sort: conf.private_upload_sort,
        charge_time: conf.charge_time,
        charge_msg_read: conf.charge_msg_read,
        charge_msg_write: conf.charge_msg_write,
        users_menu: path(&conf.users_menu),
        sysop_menu: path(&conf.sysop_menu),
        news_file: path(&conf.news_file),
        intro_file: path(&conf.intro_file),
        attachment_location: path(&conf.attachment_location),
        command_file: path(&conf.command_file),
        pub_upload_location: path(&conf.pub_upload_location),
        pub_upload_metadata: path(&conf.pub_upload_metadata),
        private_upload_location: path(&conf.private_upload_location),
        private_upload_metadata: path(&conf.private_upload_metadata),
        doors_menu: path(&conf.doors_menu),
        doors_file: path(&conf.doors_file),
        blt_menu: path(&conf.blt_menu),
        blt_file: path(&conf.blt_file),
        survey_menu: path(&conf.survey_menu),
        survey_file: path(&conf.survey_file),
        dir_menu: path(&conf.dir_menu),
        dir_file: path(&conf.dir_file),
        area_menu: path(&conf.area_menu),
        area_file: path(&conf.area_file),
        new_password: String::new(),
        clear_password: false,
    }
}

/// Trims text and normalizes security expressions so a round trip does not show up as a change.
fn normalize_conference(dto: &ConferenceDto) -> Result<ConferenceDto> {
    validate_conference(dto)?;
    let mut errors = Vec::new();
    let mut out = dto.clone();
    out.name = dto.name.trim().to_string();
    out.conference_type = dto.conference_type.trim().to_string();
    out.required_security = security_to_string(&parse_security("Required security", &dto.required_security, &mut errors));
    out.sec_attachments = security_to_string(&parse_security("Attachment security", &dto.sec_attachments, &mut errors));
    out.sec_write_message = security_to_string(&parse_security("Write message security", &dto.sec_write_message, &mut errors));
    out.sec_request_rr = security_to_string(&parse_security("Return receipt security", &dto.sec_request_rr, &mut errors));
    out.sec_carbon_copy = security_to_string(&parse_security("Carbon copy security", &dto.sec_carbon_copy, &mut errors));
    for field in [
        &mut out.users_menu,
        &mut out.sysop_menu,
        &mut out.news_file,
        &mut out.intro_file,
        &mut out.attachment_location,
        &mut out.command_file,
        &mut out.pub_upload_location,
        &mut out.pub_upload_metadata,
        &mut out.private_upload_location,
        &mut out.private_upload_metadata,
        &mut out.doors_menu,
        &mut out.doors_file,
        &mut out.blt_menu,
        &mut out.blt_file,
        &mut out.survey_menu,
        &mut out.survey_file,
        &mut out.dir_menu,
        &mut out.dir_file,
        &mut out.area_menu,
        &mut out.area_file,
    ] {
        *field = field.trim().to_string();
    }
    if !errors.is_empty() {
        return Err(AdminError::Validation(errors));
    }
    Ok(out)
}

fn validate_conference(dto: &ConferenceDto) -> Result<()> {
    let mut errors = Vec::new();
    check_text("Conference name", &dto.name, 60, true, &mut errors);
    if conference_type_from_str(dto.conference_type.trim()).is_none() {
        errors.push(format!("Unknown conference type '{}'", dto.conference_type));
    }
    for (label, value) in [
        ("Required security", &dto.required_security),
        ("Attachment security", &dto.sec_attachments),
        ("Write message security", &dto.sec_write_message),
        ("Return receipt security", &dto.sec_request_rr),
        ("Carbon copy security", &dto.sec_carbon_copy),
    ] {
        if let Err(e) = SecurityExpression::from_str(value.trim()) {
            errors.push(format!("{label}: {e}"));
        }
    }
    for (label, value) in [
        ("Charge time", dto.charge_time),
        ("Charge per message read", dto.charge_msg_read),
        ("Charge per message written", dto.charge_msg_write),
    ] {
        if !value.is_finite() || value < 0.0 {
            errors.push(format!("{label} must be zero or a positive number"));
        }
    }
    if !dto.new_password.is_empty() && dto.clear_password {
        errors.push("A new join password and clearing the password cannot be combined".to_string());
    }
    if dto.new_password.len() > 60 {
        errors.push("Join password must be at most 60 characters".to_string());
    }
    if errors.is_empty() { Ok(()) } else { Err(AdminError::Validation(errors)) }
}

fn apply_conference_dto(conf: &mut Conference, dto: &ConferenceDto) -> Result<()> {
    let mut errors = Vec::new();
    let Some(kind) = conference_type_from_str(dto.conference_type.trim()) else {
        return Err(AdminError::Validation(vec![format!("Unknown conference type '{}'", dto.conference_type)]));
    };

    conf.name = dto.name.trim().to_string();
    conf.conference_type = kind;
    conf.is_public = dto.is_public;
    conf.is_read_only = dto.is_read_only;
    conf.echo_mail_in_conference = dto.echo_mail_in_conference;
    conf.force_echomail = dto.force_echomail;
    conf.auto_rejoin = dto.auto_rejoin;
    conf.allow_view_conf_members = dto.allow_view_conf_members;
    conf.private_uploads = dto.private_uploads;
    conf.private_msgs = dto.private_msgs;
    conf.disallow_private_msgs = dto.disallow_private_msgs;
    conf.allow_aliases = dto.allow_aliases;
    conf.show_intro_in_scan = dto.show_intro_in_scan;
    conf.use_main_commands = dto.use_main_commands;
    conf.record_origin = dto.record_origin;
    conf.prompt_for_routing = dto.prompt_for_routing;
    conf.long_to_names = dto.long_to_names;

    conf.required_security = parse_security("Required security", &dto.required_security, &mut errors);
    conf.sec_attachments = parse_security("Attachment security", &dto.sec_attachments, &mut errors);
    conf.sec_write_message = parse_security("Write message security", &dto.sec_write_message, &mut errors);
    conf.sec_request_rr = parse_security("Return receipt security", &dto.sec_request_rr, &mut errors);
    conf.sec_carbon_copy = parse_security("Carbon copy security", &dto.sec_carbon_copy, &mut errors);

    conf.carbon_list_limit = dto.carbon_list_limit;
    conf.add_conference_security = dto.add_conference_security;
    conf.add_conference_time = dto.add_conference_time;
    conf.pub_upload_sort = dto.pub_upload_sort;
    conf.private_upload_sort = dto.private_upload_sort;
    conf.charge_time = dto.charge_time;
    conf.charge_msg_read = dto.charge_msg_read;
    conf.charge_msg_write = dto.charge_msg_write;

    set_path(&mut conf.users_menu, &dto.users_menu);
    set_path(&mut conf.sysop_menu, &dto.sysop_menu);
    set_path(&mut conf.news_file, &dto.news_file);
    set_path(&mut conf.intro_file, &dto.intro_file);
    set_path(&mut conf.attachment_location, &dto.attachment_location);
    set_path(&mut conf.command_file, &dto.command_file);
    set_path(&mut conf.pub_upload_location, &dto.pub_upload_location);
    set_path(&mut conf.pub_upload_metadata, &dto.pub_upload_metadata);
    set_path(&mut conf.private_upload_location, &dto.private_upload_location);
    set_path(&mut conf.private_upload_metadata, &dto.private_upload_metadata);
    set_path(&mut conf.doors_menu, &dto.doors_menu);
    set_path(&mut conf.doors_file, &dto.doors_file);
    set_path(&mut conf.blt_menu, &dto.blt_menu);
    set_path(&mut conf.blt_file, &dto.blt_file);
    set_path(&mut conf.survey_menu, &dto.survey_menu);
    set_path(&mut conf.survey_file, &dto.survey_file);
    set_path(&mut conf.dir_menu, &dto.dir_menu);
    set_path(&mut conf.dir_file, &dto.dir_file);
    set_path(&mut conf.area_menu, &dto.area_menu);
    set_path(&mut conf.area_file, &dto.area_file);

    if dto.clear_password {
        conf.password = Password::PlainText(String::new());
    } else if !dto.new_password.is_empty() {
        conf.password = Password::PlainText(dto.new_password.clone());
    }

    if errors.is_empty() { Ok(()) } else { Err(AdminError::Validation(errors)) }
}

/// Field wise diff over the serialized form, so new DTO fields are covered automatically.
fn diff_conference(old: &ConferenceDto, new: &ConferenceDto) -> Vec<FieldChangeDto> {
    let (Ok(old_value), Ok(new_value)) = (serde_json::to_value(old), serde_json::to_value(new)) else {
        return Vec::new();
    };
    let (Some(old_map), Some(new_map)) = (old_value.as_object(), new_value.as_object()) else {
        return Vec::new();
    };

    let mut changes: Vec<FieldChangeDto> = old_map
        .iter()
        .filter_map(|(field, old_field)| {
            let new_field = new_map.get(field)?;
            if old_field == new_field {
                return None;
            }
            Some(FieldChangeDto {
                field: field.clone(),
                old: json_scalar(old_field),
                new: json_scalar(new_field),
            })
        })
        .collect();

    if new.clear_password {
        changes.push(FieldChangeDto {
            field: "password".to_string(),
            old: "set".to_string(),
            new: "cleared".to_string(),
        });
    } else if !new.new_password.is_empty() {
        changes.push(FieldChangeDto {
            field: "password".to_string(),
            old: "***".to_string(),
            new: "***".to_string(),
        });
    }

    changes.sort_by(|a, b| a.field.cmp(&b.field));
    changes
}

fn json_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn conference_summaries(base: &ConferenceBase) -> Vec<ConferenceSummaryDto> {
    base.iter()
        .enumerate()
        .map(|(index, conf)| ConferenceSummaryDto {
            index,
            name: conf.name.clone(),
            is_public: conf.is_public,
            is_read_only: conf.is_read_only,
            conference_type: conference_type_to_string(&conf.conference_type),
            required_security: security_to_string(&conf.required_security),
            password_set: !conf.password.is_empty(),
        })
        .collect()
}

fn conference_at(base: &ConferenceBase, index: usize) -> Result<&Conference> {
    base.get(index)
        .ok_or_else(|| AdminError::Missing(format!("conference {index} does not exist")))
}

impl AdminService {
    fn conferences_file(&self) -> Result<PathBuf> {
        let config = self.load_config()?;
        let path = conferences_path(&self.root_path, &config);
        if path.as_os_str().is_empty() {
            return Err(AdminError::Missing("no conference file is configured for this board".to_string()));
        }
        Ok(path)
    }

    fn load_conferences(&self) -> Result<(PathBuf, ConferenceBase)> {
        let path = self.conferences_file()?;
        let base = ConferenceBase::load(&path).map_err(|e| AdminError::Load(e.to_string()))?;
        Ok((path, base))
    }

    fn mutate_conferences<F>(&self, fingerprint: &str, actor: &str, action: &str, mutator: F) -> Result<ApplyResultDto>
    where
        F: FnOnce(&mut ConferenceBase) -> Result<Vec<FieldChangeDto>>,
    {
        let _lock = BoardLock::acquire(&self.root_path)?;
        let (path, mut base) = self.load_conferences()?;
        backup::check_fingerprint(&path, fingerprint)?;

        let changes = mutator(&mut base)?;
        if changes.is_empty() {
            return Ok(ApplyResultDto {
                changed_fields: Vec::new(),
                backup: None,
                fingerprint: backup::fingerprint(&path)?,
            });
        }

        write_conferences(&self.root_path, &path, &base, actor, action, &changes)
    }

    pub fn list_conferences(&self) -> Result<ConferenceListResponse> {
        let (path, base) = self.load_conferences()?;
        Ok(ConferenceListResponse {
            conferences: conference_summaries(&base),
            file: path.display().to_string(),
            fingerprint: backup::fingerprint(&path)?,
        })
    }

    pub fn get_conference(&self, index: usize) -> Result<ConferenceResponse> {
        let (path, base) = self.load_conferences()?;
        let conf = conference_at(&base, index)?;
        Ok(ConferenceResponse {
            index,
            settings: to_conference_dto(conf),
            password_set: !conf.password.is_empty(),
            file: path.display().to_string(),
            fingerprint: backup::fingerprint(&path)?,
        })
    }

    pub fn create_conference(&self, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        let patch = normalize_conference(patch)?;
        self.mutate_conferences(fingerprint, actor, "create_conference", |base| {
            let mut conf = Conference::default();
            apply_conference_dto(&mut conf, &patch)?;
            let index = base.len();
            base.push(conf);
            Ok(vec![FieldChangeDto {
                field: format!("conference[{index}]"),
                old: String::new(),
                new: patch.name.clone(),
            }])
        })
    }

    pub fn update_conference(&self, index: usize, patch: &ConferenceDto, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        let patch = normalize_conference(patch)?;
        self.mutate_conferences(fingerprint, actor, "update_conference", |base| {
            let conf = base
                .get_mut(index)
                .ok_or_else(|| AdminError::Missing(format!("conference {index} does not exist")))?;
            let changes = diff_conference(&to_conference_dto(conf), &patch);
            if !changes.is_empty() {
                apply_conference_dto(conf, &patch)?;
            }
            Ok(changes)
        })
    }

    pub fn delete_conference(&self, index: usize, fingerprint: &str, actor: &str) -> Result<ApplyResultDto> {
        self.mutate_conferences(fingerprint, actor, "delete_conference", |base| {
            if index >= base.len() {
                return Err(AdminError::Missing(format!("conference {index} does not exist")));
            }
            if base.len() == 1 {
                return Err(AdminError::Validation(vec!["The last conference cannot be deleted".to_string()]));
            }
            let removed = base.remove(index);
            Ok(vec![FieldChangeDto {
                field: format!("conference[{index}]"),
                old: removed.name.clone(),
                new: String::new(),
            }])
        })
    }
}

/// Shared write path for the conference file: backup, atomic save, read back check, audit.
fn write_conferences(
    root_path: &Path,
    path: &Path,
    base: &ConferenceBase,
    actor: &str,
    action: &str,
    changes: &[FieldChangeDto],
) -> Result<ApplyResultDto> {
    let backup_path = backup::create_backup(root_path, path)?;
    base.save_atomic(&path).map_err(|e| AdminError::Save(e.to_string()))?;

    if let Err(e) = ConferenceBase::load(&path) {
        let _ = std::fs::copy(&backup_path, path);
        return Err(AdminError::Save(format!(
            "written conference file could not be read back ({e}), the backup was restored"
        )));
    }

    backup::append_audit(
        root_path,
        &serde_json::json!({
            "time": chrono::Utc::now().to_rfc3339(),
            "actor": actor,
            "action": action,
            "file": path.display().to_string(),
            "backup": backup_path.display().to_string(),
            "changes": changes.iter().map(|c| serde_json::json!({ "field": c.field, "old": c.old, "new": c.new })).collect::<Vec<_>>(),
        }),
    );

    Ok(ApplyResultDto {
        changed_fields: changes.iter().map(|c| c.field.clone()).collect(),
        backup: Some(backup_path.display().to_string()),
        fingerprint: backup::fingerprint(path)?,
    })
}

impl LiveAdminBackend {
    fn live_conferences_path(&self, config: &IcbConfig) -> Result<PathBuf> {
        let path = conferences_path(&self.root_path, config);
        if path.as_os_str().is_empty() {
            return Err(AdminError::Missing("no conference file is configured for this board".to_string()));
        }
        Ok(path)
    }

    async fn mutate_conferences_live<F>(&self, fingerprint: &str, actor: &str, action: &str, mutator: F) -> Result<ApplyResultDto>
    where
        F: Fn(&mut ConferenceBase) -> Result<Vec<FieldChangeDto>> + Send,
    {
        let _lock = BoardLock::acquire(&self.root_path)?;
        let mut board = self.board.lock().await;
        let path = self.live_conferences_path(&board.config)?;
        backup::check_fingerprint(&path, fingerprint)?;

        let changes = mutator(&mut board.conferences)?;
        if changes.is_empty() {
            return Ok(ApplyResultDto {
                changed_fields: Vec::new(),
                backup: None,
                fingerprint: backup::fingerprint(&path)?,
            });
        }

        // The running board keeps runtime state the file does not have, so the disk image
        // is mutated separately instead of being serialized from memory.
        let mut disk = ConferenceBase::load(&path).map_err(|e| AdminError::Load(e.to_string()))?;
        let _ = mutator(&mut disk)?;

        write_conferences(&self.root_path, &path, &disk, actor, action, &changes)
    }
}

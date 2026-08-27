error_cmd_line_label = error:
error_board_config_not_found = IcyBoard configuration not found: { $path }
error_board_config_help =
    An explicit file or directory is used as given. Without one, IcyBoard looks
    for icboard.toml in the current directory and then at ICB_PATH.

    Usage: { $program } [options] [FILE|DIRECTORY]
    Create a board: icbsetup create mybbs
    Then start it: icboard mybbs
    Command help: { $program } --help
    Guide: https://github.com/mkrueger/icy_board/blob/main/docs/gettingstarted.md
error_input_file_not_found = Input file not found: { $path }
error_input_file_help =
    Usage: { $program } [options] FILE
    Create it: { $program } --create { $path }
    Command help: { $program } --help
error_parent_board_config_not_found = No icboard.toml found for: { $path }
error_parent_board_config_help =
    { $program } looks for icboard.toml in the file's directory and its parents.

    Create a board: icbsetup create mybbs
    Command help: { $program } --help
    Guide: https://github.com/mkrueger/icy_board/blob/main/docs/gettingstarted.md
run_ppe_completed = Run completed - press any key to exit

# Shown for an option the board does not read yet
option_not_read_yet = the board does not act on this yet
option_imported_only = kept from the PCBoard import, the board does not act on it

exit_icy_board_msg = Thank you for using { $name } Professional BBS Software!

# Call wait screen
call_wait_screen_user_button_busy=User - Busy
call_wait_screen_user_button_busy_descr=Log in as a regular user. Callers will get a busy signal.
call_wait_screen_sysop_button_busy=Sysop - Busy
call_wait_screen_sysop_button_busy_descr=Log in as the Sysop. Callers will get a busy signal.
call_wait_screen_dos_button_busy=Shell - Busy
call_wait_screen_dos_button_busy_descr=Drop to Shell. Callers will get a busy signal.

call_wait_screen_user_button_not_busy=User - Not Busy
call_wait_screen_user_button_not_busy_descr=Log in as a regular user. RING Alert will be activated.
call_wait_screen_sysop_button_not_busy=Sysop - Not Busy
call_wait_screen_sysop_button_not_busy_descr=Log in as the Sysop. RING Alert will be activated.
call_wait_screen_dos_button_not_busy=Shell - Not Busy
call_wait_screen_dos_button_not_busy_descr=Drop to Shell. Callers will NOT get a busy signal.

call_wait_screen_call_log_on=Call Log - On
call_wait_screen_call_log_off=Call Log - Off
call_wait_screen_call_log_descr=When on, callers will be logged.

call_wait_screen_page_bell_on=Page Bell is On
call_wait_screen_page_bell_off=Page Bell is Off
call_wait_screen_page_bell_descr=System will BEEP when caller pages you.

call_wait_screen_alarm_on=Alarm is On
call_wait_screen_alarm_off=Alarm is Off
call_wait_screen_alarm_descr=System BEEPs as caller logs on, etc.

call_wait_screen_monitor_button_not_busy=ICBMoni
call_wait_screen_monitor_button_not_busy_descr=Run ICBMoni to monitor NODE activity

call_wait_screen_system_manager=ICBSM
call_wait_screen_system_manager_descr=Run IcyBoard System Manager for User File maintenance.

call_wait_screen_setup=ICBSetup
call_wait_screen_setup_descr=Run ICBSetup to change Icy Board configuration.

call_wait_screen_icb_text=ICBText
call_wait_screen_icb_text_descr=Changes the ICBText files on the system.

call_wait_screen_total_statistics=ALL TIME Statistics
call_wait_screen_today_statistics=TODAY Statistics
call_wait_screen_statistics_descr=Choose between All Time or Today Statistics Display

call_wait_screen_show_statistics=Show Statistics
call_wait_screen_show_statistics_descr=Shows all Statistics for the system

call_wait_screen_sys_ready = System is Ready For Callers
call_wait_screen_last_caller = Last Caller:
call_wait_screen_last_caller_none = None
call_wait_screen_num_calls = Calls:
call_wait_screen_num_msgs = Msgs:
call_wait_screen_num_dls = D/Ls:
call_wait_screen_num_uls = U/Ls:

# System Statistics
icb_system_statistics_title = [ IcyBoard System Statistics ]
icb_system_statistics_footer = [ (↑), (↓), (Del) Reset Stats, (Esc) to End ]
icb_system_statistics_confirm_reset = [ Reset ALL statistics, including the caller number? (Y) to confirm ]
icb_system_statistics_header = Statistic

icb_system_statistics_total_calls = All Time Calls
icb_system_statistics_total_messages = All Time Messages
icb_system_statistics_total_uploads = All Time Uploads
icb_system_statistics_total_uploads_kb = All Time KB Uploaded
icb_system_statistics_total_downloads = All Time Downloads
icb_system_statistics_total_downloads_kb = All Time KB Downloaded

icb_system_statistics_today_calls = Today Calls
icb_system_statistics_today_messages = Today Messages
icb_system_statistics_today_uploads = Today Uploads
icb_system_statistics_today_uploads_kb = Today KB Uploaded
icb_system_statistics_today_downloads = Today Downloads
icb_system_statistics_today_downloads_kb = Today KB Downloaded

# Node Monitioring Utility
icbmoni_title = [ IcyBoard Node Monitoring Utility ]
icbmoni_footer = [ (↑), (↓), (Esc) to End ]
icbmoni_on_note_footer = [ (↑), (↓), (Return) to Monitor, (Esc) to End ]
icbmoni_no_caller = No Caller this Node
icbmoni_user_log_in = Logging Into System
icbmoni_user_browse_menu = Browsing Menus
icbmoni_user_enter_message = Enter Message
icbmoni_comment_to_sysop = Comment to Sysop
icbmoni_user_browse_files = Browsing Files
icbmoni_user_read_messages = Read Messages
icbmoni_user_read_bulletins = Read Bulletins
icbmoni_user_take_survey = Take Survey
icbmoni_user_upload = Upload Files
icbmoni_user_download = Download
icbmoni_user_logoff = Logging off
icbmoni_user_door = Running Door
icbmoni_user_chat_with_sysop = Chat with Sysop
icbmoni_user_group_chat = Group Chat
icbmoni_user_page_sysop = Page Sysop
icbmoni_user_read_broadcast = Read Broadcast

icbmoni_status_header = Status
icbmoni_user_header = User
icbmoni_protocol_header = Protocol

icbmoni_log_in=User logging in…
icbmoni_web_admin_url = Web Admin: { $url }
icbmoni_web_admin_token = Token: { $token }

yes=Yes
no=No
quick_save=Quick

icbtext_save_changes=Save changes?
icbtext_edit_title=Edit Record #{ $number }
icbtext_edit_original_text_title=Original Text:
icbtext_edit_preview_text_title=Preview:
icbtext_edit_edit_text_title=Edit Text:
icbtext_edit_hard_space_info=Use the tilde (~) character to add hard-spaces to the end of a string.
icbtext_edit_justify_left=Left
icbtext_edit_justify_right=Right
icbtext_edit_justify_center=Center
icbtext_edit_justify_title=Justification: { $justify }
icbtext_edit_record_length_title=Record Length: { $number } chars
icbtext_edit_style=Style:

icbtext_filter_title=Filter
icbtext_filter_text=Show filtered entries: { $filter }
icbtext_no_entries=No entries found

icbtext_jump_to_title=Jump to Record #

icbtext_style_plain = Plain
icbtext_style_red = Red
icbtext_style_green = Green
icbtext_style_yellow = Yellow
icbtext_style_blue = Blue
icbtext_style_purple = Purple
icbtext_style_cyan = Cyan
icbtext_style_white = White

icbtext_tab_record=Records
icbtext_tab_about=About

key_desc_quit=Quit
key_desc_back=Back
key_desc_next_prev_style=Next/Prev Style
key_desc_restore=Restore
key_desc_accept=Accept
key_desc_cancel=Cancel
key_desc_filter=Filter
key_desc_jump=Jump
key_desc_edit=Edit

# ICBSetup

icb_setup_key_main_help=↑ Up  ↓ Down  F1 Help  ␛ Quit
icb_setup_key_menu_help=↑ Up  ↓ Down  F1 Help  ␛ Back
icb_setup_key_menu_edit_help=↑ Up  ↓ Down  F1 Help  F2 Edit this file  ␛ Back
icb_setup_key_menu_create_help=↑ Up  ↓ Down  F1 Help  F3 Create file  ␛ Back
icb_setup_key_conf_list_help=↑ Up  ↓ Down  INS New  ␡ Delete  PgUp/Dn Move ␛ Back

icb_setup_main_title=Main Menu
icb_setup_main_use_label=Use /w ICB { $version }
icb_setup_save_failed=Can't save: { $error }
icb_setup_main_sysop_info=Sysop Information
icb_setup_main_sysop_info-help=
    # Sysop Information
    
    The Sysop Information screen contains such items as the sysop's name,
    local logon password, graphics default, etc.
    
    These are items that are about the sysop which are not stored in the
    sysop record of the sysop users file.

icb_setup_main_file_locs=File Locations
icb_setup_main_file_locs-help=
    # File Locations

    File Locations is a menu that splits Icy Board's system paths and
    file names into several input screens.

icb_setup_main_con_info=Connection Information
icb_setup_main_con_info-help=
    # Connection Information

    Connection Information is a menu for a few screens of information
    relating to how users to connect to Icy Board.

icb_setup_main_board_cfg=Board Configuration
icb_setup_main_board_cfg-help=
    # Board Configuration

    The Board Configuration screen stores information about the board itself.

icb_setup_main_evt_setup=Event Setup
icb_setup_main_evt_setup-help=
    # Event Setup

    The Event Setup screen provides general information for running events.
    The F2 key on the EVENT.DAT file takes you into the full event config.

icb_setup_main_subscription=Subscription
icb_setup_main_subscription-help=
    # Subscription

    The Subscription screen contains information pertaining to the setup
    and operation of a subscription system where new callers are provided
    a set number of days in their subscription and then security levels 
    are modified according to the information provided.

icb_setup_main_conf_opt=Configuration Options
icb_setup_main_conf_opt-help=
    # Configuration Options

    Configuration Options leads to another menu for several screens 
    worth of information used to customize the behavior of a Icy Board system.

icb_setup_main_sec_levels=Security Levels
icb_setup_main_sec_levels-help=
    # Security Levels

    Security Levels leads to a menu of more choices for setting 
    up security levels for sysop functions, sysop commands and user
    commands.

icbsm_define_editors=Define Text & Graphics Editors
icbsm_customize_colors=Customize Colors
icbsm_text_editor=Text Editor
icbsm_graphics_editor=Graphics Editor
icbsm_color_title=Color Customization
icbsm_color_default_1=Default Color Set #1
icbsm_color_default_2=Default Color Set #2
icbsm_color_bw=Default B&W Colors
icbsm_color_customize=Customize Colors

icb_setup_main_acc_cfg=Accounting Configuration
icb_setup_main_acc_cfg-help=
    # Accounting Configuration

    Accounting Configuration is an optional component of a Icy Board system.

    The accounting configuration screens allow you to define 
    the costs or rewards for various activities on your BBS and to
    also define Peak Usage times as well as holidays.

icb_setup_main_new_user=New User Options
icb_setup_main_new_user-help=
    # New User Options

    New User Options is a menu for setting up the default settings
    and questions for new users.

    This includes the default security level, the default groups, and the
    default questions to ask new users.

icb_setup_msg_networking=Messaging & Networking
icb_setup_msg_networking-help=
    # Messaging & Networking

    Messaging & Networking is a menu for setting up the QWK/FTN etc. settings
    for Icy Board.

icb_setup_mb_conf=Main Board Configuration
icb_setup_mb_conf-help=
    # Main Board Configuration

    The Main Board Configuration screen contains the definitions
    required for the main board including download paths, bulletins,
    scripts and menus.

icb_setup_conferences=Conferences
icb_setup_conferences-help=
    # Conferences

    The Conferences selection brings up a list of conferences that
    can be selected for editing.

    Conferences can be added, deleted, rearranged or edited.

board_config_title=Board Configuration

board_name=Board Name
board_name-status=Board name is shown on login to the caller
board_name-help=
    # Board Name

    Enter here the name of your bulletin board system. This name is displayed
    at connect to the caller.

allow_iemsi=Allow IEMSI
allow_iemsi-status=Allow IEMSI login
allow_iemsi-help=
    # Allow IEMSI

    IEMSI is an automatic login/capability exchange used by advanced BBS clients. 
    The caller sends name, terminal features (ANSI/color, size), 
    and preferred protocols so IcyBoard can skip most prompts and tailor output immediately. 

    If no IEMSI data arrives, normal interactive login is used. 
    Because classic IEMSI isn’t encrypted, prefer it on trusted or secured connections.

board_iemsi_location=Location
board_iemsi_location-status=Board location used in IEMSI
board_iemsi_location-help=
    # Location

    Human-readable board location (city/region) advertised in the IEMSI handshake.

board_iemsi_operator=Operator
board_iemsi_operator-status=Board operator used in IEMSI
board_iemsi_operator-help=
    # Operator

    Name or handle of the board operator/sysop sent to the client.

board_iemsi_notice=Notice
board_iemsi_notice-status=Board notice used in IEMSI
board_iemsi_notice-help=
    # Notice

    Short welcome / status blurb shown by capable IEMSI clients after login.

board_iemsi_caps=Capabilities
board_iemsi_caps-status=Board capabilities used in IEMSI
board_iemsi_caps-help=
    # Capabilities

    Capability flags string (e.g. ANSI,COLOR,RIP,DOORS,MAIL) describing supported features.
    Used in IEMSI handshake.

board_node_num=# Nodes
board_node_num-status=Number of maximum active nodes
board_node_num-help=
    # Nodes

    Maximum number of nodes allowed. Prevents DDoS attacks.

who_include_city=Include City Field in WHO Display
who_include_city-status=Include City Field in WHO Display
who_include_city-help=
    # Include City Field in WHO Display

    When a user types WHO at the command prompt in IcyBoard, this 
    setting will determine if the city field of each user online is
    included in the list.

web_admin_enabled=Enable Web Administration
web_admin_enabled-status=Start the web administration server with IcyBoard
web_admin_enabled-help=
    # Enable Web Administration

    Starts the web administration interface when IcyBoard is running.
    It is disabled by default and does not replace icbsetup or icbsm.

web_admin_address=Web Admin Address
web_admin_address-status=Address used by the web administration server
web_admin_address-help=
    # Web Admin Address

    Network address on which the web administration server listens.
    Keep this at 127.0.0.1 unless remote access is explicitly required.

web_admin_port=Web Admin Port
web_admin_port-status=TCP port used by the web administration server
web_admin_port-help=
    # Web Admin Port

    TCP port on which the web administration server listens.
    The default port is 8787.

web_admin_allow_remote=Allow Remote Web Admin
web_admin_allow_remote-status=Permit the web administration server to listen outside localhost
web_admin_allow_remote-help=
    # Allow Remote Web Administration

    Allows the administration server to bind to a non-loopback address.
    This exposes board administration to the network. Only enable it behind
    an authenticated TLS reverse proxy and with a strong access token.

who_show_alias=Show ALIAS Name in WHO Display
who_show_alias-status=Show ALIAS Name in WHO Display
who_show_alias-help=
    # Show ALIAS Name in WHO Display

    Switch to display ALIAS instead of names in the WHO display.

date_format=Date Format
date_format-status=Date format used in the system
date_format-help=
    # Date Format

    The default Date Format used by IcyBoard.

new_user_options_title=New User Options

new_user_options_ask_label=Ask New Users for:

new_user_security_level=Security Level
new_user_security_level-status=Default security level for new users
new_user_security_level-help=
    # Security Level

    The security level a caller receives when they agree to register.
    It decides which commands, conferences and file areas are open to them.
    A level of 0 locks the caller out of the board.

allow_one_name_users=Allow One Name Users
allow_one_name_users-status=Allow single name users
allow_one_name_users-help=
    # Allow One Name Users

    Whether a caller may register with a single name instead of a first and
    a last name. Turn it off to keep the user list to real, full names.

auto_register_conferences=Register New Users in Public Conferences
auto_register_conferences-status=A new caller starts out registered in every public conference
auto_register_conferences-help=Conferences that carry a security requirement of their own are left out.

new_user_groups=New User Default Groups
new_user_groups-status=Default groups for new users
new_user_groups-help=
    # New User Default Groups

    The groups a new caller is put into, separated by commas.
    Groups are what security expressions in menus, conferences and file areas
    test against, so this is how a new caller gets access that is not tied to
    a plain security level.

ask_city_or_state=City or State
ask_city_or_state-status=Ask for city or state
ask_city_or_state-help=
    # City or State

    Ask a new caller where they are calling from, and let them change it later
    with the W command. The answer is shown in the WHO listing when the board
    is set to include the city.

ask_address=Address
ask_address-status=Ask for address
ask_address-help=
    # Address

    Ask a new caller for a postal address, and let them change it later with
    the W command. Leave it off unless the board really needs the address,
    a mail-in subscription for instance.

ask_verification=Verification
ask_verification-status=Ask for verification
ask_verification-help=
    # Verification

    Ask a new caller for an answer only they would know, a mother's maiden name
    for instance. The sysop can ask for it again later to confirm that somebody
    calling for their account back really is that caller.

ask_bus_data_phone=Business Phone
ask_bus_data_phone-status=Ask for business data phone
ask_bus_data_phone-help=
    # Business Phone

    Ask a new caller for a business or data phone number, and let them change it
    later with the W command.

ask_home_phone=Home Phone
ask_home_phone-status=Ask for home phone
ask_home_phone-help=
    # Home Phone

    Ask a new caller for a home or voice phone number, and let them change it
    later with the W command.

ask_comment=Comment
ask_comment-status=Ask for comment
ask_comment-help=
    # Comment

    Ask a new caller for a line about themselves. It is kept in the user record
    and the sysop sees it in the user editor.

ask_clr_msg=Clear Message
ask_clr_msg-status=Ask for clear message
ask_clr_msg-help=
    # Clear Message

    Ask a new caller whether the screen should be cleared between messages.
    Callers on a slow or scrollback-keeping terminal usually say no.

ask_fse=Full Screen Editor
ask_fse-status=Ask for full screen editor
ask_fse-help=
    # Full Screen Editor

    Ask a new caller whether they want to write messages in the full screen
    editor instead of the line editor.

ask_xfer_protocol=Protocols
ask_xfer_protocol-status=Ask for transfer protocol
ask_xfer_protocol-help=
    # Protocols

    Ask a new caller which transfer protocol to use by default, taken from the
    protocol list. Leave it off to start everybody on the board default and let
    them pick one later with the T command.

ask_date_format=Date Format
ask_date_format-status=Ask for date format
ask_date_format-help=
    # Date Format

    Ask a new caller which date format they want to see, chosen from the formats
    the board offers. Without it every caller starts on the board default.

ask_alias=Alias
ask_alias-status=Ask for alias
ask_alias-help=
    # Alias

    Ask a new caller for a handle to appear under. An alias is only used in
    conferences that allow aliases, and whether it can be changed afterwards
    is decided by 'Allow Alias Change'.

ask_gender=Gender
ask_gender-status=Ask for Gender
ask_gender-help=
    # Gender

    Ask a new caller for their gender. It is kept in the user record and can be
    read from PPE programs.

ask_birthdate=Birthdate
ask_birthdate-status=Ask for birthdate
ask_birthdate-help=
    # Birthdate

    Ask a new caller for their date of birth. It is kept in the user record and
    is what an age check in a PPE or a birthday greeting can read.

ask_email=Email
ask_email-status=Ask for email
ask_email-help=
    # Email

    Ask a new caller for an e-mail address, and let them change it later with
    the W command.

ask_web_address=Web Address
ask_web_address-status=Ask for web address
ask_web_address-help=
    # Web Address

    Ask a new caller for the address of their home page, and let them change it
    later with the W command.

ask_use_short_descr=Short Description
ask_use_short_descr-status=Ask for short description
ask_use_short_descr-help=
    # Short Description

    Ask a new caller whether file listings should show only the first line of
    each description. It keeps a listing short on a small screen; the caller can
    change it later with the W command.

subscription_information_title=Subscription Information

subscription_is_enabled=Enable Subscription Mode
subscription_is_enabled-status=Enables account expiration support.
subscription_is_enabled-help=
    # Enable Subscription Mode

    Whether the board looks at the expiration date in each user record at login.
    While it is off an expiration date is ignored, however old it is. While it is
    on, a caller past their date is put on the expired security level.

subscription_length=Default Subscription Length in Days
subscription_length-status=Number of days before account expiration.
subscription_length-help=
    # Default Subscription Length in Days

    How many days a new caller's subscription runs, counted from the day they
    register. 365 gives a year; 0 leaves the account without an expiration date,
    so it never expires.

default_expired_level=Default 'Expired' Security Level
default_expired_level-status=Security level for expired accounts.
default_expired_level-help=
    # Default 'Expired' Security Level

    The security level a new caller falls back to once their subscription runs
    out. It starts out the same as the new user level, so set it lower to leave
    an expired caller with less than a paying one, or to 0 to lock them out.

warning_days=Warning Days Prior to Expiration
warning_days-status=Displays WARNING file to the caller prior to expiration.
warning_days-help=
    # Warning Days Prior to Expiration

    How many days before the expiration date the WARNING file is shown at login,
    so a caller has time to renew. 0 turns the warning off.

sysop_information_title=Sysop Information

sysop_name=Sysop's Name
sysop_name-status=When NOT using the real name.
sysop_name-help=
    # Sysop Name

    Enter the first name of the sysop.
    
    NOTE: Do NOT use your full name.
    This is just the sysop's first name.
    Your FULL NAME should be entered in record #1 of the USERS file via `icbsm`.

local_password=Local Password
local_password-status=Call waiting screen password.
local_password-help=
    # Local Password

    Enter the password that the sysop will enter at the LOCAL station
    to get into Icy Board (at the call waiting screen).

require_password_to_exit=Require Password to Exit
require_password_to_exit-status=IcyBoard requires pw to exit the call waiting screen.
require_password_to_exit-help=
    # Require Password to Exit

    If this is set to YES, the sysop will be required to enter the
    local password to exit the call waiting screen.

sys_info_external_editor=External Editor
sys_info_external_editor-status=External editor for sysop messages.
sys_info_external_editor-help=
    # External Editor

    Enter the name of the external editor that the sysop will use
    to edit general text files in `icbsetup`.

sys_info_graphics_editor=Graphics Editor
sys_info_graphics_editor-status=Editor for ANSI and graphics files.
sys_info_graphics_editor-help=
    # Graphics Editor

    Enter the name of the editor used for ANSI and graphics files.

sys_info_theme=Color Theme
sys_info_theme-status=Color Theme
sys_info_theme-help=
    # Color Theme

    Enter the color theme that the `icbsetup` is using.

use_real_name=Use Real Name
use_real_name-status=Use the real name of the sysop.
use_real_name-help=
    # Use Real Name

    If you answer 'N' to this question then, when leaving messages
    the system will record the message as having been left by the
    SYSOP.
    
    Answering 'Y' will cause it to use the sysop name that is found
    in record #1 of the users file.

sec_level_menu_title=Security Levels
sec_level_menu_sysop_funcs=Sysop Functions
sec_level_menu_sysop_commands=Sysop Commands
sec_level_menu_user_commands=User Commands

sysop_commands_title=Sysop Commands

sysop_sec_level=Sysop Level
sysop_sec_level-status=For Sysop Menu and F1-Temp-Sysop Upgrade
sysop_sec_level-help=
    # Sysop Level

    The level from which a caller counts as a sysop. They see the sysop menu
    instead of the user menu, and this is the level handed out by a temporary
    upgrade or by the conference sysop flag. It does not by itself grant the
    individual sysop commands below, those keep their own levels.

sysop_sec_read_all_comments=Level Needed to Read All Comments
sysop_sec_read_all_comments-status=Level Needed to Read All Comments
sysop_sec_read_all_comments-help=
    # Level Needed to Read All Comments

    Comments left with the C command are the most private mail on the board,
    so whoever may read them may read every other message as well.

sysop_sec_read_all_mail=Level Needed to Read All Mail Except Comments
sysop_sec_read_all_mail-status=Level Needed to Read All Mail Except Comments
sysop_sec_read_all_mail-help=
    # Level Needed to Read All Mail

    Reading private mail that is neither from nor to the caller. Comments to the
    sysop stay behind the level above.

sysop_sec_copy_move_messages=Level Needed to Copy or Move Messages Between Areas
sysop_sec_copy_move_messages-status=Level Needed to Copy or Move Messages Between Areas
sysop_sec_copy_move_messages-help=
    # Level Needed to Copy or Move Messages

    Copying or moving a message into another conference from the end of message
    prompt.

sysop_sec_enter_color_codes_in_messages=Level Needed to Enter @-Variables in Message Base
sysop_sec_enter_color_codes_in_messages-status=Level Needed to Enter @-Variables in Message Base
sysop_sec_enter_color_codes_in_messages-help=
    # Level Needed to Enter @-Variables in Messages

    Writing @ macros such as @USER@ or @MORE@ into a message, which are expanded
    when the message is read. @X colour codes are open to everybody.

sysop_sec_edit_any_message=Level Needed to Edit Any Message in the Message Base
sysop_sec_edit_any_message-status=Level Needed to Edit Any Message in the Message Base
sysop_sec_edit_any_message-help=
    # Level Needed to Edit Any Message

    Editing any message the caller can read, not only their own. Keep it above
    the user level for editing one's own messages: whoever has it can make
    somebody else appear to have written something.

sysop_sec_not_update_msg_read=Level Needed to NOT Update Msg Read Status
sysop_sec_not_update_msg_read-status=R O cmd
sysop_sec_not_update_msg_read-help=
    # Level Needed to NOT Update Msg Read Status

    Reading with R O, which leaves the last read pointer where it was. A caller
    who has it can read mail without the board recording that they did.

sysop_sec_use_broadcast_command=Level Needed to Use the BROADCAST Command
sysop_sec_use_broadcast_command-status=BR command
sysop_sec_use_broadcast_command-help=
    # Level Needed to Use the BROADCAST Command

    Sending a one line message with BR to a caller on another node, or to every
    node at once.

sysop_sec_view_private_uploads=Level Needed to View the Private Upload Directory
sysop_sec_view_private_uploads-status=Level Needed to View the Private Upload Directory
sysop_sec_view_private_uploads-help=
    # Level Needed to View the Private Upload Directory

    Listing the private upload directory of the current conference, where new
    uploads wait until they are moved into a public directory.

sysop_sec_enter_generic_messages=Level Needed to Enter Generic Messages
sysop_sec_enter_generic_messages-status=Messages to @USER@
sysop_sec_enter_generic_messages-help=
    # Level Needed to Enter Generic Messages

    Addressing a message to @USER@ or to a security range, so that one message
    reaches many callers and looks personal to each of them.

sysop_sec_edit_message_headers=Level Needed to Edit Message Headers
sysop_sec_edit_message_headers-status=Level Needed to Edit Message Headers
sysop_sec_edit_message_headers-help=
    # Level Needed to Edit Message Headers

    Changing the header of a message: who it is from, who it is addressed to and
    how it is protected.

sysop_sec_protect_unprotect_messages=Level Needed to Protect/Unprotect a Message
sysop_sec_protect_unprotect_messages-status=Level Needed to Protect/Unprotect a Message
sysop_sec_protect_unprotect_messages-help=
    # Level Needed to Protect/Unprotect a Message

    Turning a message private or public again at the end of message prompt.

sysop_sec_overwrite_files_on_uploads=Level Needed to Overwrite Files on Uploads
sysop_sec_overwrite_files_on_uploads-status=Level Needed to Overwrite Files on Uploads
sysop_sec_overwrite_files_on_uploads-help=
    # Level Needed to Overwrite Files on Uploads

    Uploading a file the board already has: the caller may replace the old file,
    keep both, or abandon the upload instead of simply being refused.

sysop_sec_set_pack_out_date_on_messages=Level Needed to Set the Pack-Out Date on Messages
sysop_sec_set_pack_out_date_on_messages-status=Level Needed to Set the Pack-Out Date on Messages
sysop_sec_set_pack_out_date_on_messages-help=
    # Level Needed to Set the Pack-Out Date

    Giving a message a date on which it is thrown away by itself, which suits an
    announcement that stops being true on a known day.

sysop_sec_see_all_return_receipts=Level Needed to See All Return Receipt Messages
sysop_sec_see_all_return_receipts-status=Level Needed to See All Return Receipt Messages
sysop_sec_see_all_return_receipts-help=
    # Level Needed to See All Return Receipts

    A return receipt is normally shown only to the caller who asked for one.
    This level lets somebody see the receipts that belong to others.

sysop_functions_title=Sysop Functions

sysop_sec_1_view_caller_log=(1) View/Print Caller Log
sysop_sec_1_view_caller_log-status=View/Print Caller Log
sysop_sec_1_view_caller_log-help=
    # (1) View Caller Log

    Reading the caller log, which records who called and what they did.
    Sysop levels are usually 100 or higher.

sysop_sec_2_view_usr_list=(2) View/Print User List
sysop_sec_2_view_usr_list-status=View/Print User List
sysop_sec_2_view_usr_list-help=
    # (2) View User List

    Listing the user file with the sysop command, which shows more than the
    caller-visible user search.

sysop_sec_3_pack_renumber_msg=(3) Pack Renumber Messages
sysop_sec_3_pack_renumber_msg-status=Pack Renumber Messages
sysop_sec_3_pack_renumber_msg-help=
    # (3) Pack/Renumber Messages

    Packing and renumbering a message base, which drops killed messages and
    rewrites the numbering. It rewrites the base, so keep it high.

sysop_sec_4_recover_deleted_msg=(4) Recover Killed Message
sysop_sec_4_recover_deleted_msg-status=Recover Killed Message
sysop_sec_4_recover_deleted_msg-help=
    # (4) Recover Deleted Message

    Bringing a killed message back, as long as the base has not been packed
    since it was killed.

sysop_sec_5_list_message_hdr=(5) List Message Headers
sysop_sec_5_list_message_hdr-status=List Message Headers
sysop_sec_5_list_message_hdr-help=
    # (5) List Message Headers

    Listing message headers only, which is the quick way to look over a base
    without reading the messages themselves.

sysop_sec_6_view_any_file=(6) View Any File
sysop_sec_6_view_any_file-status=View Any File
sysop_sec_6_view_any_file-help=
    # (6) View Any File

    Viewing any file on the system, wherever it is and whether or not it sits in
    a file directory the caller may list.

sysop_sec_7_user_maint=(7) User Maintenance
sysop_sec_7_user_maint-status=User Maintenance
sysop_sec_7_user_maint-help=
    # (7) User Maintenance

    Editing user records from the board: security level, flags, expiration date
    and the rest of the record.

sysop_sec_8_pack_usr_file=(8) Pack User File
sysop_sec_8_pack_usr_file-status=Pack User File
sysop_sec_8_pack_usr_file-help=
    # (8) Pack User File

    Packing the user file, which removes the records that match the criteria
    given and rewrites the file.

sysop_sec_9_exit_to_dos=(9) Exit to Shell remote
sysop_sec_9_exit_to_dos-status=Exit to Shell remote
sysop_sec_9_exit_to_dos-help=
    # (9) Exit to the Shell

    In PCBoard this was the level needed to drop the board to DOS from remote.

sysop_sec_10_shelled_dos_func=(10) Run PPE
sysop_sec_10_shelled_dos_func-status=Security required to run a PPE from the command prompt
sysop_sec_10_shelled_dos_func-help=PCBoard used this level for command 10 and its DOS shell. IcyBoard has no DOS shell; the level protects the PPE command instead.

sysop_sec_11_view_other_nodes=(11) View Other Nodes
sysop_sec_11_view_other_nodes-status=View Other Nodes
sysop_sec_11_view_other_nodes-help=
    # (11) View Other Nodes

    Seeing who is online on the other nodes.

sysop_sec_12_logoff_alt_node=(12) Logoff Alternate Node
sysop_sec_12_logoff_alt_node-status=Logoff Alternate Node
sysop_sec_12_logoff_alt_node-help=
    # (12) Log Off Alternate Node

    Logging off a caller who is online on another node.

sysop_sec_13_view_alt_node_callers=(13) View Alt Node Callers
sysop_sec_13_view_alt_node_callers-status=View Alt Node Callers
sysop_sec_13_view_alt_node_callers-help=
    # (13) View Alternate Node Callers

    Reading the caller log of another node, not only of this one.

sysop_sec_14_drop_alt_node_to_dos=(14) Drop Alt Node to DOS
sysop_sec_14_drop_alt_node_to_dos-status=Drop Alt Node to DOS
sysop_sec_14_drop_alt_node_to_dos-help=
    # (14) Drop Alternate Node to the Shell

    In PCBoard this was the level needed to force another node out to DOS.

user_commands_title=User Commands

user_sec_cmd_a=A) Abandon Conference
user_sec_cmd_a-status=Level required to leave a conference
user_sec_cmd_a-help=
    # A) Abandon Conference

    Leaving the current conference and returning to the main board.
    A caller who was auto-joined into a conference and cannot abandon it is
    stuck there, so keep this within reach of everybody.

user_sec_cmd_b=B) Bulletin Listings
user_sec_cmd_b-status=Level required to read bulletins
user_sec_cmd_b-help=
    # B) Bulletin Listings

    Listing and reading the bulletins of the current conference.

user_sec_cmd_c=C) Comment to Sysop
user_sec_cmd_c-status=Level required to comment to the sysop
user_sec_cmd_c-help=
    # C) Comment to Sysop

    Leaving a comment for the sysop. It is an ordinary message, but addressed to
    the sysop and protected by the level for reading all comments.

user_sec_cmd_d=D) Download a File
user_sec_cmd_d-status=Level required to download files
user_sec_cmd_d-help=
    # D) Download a File

    Downloading a file and flagging files for download. More than one file at a
    time also needs the batch transfer level, and the file directory itself can
    require a level of its own.

user_sec_cmd_e=E) Enter a Message
user_sec_cmd_e-status=Level required to write messages
user_sec_cmd_e-help=
    # E) Enter a Message

    Writing a message. A conference can demand a higher level of its own, and a
    read-only conference refuses messages whatever this says.

user_sec_cmd_f=F) File Directory
user_sec_cmd_f-status=Level required to list file directories
user_sec_cmd_f-help=
    # F) File Directory

    Listing the file directories of the current conference. A single directory
    can still be closed off by its own list security.

user_sec_cmd_h=H) Help Functions
user_sec_cmd_h-status=Level required to read help files
user_sec_cmd_h-help=
    # H) Help Functions

    Reading the help files. Keep it low enough that a brand new caller can read
    the help before they have earned anything else.

user_sec_cmd_i=I) Initial Welcome
user_sec_cmd_i-status=Level required to see the welcome screen again
user_sec_cmd_i-help=
    # I) Initial Welcome

    Showing the welcome screen again after login.

user_sec_cmd_j=J) Join a Conference
user_sec_cmd_j-status=Level required to join conferences
user_sec_cmd_j-help=
    # J) Join a Conference

    Joining another conference. A private conference additionally requires the
    caller to be registered in it, and a conference may demand its own level.

user_sec_cmd_k=K) Kill a Message
user_sec_cmd_k-status=Level required to kill messages
user_sec_cmd_k-help=
    # K) Kill a Message

    Killing a message. A message that the caller cannot read cannot be killed
    either, and callers may only kill their own mail.

user_sec_cmd_l=L) Locate File Name
user_sec_cmd_l-status=Level required to search for file names
user_sec_cmd_l-help=
    # L) Locate File Name

    Searching the file listings for a name or a wildcard, across the directories
    the caller may list.

user_sec_cmd_m=M) Graphics Mode
user_sec_cmd_m-status=Level required to switch graphics mode
user_sec_cmd_m-help=
    # M) Graphics Mode

    Switching between plain text and graphics, which also covers the M CTTY and
    M ANSI forms of the command.

user_sec_cmd_n=N) New File Scan
user_sec_cmd_n-status=Level required to scan for new files
user_sec_cmd_n-help=
    # N) New File Scan

    Scanning the file directories for everything uploaded since the caller last
    looked.

user_sec_cmd_o=O) Operator Page
user_sec_cmd_o-status=Level required to page the sysop
user_sec_cmd_o-help=
    # O) Operator Page

    Paging the sysop for a chat. The page only rings while the page bell is on
    and the call falls inside the sysop's hours.

user_sec_cmd_p=P) Page Length
user_sec_cmd_p-status=Level required to set the page length
user_sec_cmd_p-help=
    # P) Page Length

    Setting how many lines are printed before the board pauses.

user_sec_cmd_q=Q) Quick Message Scan
user_sec_cmd_q-status=Level required to scan message headers
user_sec_cmd_q-help=
    # Q) Quick Message Scan

    Scanning message headers without reading the messages. It usually carries
    the same level as reading messages.

user_sec_cmd_r=R) Read Messages
user_sec_cmd_r-status=Level required to read messages
user_sec_cmd_r-help=
    # R) Read Messages

    Reading messages. Private mail that is neither from nor to the caller stays
    behind the sysop level for reading all mail.

user_sec_cmd_s=S) Surveys
user_sec_cmd_s-status=Level required to answer surveys
user_sec_cmd_s-help=
    # S) Surveys

    Answering a survey. Each survey can also carry a security level of its own.

user_sec_cmd_t=T) Transfer Protocol
user_sec_cmd_t-status=Level required to change the transfer protocol
user_sec_cmd_t-help=
    # T) Transfer Protocol

    Choosing the default transfer protocol from the protocol list.

user_sec_cmd_u=U) Upload a File
user_sec_cmd_u-status=Level required to upload files
user_sec_cmd_u-help=
    # U) Upload a File

    Uploading a file. More than one file at a time also needs the batch transfer
    level, and uploads land in the upload directory of the conference.

user_sec_cmd_v=V) View Settings
user_sec_cmd_v-status=Level required to view own settings
user_sec_cmd_v-help=
    # V) View Settings

    Showing the caller their own record: security, time used, transfer counts
    and the settings they chose.

user_sec_cmd_w=W) Write User Info
user_sec_cmd_w-status=Level required to change own user information
user_sec_cmd_w-help=
    # W) Write User Info

    Changing their own record: password, location, phone numbers and the
    preferences the new user questions asked for.

user_sec_cmd_x=X) Expert Mode Toggle
user_sec_cmd_x-status=Level required to toggle expert mode
user_sec_cmd_x-help=
    # X) Expert Mode Toggle

    Switching between the short expert prompts and the full menus.

user_sec_cmd_y=Y) Your Personal Mail
user_sec_cmd_y-status=Level required to scan for own mail
user_sec_cmd_y-help=
    # Y) Your Personal Mail

    Scanning for mail addressed to the caller. It usually carries the same level
    as reading messages.

user_sec_cmd_z=Z) Zippy DIR Scan
user_sec_cmd_z-status=Level required to search file descriptions
user_sec_cmd_z-help=
    # Z) Zippy DIR Scan

    Searching the text of the file descriptions, which finds a file by what it
    does rather than by its name.

user_sec_cmd_chat=Group CHAT
user_sec_cmd_chat-status=Level required to join group chat
user_sec_cmd_chat-help=
    # Group CHAT

    Joining the group chat between nodes, and with it the commands that make a
    caller available or unavailable for chat.

user_sec_cmd_open_door=OPEN a DOOR
user_sec_cmd_open_door-status=Level required to run doors
user_sec_cmd_open_door-help=
    # OPEN a DOOR

    Running a door program. A single door can require a higher level or a
    password of its own.

user_sec_cmd_test_file=TEST a File
user_sec_cmd_test_file-status=Level required to test files
user_sec_cmd_test_file-help=
    # TEST a File

    Testing whether an archive on the board is intact before spending download
    time on it.

user_sec_cmd_show_user_list=USER Search/Display
user_sec_cmd_show_user_list-status=Level required to search the user list
user_sec_cmd_show_user_list-help=
    # USER Search/Display

    Listing or searching the callers registered in the current conference.
    A conference can forbid this outright with its own setting.

user_sec_cmd_who=WHO is on Another Node
user_sec_cmd_who-status=Level required to see other nodes
user_sec_cmd_who-help=
    # WHO is on Another Node

    Showing who is connected on the other nodes. It has nothing to show on a
    board that runs a single node.

user_sec_batch_file_transfer=Level Required for BATCH File Transfers
user_sec_batch_file_transfer-status=Level required for batch transfers
user_sec_batch_file_transfer-help=
    # Level Required for BATCH File Transfers

    Transferring more than one file per turn, upwards and downwards. The caller
    still needs the plain upload or download level as well.

user_sec_edit_own_messages=Level Required to EDIT Your Own Messages
user_sec_edit_own_messages-status=Level required to edit own messages
user_sec_edit_own_messages-help=
    # Level Required to EDIT Your Own Messages

    Editing a message the caller wrote themselves after it was saved, which
    repairs a typo or a message cut short by a lost connection.

configuration_options_title=Configuration Options
configuration_options_messages=Messages
configuration_options_file_transfer=File Transfers
configuration_options_system_control=System Control
configuration_options_config_switches=Configuration Switches
configuration_options_limits=Limits
configuration_options_colors=Colors
configuration_options_func_keys=Function Keys
configuration_options_ppl_http=PPL HTTP

ppl_http_title=PPL HTTP Policy
ppl_http_policy=Destination policy
ppl_http_policy-status=Choose which outbound destinations PPL programs may contact.
ppl_http_policy-help=
    # Destination policy

    Disabled rejects every PPL HTTP request. Allowlist permits only the exact
    origins below. Public permits hosts only when every resolved address is
    publicly routable. Every redirect is checked again.
ppl_http_policy_disabled=Disabled
ppl_http_policy_allowlist=Exact origin allowlist
ppl_http_policy_public=Public destinations

ppl_http_allowed_origins=Allowed origins
ppl_http_allowed_origins-status=Comma-separated origins used by allowlist mode.
ppl_http_allowed_origins-help=
    # Allowed origins

    Exact origins separated by commas, for example
    https://api.example.com, https://files.example.com:8443

    Scheme, host and port must all match. Paths do not belong here. An
    allowlisted origin may deliberately resolve to a private service.

ppl_http_allow_http=Allow plain HTTP
ppl_http_allow_http-status=Permit unencrypted http:// destinations.
ppl_http_allow_http-help=
    # Allow plain HTTP

    Off permits HTTPS only. Turn this on only for origins that cannot use TLS;
    request headers and bodies sent over HTTP can be read or changed in transit.

ppl_http_max_response_bytes=Maximum response bytes
ppl_http_max_response_bytes-status=Largest response body a PPL request may receive.
ppl_http_max_response_bytes-help=
    # Maximum response bytes

    The transfer stops when the decoded response body crosses this limit.
    Downloads keep their previous destination file when that happens.

ppl_http_max_request_bytes=Maximum request bytes
ppl_http_max_request_bytes-status=Largest POST body a PPL request may send.
ppl_http_max_request_bytes-help=
    # Maximum request bytes

    Requests larger than this limit are rejected before a connection is made.

ppl_http_max_headers=Maximum headers
ppl_http_max_headers-status=Maximum number of request or response headers.
ppl_http_max_headers-help=
    # Maximum headers

    Applies to both headers supplied by a PPL program and headers returned by a
    server.

ppl_http_max_header_bytes=Maximum header bytes
ppl_http_max_header_bytes-status=Combined byte limit for request or response headers.
ppl_http_max_header_bytes-help=
    # Maximum header bytes

    Applies independently to each request and response header block.

ppl_http_connect_timeout=Connect timeout seconds
ppl_http_connect_timeout-status=Maximum time allowed to establish one connection.
ppl_http_connect_timeout-help=
    # Connect timeout

    Bounds one connection attempt. The total request timeout still covers DNS,
    queueing, redirects and the response body.

ppl_http_request_timeout=Total timeout seconds
ppl_http_request_timeout-status=Total deadline for one PPL HTTP operation.
ppl_http_request_timeout-help=
    # Total request timeout

    Includes waiting for concurrency capacity, DNS, every redirect, connection
    setup and the complete response body.

ppl_http_max_redirects=Maximum redirects
ppl_http_max_redirects-status=Maximum redirect hops followed by one request.
ppl_http_max_redirects-help=
    # Maximum redirects

    Zero rejects redirects. Each permitted hop must pass the destination policy.

ppl_http_max_concurrent=Board concurrent requests
ppl_http_max_concurrent-status=Maximum PPL HTTP operations across the whole board.
ppl_http_max_concurrent-help=
    # Board concurrent requests

    Requests beyond this number wait within their total timeout.

ppl_http_max_concurrent_node=Node concurrent requests
ppl_http_max_concurrent_node-status=Maximum PPL HTTP operations from one node.
ppl_http_max_concurrent_node-help=
    # Node concurrent requests

    Prevents one caller or PPE from consuming all of the board-wide capacity.

system_control_title=System Control

disable_ns_logon=Disable NS Logon Feature
disable_ns_logon-status=Allow to skip reading the WELCOME file.
disable_ns_logon-help=
    # Disable NS Logon Feature

    A caller can normally answer the graphics question with a ;Q to skip the
    welcome screen, and add ;NS to skip the news as well. Turn this on to take
    the non-stop shortcut away, so everybody sees the news at least once.

is_multi_lingual=Multi-Lingual Operation
is_multi_lingual-status=Enable Multi-Lingual Operation
is_multi_lingual-help=
    # Multi-Lingual Operation

    Offers the caller a choice of language at login, so prompts and display files
    are taken from the language they picked.

allow_alias_change=Allow Alias Change after Chosen
allow_alias_change-status=Preventing that avoids problems when leaving messages.
allow_alias_change-help=
    # Allow Alias Change after Chosen

    Whether a caller may change their alias once they have one. Leaving it off
    keeps somebody from appearing under a new name every few days, which is what
    makes an alias worth anything to the other callers.

is_closed_board=Run System as a Closed Board
is_closed_board-status=Disallow new users on the board.
is_closed_board-help=
    # Run System as a Closed Board

    Stops the board from taking new callers. A new name is no longer turned into
    a user record, so only the callers already in the user file get in.

enforce_daily_time_limit=Enforce Daily Time Limit
enforce_daily_time_limit-status=Switch between session and daily time limits.
enforce_daily_time_limit-help=
    # Enforce Daily Time Limit

    Counts the minutes a caller already used today against their allowance, so
    calling back does not hand out the session limit again. With it off, each
    call starts with a full session.
enforce_transfer_limits=Enforce Transfer Limits
enforce_transfer_limits-status=Apply PWRD byte, file and ratio limits to downloads
enforce_transfer_limits-help=Off by default so imported boards do not begin refusing downloads until the limits have been reviewed.

allow_password_failure_comment=Allow Password Failure Comment
allow_password_failure_comment-status=On PW failures message to sysop can be entered.
allow_password_failure_comment-help=
    # Allow Password Failure Comment

    Lets a caller who has run out of password attempts leave a comment for the
    sysop instead of only being disconnected, which is how somebody who lost
    their password asks for it back.

password_storage_method=Password Storage Method
password_storage_method-status=Turn on/off plaintext storage
password_storage_method-help=
    # Password Storage Method

    How passwords are kept in the user file. Plain text is what PCBoard did and
    is readable by anybody who can read the file; bcrypt and Argon2 store a hash
    instead, so a stolen user file does not hand over the passwords.
    Existing passwords are converted as each caller logs in.
password_storage_method_plain_text=Plain text
password_storage_method_bcrypt=BCrypt hash
password_storage_method_argon2=Argon 2 hash

guard_logoff=Warning on Logoff Command
guard_logoff-status=Show Warning on 'g' command.
guard_logoff-help=
    # Warning on Logoff Command

    Asks the caller to confirm the G command before hanging up, which saves the
    caller who typed G by accident. BYE logs off without the question.

confirm_caller_name=Confirm the Caller Name
confirm_caller_name-status=Show the matched record and let the caller correct a typo
confirm_caller_name-help=Catches the mistyped name that would otherwise create a second account.

reread_sec_level_on_join=Re-read Security Levels on Join
reread_sec_level_on_join-status=Apply the level limits again when a conference changes the level
reread_sec_level_on_join-help=Conferences may raise or lower the security level of the caller.


max_msg_lines=Maximum Lines in the Message Editor
max_msg_lines-status=Maximum number of lines the user is able to edit in the message editor.
max_msg_lines-help=
    # Maximum Lines in the Message Editor

    How many lines a caller may write in one message, between 17 and 400.
    The limit also applies to messages that arrive in an uploaded mail packet.

disable_message_scan_prompt=Disable Message Scan Prompt
disable_message_scan_prompt-status=Scans for messages on sign on or switch conference.
disable_message_scan_prompt-help=
    # Disable Message Scan Prompt

    Stops the board from asking whether to scan for messages at login and when
    joining a conference that has not been scanned yet.

allow_esc_codes=Allow ESC Codes in Messages
allow_esc_codes-status=Allow @X-color codes in messages.
allow_esc_codes-help=
    # Allow ESC Codes in Messages

    Lets a caller put raw escape sequences into a message. @X colour codes are
    shorter and are allowed either way, and a caller reading in plain text sees
    the escape codes as rubbish, so most boards leave this off.

allow_carbon_copy=Allow Carbon-Copy Messages
allow_carbon_copy-status=Allows the SC command to send messages to multiple users.
allow_carbon_copy-help=
    # Allow Carbon-Copy Messages

    Lets a caller save one message to several recipients with the SC command.
    It is worth telling callers to use it for private mail only, a public
    message is read by everyone anyway.

validate_to_name=Valiadate TO: Name in Messages
validate_to_name-status=Validate the TO: name in messages.
validate_to_name-help=
    # Validate TO: Name in Messages

    Checks the name at the TO: prompt against the user file and lets the caller
    correct a typo instead of writing a message nobody will ever receive.
    Mail for an echoed conference is not checked this way.

default_quick_personal_scan=Default to (Q)uick on Personal Mail Scan
default_quick_personal_scan-status=Default to (Q)uick on Personal Mail Scan
default_quick_personal_scan-help=
    # Default to (Q)uick on Personal Mail Scan

    Makes the short header listing the default when a caller scans for their own
    mail, instead of reading the messages straight away.

default_scan_all_selected_confs_at_login=Default to Scan ALL Conferences at Login
default_scan_all_selected_confs_at_login-status=On first login scan current or all conferences.
default_scan_all_selected_confs_at_login-help=
    # Default to Scan ALL Conferences at Login

    Makes the login scan cover every conference the caller selected rather than
    just the current one. It goes well with the quick scan default.

prompt_to_read_mail=Prompt to Read Mail when Mail Waiting
prompt_to_read_mail-status=Prompt to read a new message addressed to the user.
prompt_to_read_mail-help=
    # Prompt to Read Mail when Mail Waiting

    After telling the caller which conferences hold new mail for them, ask
    whether to read it now instead of leaving them to find it.

force_comments_to_main=Force Comments to the Main Board
force_comments_to_main-status=Comments to the sysop are always entered in the main board
force_comments_to_main-help=Keeps comments from scattering over the conferences.

update_last_read_pointer=Reading Moves the Last Read Pointer
update_last_read_pointer-status=Reading a message moves the last read pointer along
update_last_read_pointer-help=Decides what the next new message scan will show.

keyboard_timeout=Keyboard Timeout (in min)
keyboard_timeout-status=0=disable
keyboard_timeout-help=
    # Keyboard Timeout

    How many minutes of silence from the caller end the call, which frees the
    node when somebody walks away. Three to five minutes suits most boards,
    0 turns it off and lets a caller sit until their time runs out.

max_number_upload_descr_lines=Max Number of Upload Description Lines
max_number_upload_descr_lines-status=0=disable
max_number_upload_descr_lines-help=
    # Max Number of Upload Description Lines

    How many lines a caller may write to describe an upload.

password_expire_days=Number Days Before FORCES Password Change
password_expire_days-status=0=disable
password_expire_days-help=
    # Number Days Before FORCED Password Change

    After this many days a caller has to choose a new password before they can
    carry on. 0 lets a password stand forever.

password_expire_warn_days=Number Days to Warn Prior to FORCED Change
password_expire_warn_days-status=0=disable
password_expire_warn_days-help=
    # Number Days to Warn Prior to FORCED Change

    How many days ahead of the forced change the caller is warned, so the new
    password does not have to be invented on the spot. 0 turns the warning off.

min_pwd_length=Minimum Password Length
min_pwd_length-status=Shortest password a caller may choose
min_pwd_length-help=
    # Minimum Password Length

    The shortest password the board accepts when a caller sets one. It is checked
    when a caller registers or changes their password, not against passwords that
    are already in the user file.


disallow_batch_uploads=Disallow BATCH Uploads
disallow_batch_uploads-status=Disallow BATCH uploads should be avoided.
disallow_batch_uploads-help=
    # Disallow BATCH Uploads

    Refuses uploads of more than one file per turn. It mainly helps against
    callers whose file names do not survive the transfer, and it does discourage
    uploading, so most boards leave batch uploads on.

promote_to_batch_transfers=Promote to Batch Transfer
promote_to_batch_transfers-status=Auto promote to batch when batch protocol is selected.
promote_to_batch_transfers-help=
    # Promote to Batch Transfer

    Turns a plain U or D into the batch form when the caller has a batch protocol
    selected, so they can name several files at once instead of one per command.


upload_credit_time=Upload Credit for Time
upload_credit_time-status=Default is 1.0 which means 'stop the clock'.
upload_credit_time-help=
    # Upload Credit for Time

    How much of the time spent uploading is given back. 1.0 stops the clock, so
    the caller has as much time left as before the upload; 2.5 pays back two and
    a half minutes for every minute spent uploading.

upload_credit_bytes=Upload Credit for Bytes
upload_credit_bytes-status=Default is 1.0 which means 'stop the clock'.
upload_credit_bytes-help=
    # Upload Credit for Bytes

    How much download allowance an uploaded byte earns. 1.0 gives a byte back for
    every byte uploaded, higher values reward uploading more. The credit counts
    against the daily download limit, so it is gone at the end of the day.

display_uploader=Include 'Uploaded By' in Desc.
display_uploader-status=Include 'Uploaded By' in Description.
display_uploader-help=
    # Include 'Uploaded By' in Desc.

    Adds a line naming the uploader to the description of every uploaded file,
    which makes it easy to see later who brought a file in.

strip_colors_in_descriptions=Strip Colors in Descriptions
strip_colors_in_descriptions-status=Drop the colors a FILE_ID.DIZ brings with it.
strip_colors_in_descriptions-help=A FILE_ID.DIZ carries whatever colors its author chose, and a reset in one of them puts the caller back to the terminal default rather than the color this board lists files in. Turn this on to keep a listing in the board's own colors. Spacing and line art are left alone either way.

verify_files_uploaded=Verify Files Uploaded
verify_files_uploaded-status=Verify files uploaded after upload.
verify_files_uploaded-help=
    # Verify Files Uploaded

    Checks an upload before it is offered to anybody, which is where PCBoard ran
    a virus scanner or an archive test.

disable_drive_size_check=Disable Drive Size Check
disable_drive_size_check-status=That disables the message as well.
disable_drive_size_check-help=
    # Disable Drive Size Check

    Stops the board from looking at the free space before an upload, and with it
    the message about the space left. Only worth turning on where the free space
    cannot be measured sensibly.

stop_uploads_free_space=Stop Uploads when Free Space is less than
stop_uploads_free_space-status=Minimum kb left on drive to allow uploads.
stop_uploads_free_space-help=
    # Stop Uploads when Free Space is less than

    Uploads are refused once the upload drive has less than this many kilobytes
    free, which keeps a full disk from taking the board down with it. 0 turns the
    limit off.

disable_registration_edits=Disable Registration Edits
disable_registration_edits-status=Disable logon prompt filtering
disable_registration_edits-help=
    # Disable Registration Edits

    Turns off the filtering of what a caller types at the login prompts. The
    filtering guards against line noise, so only turn it off where callers need
    to enter characters the filter would throw away.

disable_high_ascii_filter=Disable High-ASCII Filter
disable_high_ascii_filter-status=Disable filtering of high ASCII chars.
disable_high_ascii_filter-help=
    # Disable High-ASCII Filter

    Lets characters above plain ASCII through, which callers writing in a
    language with accents need. The filter otherwise drops them as line noise.

default_graphics_at_login=Default to Graphics At Login
default_graphics_at_login-status=Set default answer to 'Do you want graphics?' question
default_graphics_at_login-help=
    # Default to Graphics At Login

    Makes yes the default answer to the graphics question, so a caller who just
    presses return gets colour.

non_graphics=Use Non-Graphics Mode Only
non_graphics-status=Disables all graphics
non_graphics-help=
    # Use Non-Graphics Mode Only

    Runs the board in plain text for everybody and stops asking about graphics.
    Colour and block characters are never sent, which suits a board reached from
    terminals that cannot show them.

exclude_local_calls_stats=Exclude Local Logins from Stats
exclude_local_calls_stats-status=Disables counting local logins from stats
exclude_local_calls_stats-help=
    # Exclude Local Logins from Stats

    Keeps local logins, and the transfers and messages made in them, out of the
    statistics, so the sysop's own testing does not flatter the numbers.

display_news_behavior=Display NEWS Only if Changed
display_news_behavior-status=Y (Always New), N (Once per Day), A (Always), X (Never)
display_news_behavior-help=
    # Display NEWS Only if Changed

    When the news is shown at login:

    - Y shows it when the file is newer than the caller's last call
    - N shows it once a day
    - A shows it on every call
    - X never shows it


display_userinfo_at_login=Display User Info at Login
display_userinfo_at_login-status=Display (V)iew Stats on login.
display_userinfo_at_login-help=
    # Display User Info at Login

    Shows the caller their own statistics right after login, the same thing the
    V command prints: last call, number of calls and transfer counts.

force_intro_on_join=Force INTRO Display on Join
force_intro_on_join-status=Force INTRO display on join.
force_intro_on_join-help=
    # Force INTRO Display on Join

    A caller can normally skip the intro of a conference by stacking a Q onto the
    join command. This forces the intro to be shown, which is what a conference
    with rules in its intro needs.

scan_new_blt=Scan for New Bulletins
scan_new_blt-status=Scan for new blt on login.
scan_new_blt-help=
    # Scan for New Bulletins

    Looks for bulletins the caller has not seen yet at login and says so.
    Turning it off makes login quicker on a board with many bulletins, at the
    price of callers missing new ones.

capture_grp_chat_session=Capture GROUP CHAT Session to Disk
capture_grp_chat_session-status=CaptLogs GROUP CHAT session
capture_grp_chat_session-help=
    # Capture GROUP CHAT Session to Disk

    Writes what is typed in group chat to a file, so the sysop can read a session
    afterwards.

allow_handle_in_grpchat=Allow Handles in GROUP CHAT
allow_handle_in_grpchat-status=Allow handles in GROUP CHAT
allow_handle_in_grpchat-help=
    # Allow Handles in GROUP CHAT

    Lets a caller pick the name they appear under in group chat instead of using
    their first name.

call_log=Write a Caller Log
call_log-status=Record every logon in the caller log
call_log-help=The file is named under File Locations - System Files.

log_caller_number=Log the Caller Number
log_caller_number-status=Write the caller number of the session to the caller log
log_caller_number-help=
    # Log the Caller Number

    Writes the running number of the call into the caller log, which is what ties
    a line in the log to one particular session.

log_connect_string=Log the Connection
log_connect_string-status=Write how the caller reached the board to the caller log
log_connect_string-help=
    # Log the Connection

    Writes down how the caller reached the board, so a complaint about a slow or
    broken session can be traced to the way they connected.

log_security_level=Log the Security Level
log_security_level-status=Write the security level of the caller to the caller log
log_security_level-help=
    # Log the Security Level

    Writes the caller's security level at login into the caller log. Remember it
    can change again when they join a conference.

# ICBSETUP -> Configuration Options > Colors
default_color=Default Color
default_color-status=Default color for text prompts.
default_color-help=
    # Default Color

    The colour the board returns to after a prompt or a display file has finished
    with a colour of its own. Everything that does not set its own colour is
    printed in this one.

msg_hdr_date=Header DATE Line
msg_hdr_date-status=Color for Message Header DATE Line
msg_hdr_date-help=
    # Header DATE Line

    Colour of the date line in the header shown above a message.

msg_hdr_to=Header TO Line
msg_hdr_to-status=Color for Message Header TO Line
msg_hdr_to-help=
    # Header TO Line

    Colour of the recipient line in the header shown above a message.


msg_hdr_from=Header FROM Line
msg_hdr_from-status=Color for Message Header FROM Line
msg_hdr_from-help=
    # Header FROM Line

    Colour of the sender line in the header shown above a message.

msg_hdr_subj=Header SUBJ Line
msg_hdr_subj-status=Color for Message Header SUBJ Line
msg_hdr_subj-help=
    # Header SUBJ Line

    Colour of the subject line in the header shown above a message.

msg_hdr_read=Header READ Line
msg_hdr_read-status=Color for Message Header READ Line
msg_hdr_read-help=
    # Header READ Line

    Colour of the line that says whether a message has been read.

msg_hdr_conf=Header CONF Line
msg_hdr_conf-status=Color for Message Header CONF Line
msg_hdr_conf-help=
    # Header CONF Line

    Colour of the conference line in the header shown above a message.


file_head=File HEAD Color
file_head-status=Color for File Header
file_head-help=
    # File HEAD Color

    Colour of the heading printed above a file listing.

file_name=File NAME Color
file_name-status=Color for File Name
file_name-help=
    # File NAME Color

    Colour of the file name column in a file listing.

file_size=File SIZE Color
file_size-status=Color for File Size
file_size-help=
    # File SIZE Color

    Colour of the size column in a file listing.

file_date=File DATE Color
file_date-status=Color for File Date
file_date-help=
    # File DATE Color

    Colour of the date column in a file listing.

file_description=File DESCR1 Color
file_description-status=Color for File first line of Description
file_description-help=
    # File DESCR1 Color

    Colour of the first line of a file description, the line that carries the
    short description in a listing.

file_duplicate=Duplicate File Color
file_duplicate-status=Color used when reporting duplicate files
file_duplicate-help=
    # Duplicate File Color

    Colour used for duplicate-file information in a directory listing.

file_text=File Text Color
file_text-status=Color for Text in Files
file_text-help=
    # File Text Color

    Colour of plain text printed between the entries of a file listing.

file_deleted=File Deleted Color
file_deleted-status=Color for 'Deleted' in Files
file_deleted-help=
    # File Deleted Color

    Colour of the word that marks an entry whose file is no longer on the board.

# ICBSETUP -> File Locations

file_locations_title=File Locations
file_locations_files_dirs=System Files & Directories
file_locations_config_files=Configuration Files
file_locations_display_files=Display Files
file_locations_surveys=New User/Logon/off Surveys

# ICBSETUP -> File Locations -> System Files & Directories

paths_conferences=Name/Loc of Conference Data
paths_conferences-status=Name/Loc of Conference Data
paths_conferences-help=
    # Name/Loc of Conference Data

    The file that lists the conferences of the board and everything configured
    for each of them. Without it the board has nothing but the main board.

paths_users_file=Name/Loc of User File
paths_users_file-status=Name/Loc of User File
paths_users_file-help=
    # Name/Loc of User File

    The file holding every user record: names, passwords, security levels,
    statistics and conference registrations. It is the one file a board must not
    lose, so it belongs in the backup.

paths_group_file=Name/Loc of Group File
paths_group_file-status=Name/Loc of Group File
paths_group_file-help=
    # Name/Loc of Group File

    The file that defines the groups callers can belong to. Groups are what
    security expressions test when access should not depend on a plain level.

paths_caller_log=Name/Loc of Caller Log
paths_caller_log-status=Name/Loc of Caller Log
paths_caller_log-help=
    # Name/Loc of Caller Log

    Where the board writes what happened on each call. It is the first place to
    look when a caller reports something odd.

paths_transfer_log=Name/Loc of Transfer Log
paths_transfer_log-status=Name/Location of the log every completed transfer is written to
paths_transfer_log-help=
    # Name/Loc of Transfer Log

    Where every completed upload and download is recorded, with the caller and
    the file. It answers who took what and when.

paths_statistic_file=Name/Loc of Statistics File
paths_statistic_file-status=Name/Loc of Statistics File
paths_statistic_file-help=
    # Name/Loc of Statistics File

    Where the running totals behind the call waiting screen are kept: calls,
    messages and transfers.

paths_icbtext=Location of ICBTEXT File
paths_icbtext-status=Name/Loc of ICBTEXT file
paths_icbtext-help=
    # Location of ICBTEXT File

    The file holding every prompt and message the board says to a caller.
    Edit it with mkicbtxt to reword the board or to translate it.

paths_tmp_files=Location of Temporary Work Files
paths_tmp_files-status=Location of Temporary Work Files
paths_tmp_files-help=
    # Location of Temporary Work Files

    The directory for files the board only needs while a caller is online, a
    mail packet being built for instance. It should be on a fast disk with room
    to spare.

paths_help_path=Location of Help Files
paths_help_path-status=Location of Help Files
paths_help_path-help=
    # Location of Help Files

    The directory holding the help files a caller reaches with the H command.
    Each file is named after the command it explains.

paths_security_file_path=Location of Login Security Files
paths_security_file_path-status=Location of Login Login Security Files
paths_security_file_path-help=
    # Location of Login Security Files

    The directory holding the screens shown when a caller is refused something
    on security grounds, so the board can explain rather than just say no.

paths_email_msg_base=Location of Email Message Base
paths_email_msg_base-status=Location of Email Message Base
paths_email_msg_base-help=
    # Location of Email Message Base

    The message base that carries private mail between callers, which is what
    the personal mail commands read and write.

paths_command_display_path=Location of Command Display Files
paths_command_display_path-status=Location of Command Display Files
paths_command_display_path-help=
    # Location of Command Display Files

    The directory holding the screens a command shows before it does its work.
    A file is found by the name of the command.

# ICBSETUP -> File Locations -> New User/Logon/off Surveys

paths_newask_survey=Name/Loc of New Reg Survey
paths_newask_survey-status=Name/Location of NEWASK Survey File
paths_newask_survey-help=
    # Name/Loc of New Reg Survey

    The questions a new caller is asked while registering, on top of the standard
    ones. This is where a board asks what it needs before handing out access.

paths_newask_answer=Name/Loc of New Reg Answers
paths_newask_answer-status=Name/Location of NEWASK Survey Answers
paths_newask_answer-help=
    # Name/Loc of New Reg Answers

    Where the answers to the registration questions are collected for the sysop
    to read.

paths_logon_survey=Name/Loc of Logon Survey
paths_logon_survey-status=Name/Location of Logon Survey File
paths_logon_survey-help=
    # Name/Loc of Logon Survey

    Questions put to a caller when they log in. Leave it empty unless the board
    really needs to ask something on every call.

paths_logon_answer=Name/Loc of Logon Answers
paths_logon_answer-status=Name/Location of Logon Survey Answers
paths_logon_answer-help=
    # Name/Loc of Logon Answers

    Where the answers to the logon questions are written.

paths_logoff_survey=Name/Loc of Logoff Survey
paths_logoff_survey-status=Name/Location of Logoff Survey File
paths_logoff_survey-help=
    # Name/Loc of Logoff Survey

    Questions put to a caller as they log off, which is where a board asks what
    somebody thought of the visit.

paths_logoff_answer=Name/Loc of Logoff Answers
paths_logoff_answer-status=Name/Location of Logoff Survey Answers
paths_logoff_answer-help=
    # Name/Loc of Logoff Answers

    Where the answers to the logoff questions are written.

# ICBSETUP -> File Locations -> Display Files

paths_welcome=Name/Loc of WELCOME File
paths_welcome-status=Name/Location of WELCOME File
paths_welcome-help=
    # Name/Loc of WELCOME File

    The first screen a caller sees after connecting, before they identify
    themselves. It is the board's front door.

paths_newuser=Name/Loc of NEWUSER File
paths_newuser-status=Name/Location of NEWUSER File
paths_newuser-help=
    # Name/Loc of NEWUSER File

    Shown to somebody the board does not know yet, before they are asked whether
    they want to register. This is where the rules and what registering gets
    them belong.

paths_closed=Name/Loc of CLOSED File
paths_closed-status=Name/Location of CLOSED File
paths_closed-help=
    # Name/Loc of CLOSED File

    Shown to a new caller while the board is closed, so they learn why they
    cannot register and how to ask for an account.

paths_expire_warning=Name/Loc of WARNING File
paths_expire_warning-status=Name/Location of WARNING File
paths_expire_warning-help=
    # Name/Loc of WARNING File

    Shown during the warning days before a subscription runs out, and the place
    to say how to renew.

paths_expired=Name/Loc of EXPIRED File
paths_expired-status=Name/Location of EXPIRED File
paths_expired-help=
    # Name/Loc of EXPIRED File

    Shown to a caller whose subscription has run out, once they have dropped to
    the expired security level.

paths_conf_join_menu=Name/Loc of Conference Join Menu
paths_conf_join_menu-status=Name/Location of Conference Join Menu File
paths_conf_join_menu-help=
    # Name/Loc of Conference Join Menu

    The list of conferences shown when a caller asks which ones there are.

paths_conf_chat_intro_file=Name/Loc of Group Chat Intro File
paths_conf_chat_intro_file-status=Name/Location of Group Chat Intro File
paths_conf_chat_intro_file-help=
    # Name/Loc of Group Chat Intro File

    Shown when a caller enters group chat, the place for the house rules and a
    reminder of how to leave again.

paths_conf_chat_menu=Name/Loc of Group Chat Menu
paths_conf_chat_menu-status=Name/Location of Group Chat Menu File
paths_conf_chat_menu-help=
    # Name/Loc of Group Chat Menu

    The list of commands available inside group chat.

paths_conf_chat_actions_menu=Name/Loc of Chat Actions Menu
paths_conf_chat_actions_menu-status=Name/Location of Chat Actions Menu File
paths_conf_chat_actions_menu-help=
    # Name/Loc of Chat Actions Menu

    The list of actions a caller can perform in group chat, the wave and grin
    kind of command.

paths_no_ansi=Name/Loc of NOANSI Warning
paths_no_ansi-status=Name/Location of NOANSI Warning File
paths_no_ansi-help=
    # Name/Loc of NOANSI Warning

    Shown to a caller whose terminal cannot do ANSI when they ask for something
    that needs it.

# ICBSETUP -> File Locations -> Configuration Files

paths_pwrd_sec_level_file=Name/Loc of PWRD/Security File
paths_pwrd_sec_level_file-status=Name/Location of PWRD/Security File
paths_pwrd_sec_level_file-help=
    # Name/Loc of PWRD/Security File

    Defines what each security level is worth: time per day, download allowance,
    ratios and the password for the level. It is where limits are set, rather
    than on each user record.

paths_trashcan_user=Name/Loc of User Trashcan File
paths_trashcan_user-status=Name/Location of User Trashcan File
paths_trashcan_user-help=
    # Name/Loc of User Trashcan File

    Names that may not be registered, one per line. It keeps callers from taking
    an offensive name, or one that pretends to be the sysop.

paths_trashcan_upload_files=Name/Loc of Upload File Trashcan
paths_trashcan_upload_files-status=Name/Location of Upload File Trashcan
paths_trashcan_upload_files-help=
    # Name/Loc of Upload File Trashcan

    File names that may not be uploaded, wildcards allowed. A caller offering a
    matching name is told the file is not wanted here.

paths_trashcan_passwords=Name/Loc of PWD Trashcan File
paths_trashcan_passwords-status=Name/Location of Password Trashcan File
paths_trashcan_passwords-help=
    # Name/Loc of PWD Trashcan File

    Passwords nobody may choose, one per line. This is where the obvious ones
    belong, so that a guessed password does not open an account.

paths_trashcan_email=Name/Loc of Email Trashcan File
paths_trashcan_email-status=Name/Location of Email Trashcan File
paths_trashcan_email-help=
    # Name/Loc of Email Trashcan File

    E-mail addresses the board refuses to accept from a caller.

paths_vip_users=Name/Loc of VIP Users File
paths_vip_users-status=Name/Location of VIP Users File
paths_vip_users-help=
    # Name/Loc of VIP Users File

    Names the sysop wants to be told about. When one of them logs in the board
    says so, so an important caller is not missed.

paths_protocol_data_file=Name/Loc of Protocol Data File
paths_protocol_data_file-status=Name/Location of Protocol Data File
paths_protocol_data_file-help=
    # Name/Loc of Protocol Data File

    The list of transfer protocols a caller can choose from. Press F2 on this
    line to edit it.

paths_language_file=Name/Loc of Multi-Lang. Data File
paths_language_file-status=Name/Location of Multi-Lang. Data File
paths_language_file-help=
    # Name/Loc of Multi-Lang. Data File

    The table of languages the board offers, each with the extension its display
    files and prompts carry.

paths_command_file=Name/Loc of CMD.LST File
paths_command_file-status=Name/Location of CMD.LST File
paths_command_file-help=
    # Name/Loc of CMD.LST File

    Commands of your own, added to the ones the board already knows. It is how a
    PPE or a door is given a name callers can type at the prompt.


connection_info_title=Connection Information
connection_info_telnet=Telnet
connection_info_ssh=SSH
connection_info_websockets=Websockets
connection_info_secure_websockets=Secure Websockets

connection_info_enabled=Enabled
connection_info_enabled-status=Whether callers may reach the board this way
connection_info_enabled-help=
    # Enabled

    Whether the board listens for this kind of connection at all. Turning off a
    service the board does not need is the cheapest way to close a door.

connection_info_port=Port
connection_info_port-status=TCP port callers connect to
connection_info_port-help=
    # Port

    The TCP port this service listens on. Telnet is normally 23 and SSH 22; a
    port below 1024 needs the board to be allowed to use it.

connection_info_address=Address
connection_info_address-status=Local address the service listens on
connection_info_address-help=
    # Address

    The local address the service binds to. Leave it at the address that covers
    every interface to accept calls from anywhere, or name a single address to
    keep the service on one network.

connection_info_display_file=Display File
connection_info_display_file-status=Screen shown to callers of this service
connection_info_display_file-help=
    # Display File

    A screen shown to callers who arrive through this service, before the usual
    login. It is the place for a notice that only concerns this way in.

# ICBSETUP -> Event Information

event_setup_title=Event Information

event_enabled_for_expedited_label=For EXPEDITED Events:

event_enabled=Is a Timed Event Active
event_enabled-status=Is a Timed Event Active
event_enabled-help=
    # Is a Timed Event Active

    Whether the board runs a timed event at all. With it off the settings below
    are kept but nothing is scheduled.

event_file=Name/Location of Event File
event_file-status=Name/Location of the timed event list
event_file-help=The TOML file that lists the timed events. Press F2 to edit it - the file is created when it does not exist yet.

event_suspend_minutes=Minutes Prior to Suspend All Activity
event_suspend_minutes-status=Minutes Prior to Suspend All Activity
event_suspend_minutes-help=
    # Minutes Prior to Suspend All Activity

    How long before the event the board stops letting callers do anything that
    would still be running when the event starts. A caller's time is cut short
    so that the event is not kept waiting.

event_disallow_uploads=Disallow Uploads Prior to Event
event_disallow_uploads-status=Disallow Uploads Prior to Event
event_disallow_uploads-help=
    # Disallow Uploads Prior to Event

    Turns uploads off shortly before the event, so no transfer is still running
    when the board has to go down.

event_minutes_uploads_disallowed=Minutes Prior to Disallow Uploads
event_minutes_uploads_disallowed-status=Minutes Prior to Disallow Uploads
event_minutes_uploads_disallowed-help=
    # Minutes Prior to Disallow Uploads

    How many minutes before the event uploads stop being accepted. It should be
    long enough for the largest upload the board usually sees.

# ICBSETUP -> Accounting Configuration
accounting_config_title=Accounting Configuration

accounting_enabled=Enable Accounting Features
accounting_enabled-status=Enable Accounting Features
accounting_enabled-help=
    # Enable Accounting Features

    Turns on the credit account each caller carries. Charges and rewards come
    from the rates file, and a security level only takes part when its entry in
    the PWRD file enables the account.

accounting_use_money=Display Money instead of Credits
accounting_use_money-status=Display Money instead of Credits
accounting_use_money-help=
    # Display Money instead of Credits

    Shows balances and charges with a currency symbol rather than as plain
    credits, which suits a board that really is charging money.

accounting_concurrent_tracking=Concurrent Tracking of Charges
accounting_concurrent_tracking-status=Concurrent Tracking of Charges
accounting_concurrent_tracking-help=
    # Concurrent Tracking of Charges

    Keeps the balance up to date while the caller is online instead of settling
    up at the end, so a caller cannot outspend their account within one call.

accounting_ignore_empty_sec_level=Ignore Empty Security Level
accounting_ignore_empty_sec_level-status=Ignore Empty Security Level
accounting_ignore_empty_sec_level-help=
    # Ignore Empty Security Level

    A caller whose account runs empty normally drops to the security level their
    record names for that case. This keeps them on their usual level instead.

accounting_peak_usage_start=Peak Usage Start Time
accounting_peak_usage_start-status=Peak Usage Start Time
accounting_peak_usage_start-help=
    # Peak Usage Start Time

    When peak hours begin, in 24-hour time. Minutes inside the peak range are
    charged at the peak rate instead of the normal one.

accounting_peak_usage_end=Peak Usage End Time
accounting_peak_usage_end-status=Peak Usage End Time
accounting_peak_usage_end-help=
    # Peak Usage End Time

    When peak hours end, in 24-hour time.

accounting_peak_days_of_week=Peak Days of the Week
accounting_peak_days_of_week-status=Peak Days of the Week
accounting_peak_days_of_week-help=
    # Peak Days of the Week

    The days on which peak rates apply at all. Selecting no day switches peak
    charging off entirely.

accounting_peak_holiday_list_file=Name/Loc of Peak Holidays List File
accounting_peak_holiday_list_file-status=Name/Loc of Peak Holidays List File
accounting_peak_holiday_list_file-help=
    # Name/Loc of Peak Holidays List File

    Dates on which peak charging is suspended, so a public holiday is billed at
    the cheaper rate even when it falls on a peak day.

accounting_cfg_file=Name/Loc of Account Configuration File
accounting_cfg_file-status=Name/Loc of Accounting Configuration File
accounting_cfg_file-help=
    # Name/Loc of Account Configuration File

    The file holding the charges and rewards for the things a caller does:
    time online, messages, uploads and downloads. Press F2 to edit the rates.

accounting_tracking_file=Name/Loc of Account Tracking File
accounting_tracking_file-status=Name/Loc of Accounting Tracking File
accounting_tracking_file-help=
    # Name/Loc of Account Tracking File

    Where each posting against a caller's account is recorded, so a balance can
    be explained afterwards.

accounting_info_file=Name/Loc of Accounting Info File
accounting_info_file-status=Name/Loc of Accounting Info File
accounting_info_file-help=
    # Name/Loc of Accounting Info File

    Shown at login to tell the caller where their account stands.

accounting_warning_file=Name/Loc of Accounting Warning File
accounting_warning_file-status=Name/Loc of Accounting Warning File
accounting_warning_file-help=
    # Name/Loc of Accounting Warning File

    Shown at login when the balance has fallen to the warning level, so a caller
    can top up before the account runs empty.

accounting_logoff_file=Name/Loc of Accounting Logoff File
accounting_logoff_file-status=Name/Loc of Accounting Logoff File
accounting_logoff_file-help=
    # Name/Loc of Accounting Logoff File

    Shown as the caller logs off, the place to report what the call cost and
    what is left.


# Conference Editor

conf_name=Name (#{ $number })

conf_public_conf=Public Conference
conf_public_conf-status=Public Conference
conf_public_conf-help=
    # Public Conference

    A public conference can be joined by anybody who meets the security below.
    A private one is open only to callers registered in it, whatever their level.

conf_req_sec_if_pub=Req. Security if Public
conf_req_sec_if_pub-status=Req. Security if Public
conf_req_sec_if_pub-help=
    # Req. Security if Public

    The security a caller needs to join while the conference is public. It can be
    a plain level or an expression, so a group can be admitted as well.

conf_pw_join_priv=Password to Join if Private
conf_pw_join_priv-status=Password to Join if Private
conf_pw_join_priv-help=
    # Password to Join if Private

    A password that opens the private conference to anyone who knows it, which
    saves registering each caller by hand.

conf_user_menu=Name/Loc of User's Menu
conf_user_menu-status=Name/Loc of User's Menu
conf_user_menu-help=
    # Name/Loc of User's Menu

    The menu shown to ordinary callers in this conference. Leave it empty to use
    the board's menu.

conf_sysop_menu=Name/Loc of Sysop's Menu
conf_sysop_menu-status=Name/Loc of Sysop's Menu
conf_sysop_menu-help=
    # Name/Loc of Sysop's Menu

    The menu shown here to a caller who passes the sysop level, normally the user
    menu plus the sysop commands.

conf_news_file=Name/Loc of NEWS File
conf_news_file-status=Name/Loc of NEWS File
conf_news_file-help=
    # Name/Loc of NEWS File

    News for this conference, shown by the news command and at join time
    according to the board's news setting.

conf_intro_file=Name/Loc of Conf INTRO File
conf_intro_file-status=Name/Loc of Conf INTRO File
conf_intro_file-help=
    # Name/Loc of Conf INTRO File

    Shown when a caller joins the conference, the place to say what it is for and
    what is expected here.

conf_attach_loc=Location for Attachments
conf_attach_loc-status=Location for Attachments
conf_attach_loc-help=
    # Location for Attachments

    The directory where files attached to messages in this conference are kept.

conf_cmd_lst_file=Conf. CMD.LST File
conf_cmd_lst_file-status=Conf. CMD.LST File
conf_cmd_lst_file-help=
    # Conf. CMD.LST File

    Extra commands that exist only in this conference, on top of the board wide
    command list.

conf_sort_loc_label={"              "}Sort  Name/Loc METADATA             Location of Uploads

conf_pub_upld=Public  Upld
conf_pub_upld-status=Public Upld
conf_pub_upld-help=
    # Public Upload

    Where uploads land once they are visible to everybody, and the directory the
    listing of new files is built from.

conf_priv_upld=Private Upld
conf_priv_upld-status=Private Upld
conf_priv_upld-help=
    # Private Upload

    Where uploads wait while only the sysop may see them. Viewing them needs the
    sysop level for private uploads.

conf_menu_path_label={"              "}Menu Listing                   Path/Name List File

conf_doors=Doors
conf_doors-status=Doors
conf_doors-help=
    # Doors

    The list of door programs offered in this conference. Press F2 to edit it.

conf_bulletins=Bulletins
conf_bulletins-status=Bulletins
conf_bulletins-help=
    # Bulletins

    The list of bulletins offered in this conference. Press F2 to edit it.

conf_surveys=Surveys
conf_surveys-status=Surveys
conf_surveys-help=
    # Surveys

    The list of surveys callers can answer here. Press F2 to edit it.

conf_directories=Directories
conf_directories-status=Directories
conf_directories-help=
    # Directories

    The file directories of this conference, which is what the F command lists.
    Press F2 to edit them.

conf_areas=Areas
conf_areas-status=Areas
conf_areas-help=
    # Areas

    The message areas of this conference. Unlike PCBoard, a conference can carry
    several message areas rather than a single base. Press F2 to edit them.

conf_auto_rejon=Auto-Rejoin into this Conf.
conf_auto_rejon-status=Auto-Rejoin into this Conf.
conf_auto_rejon-help=
    # Auto-Rejoin into this Conf.

    Puts a caller straight into this conference when they log in, instead of
    leaving them on the main board.

conf_add_conf_sec=Additional Conference Security
conf_add_conf_sec-status=Additional Conference Security
conf_add_conf_sec-help=
    # Additional Conference Security

    Added to the caller's security level while they are in this conference, so a
    trusted area can grant access it would not grant elsewhere.

conf_allow_view_conf_members=Allow Viewing Conf. Members
conf_allow_view_conf_members-status=Allow Viewing Conf. Members
conf_allow_view_conf_members-help=
    # Allow Viewing Conf. Members

    Whether a caller may list who else is registered here. Turn it off where the
    membership itself should stay private.

conf_add_conference_time=Additional Conference Time
conf_add_conference_time-status=Additional Conference Time
conf_add_conference_time-help=
    # Additional Conference Time

    Extra minutes granted while the caller is in this conference, which lets a
    support area stay open longer than the rest of the board.

conf_private_uploads=Make All Uploads Private
conf_private_uploads-status=Make All Uploads Private
conf_private_uploads-help=
    # Make All Uploads Private

    Keeps every upload in the private directory until the sysop moves it, so
    nothing reaches the other callers unseen.

conf_private_messages=Make All Messages Private
conf_private_messages-status=Make All Messages Private
conf_private_messages-help=
    # Make All Messages Private

    Every message written here is addressed privately, so only sender and
    recipient can read it.

conf_disallow_private_msgs=Disallow Private Messages
conf_disallow_private_msgs-status=Disallow Private Messages
conf_disallow_private_msgs-help=
    # Disallow Private Messages

    Forces every message here to be public, which is what an echoed conference
    that carries no private mail wants.

conf_sec_attachments=Level to Save File Attachment
conf_sec_attachments-status=Level to Save File Attachment
conf_sec_attachments-help=
    # Level to Save File Attachment

    The security a caller needs to attach a file to a message here.

conf_show_intro_in_scan=Show INTRO in 'R A' Scan
conf_show_intro_in_scan-status=Show INTRO in 'R A' Scan
conf_show_intro_in_scan-help=
    # Show INTRO in 'R A' Scan

    Prints the conference intro while a caller reads through all conferences, so
    they can tell where the messages are coming from.

conf_sec_write_message=Level to Enter a Message
conf_sec_write_message-status=Level to Enter a Message
conf_sec_write_message-help=
    # Level to Enter a Message

    The security needed to write a message here, on top of the board wide level
    for the E command.

conf_sec_carbon_copy=Level to Enter Carbon List Msgs
conf_sec_carbon_copy-status=Level to Enter Carbon List Msgs
conf_sec_carbon_copy-help=
    # Level to Enter Carbon List Msgs

    The security needed to send one message to a list of recipients here.

conf_carbon_list_limit=Carbon Copy List Limit
conf_carbon_list_limit-status=Carbon Copy List Limit
conf_carbon_list_limit-help=
    # Carbon Copy List Limit

    How many recipients one carbon copy message may name, which keeps a single
    message from being posted to the whole board at once.

conf_allow_aliases=Allow Aliases to be used
conf_allow_aliases-status=Allow Aliases to be used
conf_allow_aliases-help=
    # Allow Aliases to be used

    Whether callers appear under their alias here. Turn it off where messages
    should carry real names, a support conference for instance.

conf_charge_time=Charge Per Minute
conf_charge_time-status=Charge Per Minute
conf_charge_time-help=
    # Charge Per Minute

    Charged for every minute spent in this conference, on top of the board rate.
    It only does anything while accounting is switched on.

conf_charge_msg_read=Charge Per Message Read
conf_charge_msg_read-status=Charge Per Message Read
conf_charge_msg_read-help=
    # Charge Per Message Read

    Charged for each message read in this conference, which lets an expensive
    area cost more than the rest of the board.

conf_charge_msg_write=Charge Per Message Written
conf_charge_msg_write-status=Charge Per Message Written
conf_charge_msg_write-help=
    # Charge Per Message Written

    Charged for each message written in this conference. A negative value pays
    the caller and rewards contributions here.

conf_is_read_only=Make Conference Read-Only
conf_is_read_only-status=Make Conference Read-Only
conf_is_read_only-help=
    # Make Conference Read-Only

    Callers may read here but not write, which suits an announcement area fed
    from elsewhere.

conf_echo_mail_in_conference=Echo Mail in Conference
conf_echo_mail_in_conference-status=Echo Mail in Conference
conf_echo_mail_in_conference-help=
    # Echo Mail in Conference

    Marks the conference as carrying network mail, so messages written here are
    packed for the other systems instead of staying local.

conf_list_title=Conferences Menu

# Accounting Editor

accounting_title=Accounting Rates Configuration

accounting_start_balance=New User Starting Balance
accounting_start_balance-status=New User Starting Balance
accounting_start_balance-help=
    # New User Starting Balance

    The credit a new caller's account opens with, which is what lets somebody
    look around before they have paid anything.

accounting_warning_level=Balance Warning Level
accounting_warning_level-status=Balance Warning Level
accounting_warning_level-help=
    # Balance Warning Level

    The balance at which the caller starts being warned at login that the account
    is running low.

accounting_charges_label=Charges:

accounting_per_logon=Per Logon
accounting_per_logon-status=Per Logon
accounting_per_logon-help=
    # Per Logon

    Charged once for each successful login. A negative value pays the caller
    instead, which turns it into a reward for calling.

accounting_per_minute=Per Minute Online
accounting_per_minute-status=Per Minute Online
accounting_per_minute-help=
    # Per Minute Online

    Charged for every minute spent online outside the peak hours.

accounting_per_minute_peak=Per Minute Online Peak Time
accounting_per_minute_peak-status=Per Minute Online Peak Time
accounting_per_minute_peak-help=
    # Per Minute Online Peak Time

    Charged for every minute spent online inside the peak hours, on the days
    marked as peak days.

accounting_per_minute_grpChat=Per Minute in Group Chat (Added)
accounting_per_minute_grpChat-status=Per Minute in Group Chat (Added)
accounting_per_minute_grpChat-help=
    # Per Minute in Group Chat

    Charged for every minute in group chat. It is added on top of the ordinary
    per minute charge rather than replacing it.

accounting_per_message_read=Per Message Read
accounting_per_message_read-status=Per Message Read
accounting_per_message_read-help=
    # Per Message Read

    Charged for each message read online. Messages taken away in a mail packet
    are charged by the capture rate instead.

accounting_per_message_captured=Per Message Captured (QWK/c/d/z)
accounting_per_message_captured-status=Per Message Captured (QWK/c/d/z)
accounting_per_message_captured-help=
    # Per Message Captured

    Charged for each message a caller takes with them in a capture or a QWK
    packet rather than reading online.

accounting_per_message_written=Per Message Written
accounting_per_message_written-status=Per Message Written
accounting_per_message_written-help=
    # Per Message Written

    Charged for each public message written locally. A negative value rewards
    writing, which is one way to encourage traffic.

accounting_per_message_written_echoed=Per Message Written (Echoed)
accounting_per_message_written_echoed-status=Per Message Written (Echoed)
accounting_per_message_written_echoed-help=
    # Per Message Written (Echoed)

    Charged for a message that leaves the board for a network. It is kept apart
    from the local rate because echoed mail costs the board something.

accounting_per_message_written_private=Per Message Written (Private)
accounting_per_message_written_private-status=Per Message Written (Private)
accounting_per_message_written_private-help=
    # Per Message Written (Private)

    Charged for each private message written, which can differ from the rate for
    public messages that everybody gets to read.

accounting_per_file_downloaded=Per File Downloaded
accounting_per_file_downloaded-status=Per File Downloaded
accounting_per_file_downloaded-help=
    # Per File Downloaded

    Charged for each file downloaded, whatever its size.

accounting_per_file_bytes_downloaded=Per 1K-Bytes Downloaded
accounting_per_file_bytes_downloaded-status=Per 1K-Bytes Downloaded
accounting_per_file_bytes_downloaded-help=
    # Per 1K-Bytes Downloaded

    Charged for every kilobyte downloaded, so a large file costs more than a
    small one.

accounting_payback_label=Pay Back:

accounting_payback_per_file=Per File Uploaded
accounting_payback_per_file-status=Per File Uploaded
accounting_payback_per_file-help=
    # Per File Uploaded

    Paid back for each file uploaded, which is how uploading earns the credit
    that downloading spends.

accounting_payback_per_file_bytes=Per 1K-Bytes Uploaded
accounting_payback_per_file_bytes-status=Per 1K-Bytes Uploaded
accounting_payback_per_file_bytes-help=
    # Per 1K-Bytes Uploaded

    Paid back for every kilobyte uploaded, so a big contribution earns more.

# ICBord System Manager

icbsm_main_menu_title=Main Menu
icb_sysmanager_main_title=Users File Maintenance
icbsm_main_users=Users File Maintenance
icbsm_main_directory=Directory Maintenance
icbsm_dir_colors=Customize DIR File Colors
icb_sysmanager_main_edit_users=Edit Users File
icb_sysmanager_main_edit_groups=Edit Groups

icbsm_menu_edit_users=Edit Users File
icbsm_menu_sort=Sort Users File
icbsm_menu_pack=Pack Users File
icbsm_menu_adjust_security=Adjust Security Levels
icbsm_menu_insert_conf=Insert Group Conference
icbsm_menu_remove_conf=Remove Group Conference
icbsm_menu_move_conf=Move Users BETWEEN Conferences
icbsm_menu_expiration=Adjust Expiration Dates
icbsm_menu_phones=Standardize Phone Formats
icbsm_menu_undo=Undo (restore backup file)
icbsm_menu_groups=Edit Groups

icbsm_pack_title=Pack Users File
icbsm_adjust_security_title=Adjust Security Levels
icbsm_phones_title=Standardize Phone Formats
icbsm_undo_title=Undo (restore backup file)

icbsm_sec_by_ranges=Adjust by Ranges
icbsm_sec_by_ranges_title=Adjust Security by Range
icbsm_sec_by_ranges_expired_title=Adjust Expired Security by Range
icbsm_sec_by_ranges_expired=Adjust by Ranges (Expired)
icbsm_sec_by_file_ratio=Adjust by Up/Dn File Ratio
icbsm_sec_by_byte_ratio=Adjust by Up/Dn Byte Ratio
icbsm_sec_by_uploads=Adjust by Number of Uploads
icbsm_sec_by_downloads=Adjust by Number of Downloads
icbsm_sec_table_file_ratio=Create Up/Dn File Ratio Table
icbsm_sec_table_byte_ratio=Create Up/Dn Byte Ratio Table
icbsm_sec_table_uploads=Create Upload Table
icbsm_sec_table_downloads=Create Download Table
icbsm_sec_copy_expired=Change Security to Expired Level
icbsm_sec_init_counters=Initialize Upld/Dnld Counters

icbsm_table_title_file_ratio=Edit Upload/Download FILE Ratio Table
icbsm_table_title_byte_ratio=Edit Upload/Download BYTE Ratio Table
icbsm_table_title_uploads=Edit Upload Table
icbsm_table_title_downloads=Edit Download Table

icbsm_table_column_file_ratio=Ratio
icbsm_table_column_byte_ratio=Ratio
icbsm_table_column_uploads=Uploads
icbsm_table_column_downloads=Downloads
icbsm_table_security=Security

icbsm_table_help_title_file_ratio=Adjust Securities by Upload/Download Ratio
icbsm_table_help_title_byte_ratio=Adjust Securities by Upload/Download Ratio
icbsm_table_help_title_uploads=Adjust Securities by Number of Uploads
icbsm_table_help_title_downloads=Adjust Securities by Number of Downloads

icbsm_table_help_file_ratio =
    Define each of the upload/download ratios desired and attach a security level to each. A ratio is uploads divided by downloads.
    {" "}
    Examples:
    {" "}
      0.1  means   1 up for every 10 down
      1.0  means   uploads equal downloads
      5.0  means   5 up for every 1 down
    {" "}
    NOTE: A caller below the lowest step in the table keeps the security level they have.
icbsm_table_help_byte_ratio =
    Define each of the upload/download byte ratios desired and attach a security level to each. A ratio is bytes uploaded divided by bytes downloaded.
    {" "}
    Examples:
    {" "}
      0.1  means   1 byte up for every 10 down
      1.0  means   bytes up equal bytes down
      5.0  means   5 bytes up for every 1 down
    {" "}
    NOTE: A caller below the lowest step in the table keeps the security level they have.
icbsm_table_help_uploads =
    Define each of the upload counts desired and attach a security level to each. A caller who reaches a count gets that level, whether it raises or lowers the one they have.
    {" "}
    Example:      Uploads   Security
    {" "}
      assuming        0        10
      this table     10        25
      is used        20        30
                     30        35
    {" "}
    A caller with 10 to 19 uploads and a level of 20 would be raised to 25. Below the lowest step the level stays as it is.
icbsm_table_help_downloads =
    Define each of the download counts desired and attach a security level to each. A caller who reaches a count gets that level, whether it raises or lowers the one they have.
    {" "}
    Example:    Downloads   Security
    {" "}
      assuming        0        35
      this table     10        25
      is used        20        20
                     30        15
    {" "}
    A caller with 20 to 29 downloads and a level of 15 would be raised to 20. Below the lowest step the level stays as it is.

icbsm_table_empty=This table has no steps yet. Build it first.
icbsm_table_hint=A step with security 0 is left out of the table
icbsm_table_keys=ESC=Exit   PGDN=Save the table   Arrows=Move
icbsm_table_saved=The table was saved.

icbsm_counters_title=Initialize Upload/Download Counters
icbsm_counters_option1=1) Make fields EQUAL (based on download field)
icbsm_counters_option2=2) Make fields EQUAL (based on upload field)
icbsm_counters_option3=3) Initialize both upload & download fields to ZERO
icbsm_counters_option4=4) Initialize both BYTE counters (based on Up:Down FILE ratio)
icbsm_counters_choose=Choose Option (1, 2, 3 or 4 from above)
icbsm_counters_files=Adjust Upload / Download FILE Counters
icbsm_counters_bytes=Adjust Upload / Download BYTE Counters

icbsm_apply_table_question=Adjust securities from the { $count } step(s) in the table?
icbsm_question_keys=PGDN=Yes   ESC=Abort
icbsm_are_you_sure=Are you sure?

icbsm_sort_options_title=Sort Options
icbsm_sort_single_title=Single Field Sorts
icbsm_sort_multiple_title=Multiple Field Sorts
icbsm_sort_run_title=Sort Users File

icbsm_sort_name=Name
icbsm_sort_password=Password
icbsm_sort_bus_phone=Business / Data Phone
icbsm_sort_home_phone=Home / Voice Phone
icbsm_sort_registration=Registration Expiration
icbsm_sort_comment1=Comment Number 1
icbsm_sort_comment2=Comment Number 2
icbsm_sort_city=User City

icbsm_sort_security_name=Security Level then Name
icbsm_sort_times_on_name=Num Times On then Name
icbsm_sort_dnld_name=Num Files Downloaded then Name
icbsm_sort_upld_name=Num Files Uploaded then Name
icbsm_sort_file_ratio_name=Files Upld:Dnld Ratio then Name
icbsm_sort_dnld_bytes_name=Num Bytes Downloaded then Name
icbsm_sort_upld_bytes_name=Num Bytes Uploaded then Name
icbsm_sort_byte_ratio_name=Bytes Upld:Dnld Ratio then Name

icbsm_sort_field=Sort the user file by { $field }
icbsm_sort_reverse=Sort in Reverse Order : { $value }
icbsm_sort_done={ $count } record(s) moved.
icbsm_yes=Yes
icbsm_no=No
icbsm_sort_keys=R reverse order, PGDN to begin, ESC to abort
icbsm_menu_keys=Use arrow keys to move bar, press ENTER to select, ESC to exit

icbsm_min_security=Change users whose security is greater than or equal to
icbsm_max_security=and whose security level is less than or equal to
icbsm_use_expired_level=Base Security Level Criteria on the EXPIRED Level

icbsm_pack_removal_group=Criteria for User Record Removal
icbsm_pack_keep_group=Criteria for Keeping User Record

icbsm_remove_deleted_or_locked=Remove Users that are Deleted or `LOCKED OUT'
icbsm_inactive_days=Remove Users who have not been on for XXXX days
icbsm_inactive_days-status=9999 leaves the last call out of it
icbsm_last_on_since=Remove Users who have not been on since
icbsm_expired_before=Remove Users whose Reg. Exp. Date is older than
icbsm_date_off-status=01-01-80 switches this criterion off
icbsm_keep_security=Keep Users with security greater than or equal to
icbsm_keep_security-status=0 keeps nobody for their security level
icbsm_keep_locked_out=Keep Users that are `LOCKED OUT'

icbsm_new_level=To a new security level of
icbsm_write_expired_level=Change the Expired Security Level instead
icbsm_copy_expired_level=Change Security to Expired Level
icbsm_copy_expired_level-status=Takes the new level from each record instead

icbsm_expiration_title=Change Expiration Date
icbsm_expiration_range_group=Security Level Range
icbsm_expiration_change_group=Change Expiration Date To:
icbsm_exp_min_security=Adjust Expiration Date if level is Greater than or equal to
icbsm_exp_max_security=Adjust Expiration Date if level is Less than or equal to
icbsm_expiration_date=New Expiration Date (01-01-80 is ignored)
icbsm_add_days=Current Date in record plus XXXX days

icbsm_conf_insert_title=Insert Group Conference Registrations
icbsm_conf_remove_title=Remove Group Conference Registrations
icbsm_conf_move_title=Move User(s) Between Conferences

icbsm_conf_first_insert=First number of conferences to be inserted in registrations
icbsm_conf_last_insert=Last  number of conferences to be inserted in registrations
icbsm_conf_first_remove=First number of conferences to be removed from registrations
icbsm_conf_last_remove=Last  number of conferences to be removed from registrations
icbsm_conf_min_security=Adjust users with a security level greater than or equal to
icbsm_conf_max_security=and less than or equal to

icbsm_conf_from=REMOVE user(s) from which conference
icbsm_conf_to=ADD to which conference
icbsm_move_min_security=Include users with security levels GREATER THAN or EQUAL TO
icbsm_move_max_security=Include users with security levels  LESS THAN   or EQUAL TO

icbsm_flag_registered=Adjust conferences user is normally allowed to join
icbsm_flag_expired=Adjust conferences user can join with expired subscription
icbsm_flag_selected=Adjust the user selected conferences for scanning
icbsm_flag_sysop=Adjust conferences where user becomes a sysop when joining
icbsm_reset_lastread=Reset user's last message read to zero in these conferences
icbsm_flag_net_status=Adjust conferences where user has Net Status

icbsm_move_flag_registered=Adjust conferences user is allowed in (at all times)
icbsm_move_flag_expired=Adjust conferences user is allowed in (expired subscription)
icbsm_move_flag_selected=Adjust conferences user-scan preference list
icbsm_move_flag_sysop=Adjust conferences where user becomes a sysop upon joining
icbsm_move_lastread=Move the 'Last Message Read' pointer to the new conference
icbsm_move_last_conference=Set the 'Last Conference In' flag

icbsm_criteria_keys=Press PGDN to begin, or press ESC to abort
icbsm_preview_keys=ENTER Run, ESC Back
icbsm_done_keys=Press A Key
icbsm_undo_keys=ENTER Restore, ESC Cancel

icbsm_preview_count={ $count } user(s) selected
icbsm_preview_more=... and { $count } more
icbsm_preview_pack_warning=These records will be removed. A backup is written first.
icbsm_done_count={ $changed } of { $matched } user(s) changed
icbsm_done_backup_hint=The previous user file was kept, undo it from the main menu.
icbsm_backup_failed=Could not write the backup, nothing was changed: { $error }
icbsm_save_failed=Could not save the user file: { $error }

icbsm_undo_prompt=Restore Users File from backup taken { $date }?
icbsm_undo_no_backup=There is no backup to restore.
icbsm_undo_done=The user file was restored.
icbsm_undo_failed=Could not restore the user file: { $error }

icbsm_board_in_use=Another tool is working on this board. Close it and start again.
icbsm_record_one_protected=Record #1 is the sysop record and cannot be removed.

icbsm_list_sort_record=Record
icbsm_list_sort_name=Name
icbsm_list_sort_security=Security
icbsm_list_sort_last_on=Last On

icbsm_user_list_keys=F2 Save, F3 Find, F4 Sort ({ $sort }), INS Add, DEL Remove
icbsm_user_list_search=Find: { $search }_ (ENTER Keep, ESC Clear)
icbsm_user_list_filtered=Find "{ $search }": { $count } shown, sorted by { $sort } - F3 Find, F4 Sort

user_editor_name=Name
user_editor_name-status=Name
user_editor_name-help=
    # Name

    The name the caller logs in with. Changing it changes how they log in, so it
    has to stay unique in the user file.

user_editor_alias=Alias
user_editor_alias-status=Alias
user_editor_alias-help=
    # Alias

    The handle the caller appears under in conferences that allow aliases.

user_editor_password=Password
user_editor_password-status=Password
user_editor_password-help=
    # Password

    The caller's login password. It is stored the way the board's password
    storage method says, so with hashing turned on it cannot be read back here.

user_editor_security=Security
user_editor_security-status=Security
user_editor_security-help=
    # Security

    The caller's security level, which decides which commands, conferences and
    files are open to them. 0 locks the account.

user_editor_city=City
user_editor_city-status=City
user_editor_city-help=
    # City

    Where the caller says they are calling from. It appears in the WHO listing
    when the board is set to show the city.

user_editor_bus_phone=B/D Phone
user_editor_bus_phone-status=B/D Phone
user_editor_bus_phone-help=
    # B/D Phone

    The caller's business or data phone number.

user_editor_home_phone=H/V Phone
user_editor_home_phone-status=H/V Phone
user_editor_home_phone-help=
    # H/V Phone

    The caller's home or voice phone number.

user_editor_verify_answer=Verify Answer
user_editor_verify_answer-status=Verify Answer
user_editor_verify_answer-help=
    # Verify Answer

    The answer the caller gave to the verification question. Ask for it when
    somebody wants their account back to confirm who they are.

user_editor_protocol=Protocol
user_editor_protocol-status=Protocol
user_editor_protocol-help=
    # Protocol

    The transfer protocol used for this caller's uploads and downloads.

user_editor_page_len=Page Len
user_editor_page_len-status=Page Len
user_editor_page_len-help=
    # Page Len

    How many lines are printed before the board pauses for this caller.
    0 turns the pause off and lets everything scroll past.

user_editor_reg_ex_date=Reg Ex Date
user_editor_reg_ex_date-status=Reg Ex Date
user_editor_reg_ex_date-help=
    # Reg Ex Date

    The day this caller's subscription runs out. It is only looked at while
    subscription mode is on; an empty date never expires.

user_editor_exp_sec=Expired Sec
user_editor_exp_sec-status=Expired Sec
user_editor_exp_sec-help=
    # Expired Sec

    The security level this caller drops to once the expiration date has passed.

user_editor_msg_clear=Msg clear
user_editor_msg_clear-status=Msg clear
user_editor_msg_clear-help=
    # Msg clear

    Whether the screen is cleared between messages for this caller.

user_editor_scroll_msg=Scroll msg
user_editor_scroll_msg-status=Scroll msg
user_editor_scroll_msg-help=
    # Scroll msg

    Whether message bodies scroll past instead of stopping page by page.

user_editor_fse_mode=Full Scrn editor
user_editor_fse_mode-status=Full Scrn editor
user_editor_fse_mode-help=
    # Full Scrn editor

    Whether this caller writes messages in the full screen editor rather than
    line by line.

user_editor_use_short_filedescr=Short Desc
user_editor_use_short_filedescr-status=Short Desc
user_editor_use_short_filedescr-help=
    # Short Desc

    Whether file listings show only the first line of each description for this
    caller.

user_editor_wide_editor=79-Column Editor
user_editor_wide_editor-status=79-Column Editor
user_editor_wide_editor-help=
    # 79-Column Editor

    Lets this caller write across the full 79 columns instead of the narrower
    default width.

user_editor_last_conference=Last in
user_editor_last_conference-status=Last Conference the user was in
user_editor_last_conference-help=
    # Last Conference

    The conference the caller was in when they last logged off, which is where
    an automatic rejoin puts them again.

user_editor_long_msg_header=Long Headers
user_editor_long_msg_header-status=Long Headers
user_editor_long_msg_header-help=
    # Long Headers

    Whether the caller sees the full message header with every line, or the
    shortened form.

user_editor_delete_user=Delete User
user_editor_delete_user-status=Delete User
user_editor_delete_user-help=
    # Delete User

    Marks the record for deletion. The account stays until the user file is
    packed, so the mark can still be taken back before then.


user_editor_chat_status=Chat Status
user_editor_chat_status-status=Chat Status
user_editor_chat_status-help=
    # Chat Status

    Whether this caller is available for chat with other nodes or wants to be
    left alone.

user_editor_expert_mode=Expert
user_editor_expert_mode-status=Expert
user_editor_expert_mode-help=
    # Expert

    Whether the caller gets the short prompts instead of the full menus.

user_editor_comment1=Comment1
user_editor_comment1-status=User Comment
user_editor_comment1-help=
    # Comment1

    The line the caller wrote about themselves when registering.

user_editor_comment2=Comment2
user_editor_comment2-status=Sysop Comment
user_editor_comment2-help=
    # Comment2

    A note only the sysop sees, the place to record why an account was upgraded
    or is being watched.

user_editor_adr1=Address #1
user_editor_adr1-status=Address #1
user_editor_adr1-help=
    # Address #1

    First line of the caller's postal address.

user_editor_adr2=Address #2
user_editor_adr2-status=Address #2
user_editor_adr2-help=
    # Address #2

    Second line of the caller's postal address.

user_editor_state=State
user_editor_state-status=State
user_editor_state-help=
    # State

    State or region of the caller's postal address.

user_editor_zip=Zip Code
user_editor_zip-status=Zip Code
user_editor_zip-help=
    # Zip Code

    Postal code of the caller's address.


user_editor_country=Country
user_editor_country-status=Country
user_editor_country-help=
    # Country

    Country of the caller's address.

user_editor_cmt_line1=Line 1
user_editor_cmt_line1-status=Custom Comment Line 1
user_editor_cmt_line1-help=
    # Custom Comment Line 1

    A free line of your own about this caller, shown wherever the board prints
    the custom comment lines.

user_editor_cmt_line2=Line 2
user_editor_cmt_line2-status=Custom Comment Line 2
user_editor_cmt_line2-help=
    # Custom Comment Line 2

    A further free line of your own about this caller.

user_editor_cmt_line3=Line 3
user_editor_cmt_line3-status=Custom Comment Line 3
user_editor_cmt_line3-help=
    # Custom Comment Line 3

    A further free line of your own about this caller.

user_editor_cmt_line4=Line 4
user_editor_cmt_line4-status=Custom Comment Line 4
user_editor_cmt_line4-help=
    # Custom Comment Line 4

    A further free line of your own about this caller.

user_editor_cmt_line5=Line 5
user_editor_cmt_line5-status=Custom Comment Line 5
user_editor_cmt_line5-help=
    # Custom Comment Line 5

    The last free line of your own about this caller.

user_editor_gender=Gender
user_editor_gender-status=Note: Could be stored in a bit
user_editor_gender-help=
    # Gender

    The gender the caller gave, if the board asks for it.

user_editor_birthdate=Birthdate
user_editor_birthdate-status=Birthdate
user_editor_birthdate-help=
    # Birthdate

    The caller's date of birth, which is what an age check in a PPE reads.

user_editor_email=Email Address
user_editor_email-status=Email
user_editor_email-help=
    # Email Address

    The caller's e-mail address.

user_editor_web=Web Address
user_editor_web-status=Web
user_editor_web-help=
    # Web Address

    The address of the caller's home page.

# ICBSETUP -> Conferences > Edit DIRS.TOML

dirs_editor_title=DIR.LST Editor { $conference }
dirs_table_name_header=Name
dirs_table_path_header=Path
dirs_edit_directory_title=Edit Directory

dirs_edit_name=Name
dirs_edit_name-status=Name
dirs_edit_name-help=
    # Name

    The name of the file directory as callers see it in the file menu.

dirs_edit_path=Path
dirs_edit_path-status=Path
dirs_edit_path-help=
    # Path

    The directory on disk holding the files that are offered here.

dirs_metadata_path=Metadata Path
dirs_metadata_path-status=Stores additional info about the files
dirs_metadata_path-help=
    # Metadata Path

    Where the board keeps what it knows about the files beyond the files
    themselves: descriptions, uploader and download counts.

dirs_edit_password=Password
dirs_edit_password-status=Password
dirs_edit_password-help=
    # Password

    A password callers must give before this directory opens, which protects it
    without giving anybody a higher security level.

dirs_edit_sort=Sort
dirs_edit_sort-status=Sort
dirs_edit_sort-help=
    # Sort

    What the listing is ordered by, the file name or its date.

dirs_edit_sort_asc=Sort ascending
dirs_edit_sort_asc-status=Sort ascending
dirs_edit_sort_asc-help=
    # Sort ascending

    Whether the order runs upwards or downwards. Sorting by date downwards puts
    the newest files at the top.

dirs_edit_has_new_files=Has New Files
dirs_edit_has_new_files-status=Has New Files
dirs_edit_has_new_files-help=
    # Has New Files

    Whether a scan for new files looks in this directory. Turn it off for a
    directory whose contents never change.

dirs_edit_is_free=Is Free
dirs_edit_is_free-status=Is Free
dirs_edit_is_free-help=
    # Is Free

    Downloads from here do not count against the caller's byte limit or ratio,
    which suits your own utilities and documentation.

dirs_edit_list_sec=List Security
dirs_edit_list_sec-status=List Security
dirs_edit_list_sec-help=
    # List Security

    The security needed to see this directory at all. A caller without it is not
    shown that the directory exists.

dirs_download_sec=Download Security
dirs_download_sec-status=Download Security
dirs_download_sec-help=
    # Download Security

    The security needed to take a file out of this directory, which can be higher
    than the security needed to look at the listing.

area_editor_title=AREA.LST Editor - { $conference }
area_editor_edit_title=Edit Area

area_editor_name=Name
area_editor_name-status=Name
area_editor_name-help=
    # Name

    The name of the message area as callers see it.

area_editor_qwk_name=QWK Name
area_editor_qwk_name-status=QWK Name (BLANK=Use Name)
area_editor_qwk_name-help=
    Area name as it appears in QWK packets.

area_editor_file=File
area_editor_file-status=File
area_editor_file-help=
    # File

    The message base on disk that holds the messages of this area.

area_editor_is_readonly=Is Read-Only
area_editor_is_readonly-status=Is Read-Only
area_editor_is_readonly-help=
    # Is Read-Only

    Callers may read this area but not write to it.

area_editor_allow_aliases=Allow Aliases
area_editor_allow_aliases-status=Allow Aliases
area_editor_allow_aliases-help=
    # Allow Aliases

    Whether messages here may be written under an alias instead of the caller's
    real name.

area_editor_list_sec=List Security
area_editor_list_sec-status=List Security
area_editor_list_sec-help=
    # List Security

    The security needed to see and read this area.

area_editor_enter_sec=Enter Security
area_editor_enter_sec-status=Enter Security
area_editor_enter_sec-help=
    # Enter Security

    The security needed to write a message in this area.

area_editor_attach_sec=Attach Security
area_editor_attach_sec-status=Attach Security
area_editor_attach_sec-help=
    # Attach Security

    The security needed to hang a file on a message here.

area_editor_qwk_number=QWK Number
area_editor_qwk_number-status=QWK Number (=0 automatic)
area_editor_qwk_number-help=
    Area number as it appears in QWK packets. This allows to use fixed numbers for QWK packets.
    This is useful for beeing able to add/remove areas without changing the QWK numbers.

doors_editor_title=DOORS File Editor { $conference }
doors_editor_edit_title=Edit Door
doors_editor_key_help=↑ Up  ↓ Down  Tab Edit Doors ␛ Back
doors_editor_key_help_door=↑ Up  ↓ Down  INS New  ␡ Delete  Tab Edit BBSLINK ␛ Back

doors_editor_header_door=Door
doors_editor_header_description=Description
doors_editor_header_type=Type

door_editor_name=Name
door_editor_name-status=Name
door_editor_name-help=
    # Name

    The name callers type to start this door.

door_editor_description=Description
door_editor_description-status=Description
door_editor_description-help=
    # Description

    The line describing the door in the door listing.

door_editor_password=Password
door_editor_password-status=Password
door_editor_password-help=
    # Password

    A password callers must give before this door starts.

door_editor_path=Path
door_editor_path-status=Path
door_editor_path-help=
    # Path

    The program that is run for this door.

door_editor_door_type=Door Type
door_editor_door_type-status=Door Type
door_editor_door_type-help=
    # Door Type

    What kind of program this is, which decides the drop file the board writes
    and how the door is handed the caller's session.

door_editor_use_shell_execute=Use Shell Execute
door_editor_use_shell_execute-status=Use Shell Execute
door_editor_use_shell_execute-help=
    # Use Shell Execute

    Runs the door through the system shell, so a command line with arguments or
    redirection is interpreted as it would be when typed.

lang_editor_title=Language Table

lang_editor_header_language=Language
lang_editor_header_ext=Extension
lang_editor_header_locale=Locale
lang_editor_header_yes=Yes
lang_editor_header_no=No
lang_editor_edit_lang=Edit Language

lang_editor_edit_lang_label=Language
lang_editor_edit_lang_label-status=Language
lang_editor_edit_lang_label-help=
    # Language

    The name of the language as the caller is offered it at login.

lang_editor_edit_extension=Extension
lang_editor_edit_extension-status=Extension
lang_editor_edit_extension-help=
    # Extension

    The extension the display files and prompt files of this language carry, so
    the board can find the right version of a screen.

lang_editor_edit_locale=Locale
lang_editor_edit_locale-status=Locale
lang_editor_edit_locale-help=
    # Locale

    The locale for this language, which decides how dates and numbers are
    written for the caller.

lang_editor_edit_yes_char=Yes Char
lang_editor_edit_yes_char-status=Yes Char
lang_editor_edit_yes_char-help=
    # Yes Char

    The key that means yes in this language, so a caller can answer in their own
    language rather than with Y.

lang_editor_edit_no_char=No Char
lang_editor_edit_no_char-status=No Char
lang_editor_edit_no_char-help=
    # No Char

    The key that means no in this language.

surveys_editor_title=Surveys { $conference }
survey_editor_editor=Edit Survey

survey_editor_editor_header_question=Question
survey_editor_editor_header_answer=Answer

survey_editor_editor_file=Survey File
survey_editor_editor_file-status=Survey File
survey_editor_editor_file-help=
    # Survey File

    The file holding the questions of this survey, one per line.

survey_editor_editor_answer_file=Answer File
survey_editor_editor_answer_file-status=Answer File
survey_editor_editor_answer_file-help=
    # Answer File

    Where the answers are collected. Every caller's answers are appended, so the
    file grows as the survey is taken.

survey_editor_editor_security=Security
survey_editor_editor_security-status=Security
survey_editor_editor_security-help=
    # Security

    The security a caller needs to be offered this survey.

sec_level_editor_title=Edit Security Levels
sec_level_editor_editor=Edit Security Level

sec_level_header_security=Security
sec_level_header_description=Description
sec_level_header_time=Time

sec_level_editor_security=Security
sec_level_editor_security-status=Security
sec_level_editor_security-help=
    # Security

    The security level this entry describes. Everything below applies to callers
    on this level.

sec_level_editor_description=Description
sec_level_editor_description-status=Description
sec_level_editor_description-help=
    # Description

    A note saying what this level is for. It is only for the sysop's benefit.

sec_level_editor_password=Password
sec_level_editor_password-status=Password
sec_level_editor_password-help=
    # Password

    A password that raises a caller to this level when they give it, which is how
    somebody is upgraded without the sysop editing their record.

sec_level_editor_time_per_day=Time
sec_level_editor_time_per_day-status=Time
sec_level_editor_time_per_day-help=
    # Time

    How many minutes a caller on this level may spend online per day.

sec_level_editor_daily_bytes=Daily KBytes
sec_level_editor_daily_bytes-status=Daily KBytes
sec_level_editor_daily_bytes-help=
    # Daily KBytes

    How many kilobytes a caller on this level may download per day.

sec_level_editor_file_ratio=File Ratio
sec_level_editor_file_ratio-status=File Ratio
sec_level_editor_file_ratio-help=
    # File Ratio

    How many files may be downloaded for each file uploaded. It only bites once
    the free file limit below is used up.

sec_level_editor_byte_ratio=Byte Ratio
sec_level_editor_byte_ratio-status=Byte Ratio
sec_level_editor_byte_ratio-help=
    # Byte Ratio

    How many bytes may be downloaded for each byte uploaded, measured the same
    way as the file ratio.

sec_level_editor_file_limit=File Limit
sec_level_editor_file_limit-status=File Limit
sec_level_editor_file_limit-help=
    # File Limit

    How many files a caller may download before the ratio starts to apply, which
    gives a new caller something to fetch before they have uploaded anything.

sec_level_editor_kb_limit=KByte Limit
sec_level_editor_kb_limit-status=KByte Limit
sec_level_editor_kb_limit-help=
    # KByte Limit

    How many kilobytes may be downloaded before the byte ratio starts to apply.


sec_level_editor_file_credit=File Credit
sec_level_editor_file_credit-status=File Credit
sec_level_editor_file_credit-help=
    # File Credit

    Files granted on top of the ratio, a starting credit for callers on this
    level.


sec_level_editor_kb_credit=KByte Credit
sec_level_editor_kb_credit-status=KByte Credit
sec_level_editor_kb_credit-help=
    # KByte Credit

    Kilobytes granted on top of the byte ratio.

sec_level_editor_enforce_time=Enforce Time Limit
sec_level_editor_enforce_time-status=Enforce Time Limit
sec_level_editor_enforce_time-help=
    # Enforce Time Limit

    Whether the daily time limit is applied to this level at all. Together with
    the board setting of the same name it decides if time is counted per day.


sec_level_editor_allow_alias=Allow Alias
sec_level_editor_allow_alias-status=Allow Alias
sec_level_editor_allow_alias-help=
    # Allow Alias

    Whether callers on this level may appear under an alias.

sec_level_force_read_mail=Force Read Mail
sec_level_force_read_mail-status=Force Read Mail
sec_level_force_read_mail-help=
    # Force Read Mail

    Makes callers on this level read the mail waiting for them before they can
    do anything else, which is how an important notice is made unavoidable.

sec_level_demo_acc=Demo Account
sec_level_demo_acc-status=Demo Account
sec_level_demo_acc-help=
    # Demo Account

    Marks this level as a look-around account, so a visitor can see the board
    without it counting as a real registration.

sec_level_enable_acc=Enable Account
sec_level_enable_acc-status=Enable Account
sec_level_enable_acc-help=
    # Enable Account

    Whether callers on this level take part in accounting. Without it their
    balance is neither charged nor checked.

protocol_editor_title=Transfer Protocols
protocol_editor_editor=Edit Protocol

protocol_editor_header_char_code=Use
protocol_editor_header_description=Description

protocol_editor_is_enabled=Enabled
protocol_editor_is_enabled-status=Enabled
protocol_editor_is_enabled-help=
    # Enabled

    Whether this protocol is offered to callers. Turning one off hides it from
    the protocol list without deleting the entry.

protocol_editor_is_batch=Batch
protocol_editor_is_batch-status=Batch
protocol_editor_is_batch-help=
    # Batch

    Whether this protocol can carry more than one file per transfer, which is
    what lets a caller flag several files and take them in one go.

protocol_editor_bidirectional=Bidirectional
protocol_editor_bidirectional-status=Bidirectional
protocol_editor_bidirectional-help=
    # Bidirectional

    Whether this protocol can send and receive at the same time.

protocol_editor_char_code=Use
protocol_editor_char_code-status=Use
protocol_editor_char_code-help=
    # Use

    The key a caller presses to choose this protocol.

protocol_editor_description=Description
protocol_editor_description-status=Description
protocol_editor_description-help=
    # Description

    The line describing this protocol in the list callers choose from.

protocol_editor_send_cmd=Send Command
protocol_editor_send_cmd-status=Send Command
protocol_editor_send_cmd-help=
    # Send Command

    The command run to send files with an external protocol. Leave it empty for
    a protocol the board implements itself.

protocol_editor_recv_cmd=Receive Command
protocol_editor_recv_cmd-status=Receive Command
protocol_editor_recv_cmd-help=
    # Receive Command

    The command run to receive files with an external protocol.

command_editor_title=CMD.LST Editor
command_editor_header_command=Command
command_editor_header_action=Action
command_editor_header_parameter=Parameter

command_editor_editor=Edit Command
command_editor_keyword=Keyword
command_editor_help=Help
command_editor_security=Security
command_editor_action=Action
command_editor_parameter=Parameter
command_editor_command_type=Command Type

msg_networking_title=Message Networking
msg_networking_qwk=QWK Settings
msg_networking_ftn=FidoNet Settings

ftn_settings_title=FidoNet Settings

ftn_run_label=Mailer and Tosser

ftn_process_in=Toss Inbound
ftn_process_in-status=Read the mail waiting in the inbound
ftn_process_in-help=When this is off nothing is read out of the inbound, which is how a board is taken out of the network without losing what arrives.

ftn_process_out=Scan Outbound
ftn_process_out-status=Pack what was written here for the links
ftn_process_out-help=When this is off nothing written on this board leaves it.

ftn_process_orphan=Toss Orphans
ftn_process_orphan-status=Read packets addressed to another system
ftn_process_orphan-help=A hub reads mail that is not addressed to it, a leaf node has no business doing so. Packets for somebody else are left in the inbound while this is off.

ftn_dial_out=Call Links
ftn_dial_out-status=Allow this board to call the links
ftn_dial_out-help=A board that is only ever called leaves this off.

ftn_import_after_xfer=Toss After Call
ftn_import_after_xfer-status=Toss the inbound when a call has ended
ftn_import_after_xfer-help=What the link handed over is read into the message bases straight away instead of waiting for the next toss.

ftn_verbose_log=Verbose Log
ftn_verbose_log-status=Say what the mailer is doing, not only what went wrong
ftn_verbose_log-help=
    # Verbose Log

    Records what the mailer does, not only what failed. Worth turning on while
    a new link is being set up, and off again once it runs.

ftn_dupes_label=Duplicates

ftn_check_dupe_msg_id=Check Message ID
ftn_check_dupe_msg_id-status=Drop a message whose id was seen in the area before
ftn_check_dupe_msg_id-help=A message travels several ways through a network and arrives more than once. The message id is what tells the copies apart.

ftn_check_dupe_path=Check Path
ftn_check_dupe_path-status=Drop a message whose path already names this board
ftn_check_dupe_path-help=A message that has been here before has come back the long way around, which the id check misses when the message carries no id.

ftn_msgs_to_track=Messages Tracked
ftn_msgs_to_track-status=How far back the duplicate check looks, 0 for the whole area
ftn_msgs_to_track-help=A busy area holds a long list of message ids, and reading all of them costs time on every run.

ftn_areas_label=Areas

ftn_auto_add=Add Unknown Areas
ftn_auto_add-status=Make an area out of a tag no area carries
ftn_auto_add-help=Without this a message for an unknown tag is counted and dropped.

ftn_auto_add_conference=Add To Conference
ftn_auto_add_conference-status=The conference an area added that way belongs to
ftn_auto_add_conference-help=
    # Add To Conference

    The conference that receives areas the tosser adds by itself, so newly
    offered areas end up somewhere known instead of at random.

ftn_pass_thru=Pass Through
ftn_pass_thru-status=Hand an area on without storing it here
ftn_pass_thru-help=A hub feeds its downlinks areas it does not read itself. The message is offered to every link that asked for the tag and has not seen it yet.

ftn_mail_label=Netmail

ftn_secure=Secure Netmail
ftn_secure-status=Keep netmail for an unknown name apart
ftn_secure-help=Netmail addressed to a name no user carries goes to a base of its own instead of the netmail the sysop reads.

ftn_sysop_change=Deliver To Sysop
ftn_sysop_change-status=Netmail to "Sysop" goes to the name the sysop reads under
ftn_sysop_change-help=
    # Deliver To Sysop

    Netmail addressed to "Sysop" is handed to the name the sysop actually reads
    under, so mail from other systems is not left for a caller nobody reads as.

ftn_default_zone=Default Zone
ftn_default_zone-status=The zone a two dimensional packet is completed with
ftn_default_zone-help=An old packet leaves the zone at zero and only the sysop knows which network it meant.

ftn_default_net=Default Net
ftn_default_net-status=The net a two dimensional packet is completed with
ftn_default_net-help=
    # Default Net

    The net number used to complete an address that arrives without one, which
    older two dimensional packets do.

ftn_paths_label=Directories

ftn_inbound=Inbound
ftn_inbound-status=Where a call drops what it received
ftn_inbound-help=
    # Inbound

    The directory where a call leaves what it brought, until the tosser reads it
    into the message bases.

ftn_outbound=Outbound
ftn_outbound-status=Where mail waits for the next call
ftn_outbound-help=
    # Outbound

    The directory where mail waits for the next call to a link. Anything sitting
    here has not been delivered yet.

ftn_netmail=Netmail Base
ftn_netmail-status=The message base arriving netmail is written to
ftn_netmail-help=
    # Netmail Base

    The message base that arriving netmail is written to, the private mail
    between systems rather than the echoed conferences.

ftn_bad_netmail=Unknown Netmail
ftn_bad_netmail-status=Where netmail for an unknown name goes
ftn_bad_netmail-help=Only used while Secure Netmail is on.

ftn_new_areas=New Areas
ftn_new_areas-status=Where the base of an added area is created
ftn_new_areas-help=Only used while Add Unknown Areas is on.

qwk_settings_title=QWK Settings

qwk_bbs_label=BBS Information
qwk_bbs_name=Name
qwk_bbs_name-status=BBS Name
qwk_bbs_name-help=
    # Name

    The board name written into the QWK packets callers download, which is how
    their offline reader labels this board.

qwk_bbs_city_and_state=City and State
qwk_bbs_city_and_state-status=BBS City and State
qwk_bbs_city_and_state-help=
    # City and State

    Where the board is, as written into the QWK packet.

qwk_bbs_phone_number=Phone
qwk_bbs_phone_number-status=BBS Phone
qwk_bbs_phone_number-help=
    # Phone

    The phone number written into the QWK packet. Leave it empty on a board that
    is only reached over the network.

qwk_bbs_sysop_name=Sysop
qwk_bbs_sysop_name-status=BBS Sysop
qwk_bbs_sysop_name-help=
    # Sysop

    The sysop name written into the QWK packet.

qwk_bbs_id=ID
qwk_bbs_id-status=BBS ID
qwk_bbs_id-help=
    # ID

    The short identifier of the board, up to eight characters. It becomes the
    name of the packet, so it should be unique among the boards a caller reads.

qwk_files_label=QWK Files

qwk_welcome_screen=Welcome Screen
qwk_welcome_screen-status=QWK Welcome Screen
qwk_welcome_screen-help=
    # Welcome Screen

    The screen packed into the QWK packet as its welcome, shown by the caller's
    offline reader.

qwk_goodbye_screen=Goodbye Screen
qwk_goodbye_screen-status=QWK Goodbye Screen
qwk_goodbye_screen-help=
    # Goodbye Screen

    The screen packed into the QWK packet as its goodbye.

qwk_news_sceen=News Screen
qwk_news_sceen-status=QWK News Screen
qwk_news_sceen-help=
    # News Screen

    The news packed into the QWK packet, so a caller who only reads offline still
    sees what is going on.

message_box_info_title= Information 
message_box_warning_title= Warning 
message_box_error_title= Error 
message_box_dismiss= Press ENTER 
no_file_name_given=No file name has been configured for this entry.

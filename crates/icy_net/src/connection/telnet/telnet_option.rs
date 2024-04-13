use std::io;

/**
<http://www.iana.org/assignments/telnet-options/telnet-options.xhtml>
*/

/// <https://www.rfc-editor.org/rfc/rfc856>
pub const TRANSMIT_BINARY: u8 = 0x00;
/// <https://www.rfc-editor.org/rfc/rfc857>
pub const ECHO: u8 = 0x01;
/// ???
pub const RECONNECTION: u8 = 0x02;
/// <https://www.rfc-editor.org/rfc/rfc858>
pub const SUPPRESS_GO_AHEAD: u8 = 0x03;
/// <https://www.rfc-editor.org/rfc/rfc859>
pub const STATUS: u8 = 0x05;
/// <https://www.rfc-editor.org/rfc/rfc860>
pub const TIMING_MARK: u8 = 0x06;
/// <https://www.rfc-editor.org/rfc/rfc726.html>
pub const REMOTE_CONTROLLED_TRANS_AND_ECHO: u8 = 0x07;
/// ???
pub const OUTPUT_LINE_WIDTH: u8 = 0x08;
/// ???
pub const OUTPUT_PAGE_SIZE: u8 = 0x09;
///<https://www.rfc-editor.org/rfc/RFC652>
pub const OUTPUT_CARRIAGE_RETURN_DISPOSITION: u8 = 10;
///<https://www.rfc-editor.org/rfc/RFC653>
pub const OUTPUT_HORIZONTAL_TAB_STOPS: u8 = 11;
///<https://www.rfc-editor.org/rfc/RFC654>
pub const OUTPUT_HORIZONTAL_TAB_DISPOSITION: u8 = 12;
///<https://www.rfc-editor.org/rfc/RFC655>
pub const OUTPUT_FORMFEED_DISPOSITION: u8 = 13;
///<https://www.rfc-editor.org/rfc/RFC656>
pub const OUTPUT_VERTICAL_TABSTOPS: u8 = 14;
///<https://www.rfc-editor.org/rfc/RFC657>
pub const OUTPUT_VERTICAL_TAB_DISPOSITION: u8 = 15;
///<https://www.rfc-editor.org/rfc/RFC658>
pub const OUTPUT_LINEFEED_DISPOSITION: u8 = 16;
///<https://www.rfc-editor.org/rfc/RFC698>
pub const EXTENDED_ASCII: u8 = 17;
///<https://www.rfc-editor.org/rfc/RFC727>
pub const LOGOUT: u8 = 18;
///<https://www.rfc-editor.org/rfc/RFC735>
pub const BYTE_MACRO: u8 = 19;
///<https://www.rfc-editor.org/rfc/RFC1043][RFC732>
pub const DATA_ENTRY_TERMINAL: u8 = 20;
///<https://www.rfc-editor.org/rfc/RFC736][RFC734>
pub const SUP_DUP: u8 = 21;
///<https://www.rfc-editor.org/rfc/RFC749>
pub const SUP_DUP_OUTPUT: u8 = 22;
///<https://www.rfc-editor.org/rfc/RFC779>
pub const SEND_LOCATION: u8 = 23;
/// <https://www.rfc-editor.org/rfc/rfc1091>
pub const TERMINAL_TYPE: u8 = 24;
/// <https://www.rfc-editor.org/rfc/rfc885>
pub const END_OF_RECORD: u8 = 25;
/// <https://www.rfc-editor.org/rfc/rfc1073>
pub const NEGOTIATE_ABOUT_WINDOW_SIZE: u8 = 31;
/// <https://www.rfc-editor.org/rfc/rfc1079>
pub const TERMINAL_SPEED: u8 = 32;
/// <https://www.rfc-editor.org/rfc/rfc1372>
pub const TOGGLE_FLOW_CONTROL: u8 = 33;
/// <https://www.rfc-editor.org/rfc/rfc1184>
pub const LINE_MODE: u8 = 34;
/// <https://www.rfc-editor.org/rfc/rfc1096>
pub const XDISPLAY_LOCATION: u8 = 35;
/// <https://www.rfc-editor.org/rfc/rfc1408>
pub const ENVIRONMENT_OPTION: u8 = 36;
/// <https://www.rfc-editor.org/rfc/rfc2941>
pub const AUTHENTICATION: u8 = 37;
/// <https://www.rfc-editor.org/rfc/rfc2946>
pub const ENCRYPT: u8 = 38;
/// <https://www.rfc-editor.org/rfc/rfc1572>
pub const NEW_ENVIRON: u8 = 39;
///<https://www.rfc-editor.org/rfc/RFC2355>
pub const TN3270E: u8 = 40;
///<https://www.rfc-editor.org/rfc/Rob_Earhart>
pub const XAUTH: u8 = 41;
///<https://www.rfc-editor.org/rfc/RFC2066>
pub const CHAR_SET: u8 = 42;
///<https://www.rfc-editor.org/rfc/Robert_Barnes>
pub const TELNET_REMOTE_SERIAL_PORT_RSP: u8 = 43;
///<https://www.rfc-editor.org/rfc/RFC2217>
pub const COM_PORT_CONTROL_OPTION: u8 = 44;
///<https://www.rfc-editor.org/rfc/Wirt_Atmar>
pub const TELNET_SUPPRESS_LOCAL_ECHO: u8 = 45;
///<https://www.rfc-editor.org/rfc/Michael_Boe>
pub const TELNET_START_TLS: u8 = 46;
///<https://www.rfc-editor.org/rfc/RFC2840>
pub const KERMIT: u8 = 47;
///<https://www.rfc-editor.org/rfc/David_Croft>
pub const SEND_URL: u8 = 48;
///<https://www.rfc-editor.org/rfc/Jeffrey_Altman>
pub const FORWARD_X: u8 = 49;
// 50-137 	Unassigned
pub const TEL_OPT_PRAGMA_LOGON: u8 = 138;
///<https://www.rfc-editor.org/rfc/Steve_McGregory>
pub const TEL_OPT_SSPILOGON: u8 = 139;
///<https://www.rfc-editor.org/rfc/Steve_McGregory>
pub const TEL_OPT_PRAGMA_HEARTBEAT: u8 = 140;
///<https://www.rfc-editor.org/rfc/Steve_McGregory>
// 141-254 	Unassigned
/// <https://www.rfc-editor.org/rfc/rfc861>
pub const EXTENDED_OPTIONS_LIST: u8 = 0xFF;

pub fn check(byte: u8) -> io::Result<u8> {
    match byte {
        0..=49 | 138..=140 | 255 => Ok(byte),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unknown option: {byte}/x{byte:02X}"),
        )),
    }
}

pub fn to_string(byte: u8) -> &'static str {
    match byte {
        TRANSMIT_BINARY => "TransmitBinary",
        ECHO => "Echo",
        RECONNECTION => "Reconnection",
        SUPPRESS_GO_AHEAD => "SuppressGoAhead",
        STATUS => "Status",
        TIMING_MARK => "TimingMark",
        REMOTE_CONTROLLED_TRANS_AND_ECHO => "RemoteControlledTransAndEcho",
        OUTPUT_LINE_WIDTH => "OutputLineWidth",
        OUTPUT_PAGE_SIZE => "OutputPageSize",
        OUTPUT_CARRIAGE_RETURN_DISPOSITION => "OutputCarriageReturnDisposition",
        OUTPUT_HORIZONTAL_TAB_STOPS => "OutputHorizontalTabStops",
        OUTPUT_HORIZONTAL_TAB_DISPOSITION => "OutputHorizontalTabDisposition",
        OUTPUT_FORMFEED_DISPOSITION => "OutputFormfeedDisposition",
        OUTPUT_VERTICAL_TABSTOPS => "OutputVerticalTabstops",
        OUTPUT_VERTICAL_TAB_DISPOSITION => "OutputVerticalTabDisposition",
        OUTPUT_LINEFEED_DISPOSITION => "OutputLinefeedDisposition",
        EXTENDED_ASCII => "ExtendedASCII",
        LOGOUT => "Logout",
        BYTE_MACRO => "ByteMacro",
        DATA_ENTRY_TERMINAL => "DataEntryTerminal",
        SUP_DUP => "SupDup",
        SUP_DUP_OUTPUT => "SupDupOutput",
        SEND_LOCATION => "SendLocation",
        TERMINAL_TYPE => "TerminalType",
        END_OF_RECORD => "EndOfRecord",
        NEGOTIATE_ABOUT_WINDOW_SIZE => "NegotiateAboutWindowSize",
        TERMINAL_SPEED => "TerminalSpeed",
        TOGGLE_FLOW_CONTROL => "ToggleFlowControl",
        LINE_MODE => "LineMode",
        XDISPLAY_LOCATION => "XDisplayLocation",
        ENVIRONMENT_OPTION => "EnvironmentOption",
        AUTHENTICATION => "Authentication",
        ENCRYPT => "Encrypt",
        NEW_ENVIRON => "NewEnviron",
        TN3270E => "TN3270E",
        XAUTH => "XAuth",
        CHAR_SET => "CharSet",
        TELNET_REMOTE_SERIAL_PORT_RSP => "TelnetRemoteSerialPortRSP",
        COM_PORT_CONTROL_OPTION => "ComPortControlOption",
        TELNET_SUPPRESS_LOCAL_ECHO => "TelnetSuppressLocalEcho",
        TELNET_START_TLS => "TelnetStartTLS",
        KERMIT => "Kermit",
        SEND_URL => "SendURL",
        FORWARD_X => "ForwardX",
        TEL_OPT_PRAGMA_LOGON => "TelOptPragmaLogon",
        TEL_OPT_SSPILOGON => "TelOptSSPILogon",
        TEL_OPT_PRAGMA_HEARTBEAT => "TelOptPragmaHeartbeat",
        EXTENDED_OPTIONS_LIST => "ExtendedOptionsList",
        _ => "Unknown",
    }
}

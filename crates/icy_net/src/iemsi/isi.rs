/// The ISI packet is used by the Server to transmit its configuration
/// and Client-related information to the Client. It contains Server data
/// and capabilities.
#[derive(Clone)]
pub struct EmsiISI {
    /// The name, version number, and optionally the serial number of the
    /// Server software. Eg. {RemoteAccess,1.10/b5,CS000001}.
    pub id: String,
    /// The name of the Server system. Eg. {Advanced Engineering S.A.R.L.}.
    pub name: String,
    /// The geographical location of the user, ie. Stockholm, Sweden.
    pub location: String,
    /// The name of the primary operator of the Server software. Eg. {Joaquim H. Homrighausen}.
    pub operator: String,
    /// Hexadecimal string representing a long integer containing the current
    /// time of the Server in UNIX notation (number of seconds since midnight,
    /// Jan 1 1970). This must be treated case insensitively by the Client.
    pub localtime: String,
    /// May contain copyright notices, system information, etc. This field may optionally be displayed by the Client.
    pub notice: String,
    /// A single character used by the Server to indicate that the user
    /// has to press the <Enter> key to resume operation. This is used in
    /// conjunction with ASCII Image Downloads (see ISM packet).
    pub wait: String,
    /// The capabilities of the Server software. No Server software
    /// capabilities have currently been defined.
    pub capabilities: String,
}

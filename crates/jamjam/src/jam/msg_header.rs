use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use bstr::BString;

use crate::{
    jam::{JamError, JAM_SIGNATURE},
    util::crc32::CRC_SEED,
};

use super::{attributes, JamMessageBase};

#[derive(Clone, Debug)]
pub struct JamMessageHeader {
    /// <J><A><M> followed by <NUL>
    //pub signature: u32,

    /// Revision level of header
    ///
    /// # Remarks
    /// This field is intended for future revisions of the specifications
    /// to allow the use of a different fixed-length binary message
    /// header. The current revision level is one (1).
    //
    // pub revision: u16,

    /// Reserved for future use
    // unused
    //    pub reserved_word: u16,

    /// Length of subfields
    ///
    /// The SubfieldLen field is set to zero (0) if the header does not
    /// have any subfield data. I.e. the length of the binary header is
    /// not included in this field.
    pub sub_fields: Vec<MessageSubfield>,

    // unused
    // pub subfield_len: u32,
    /// Number of times message read
    pub times_read: u32,

    /// CRC-32 of MSGID line
    ///
    /// # Remarks
    /// When calculating the CRC-32 of the MSGID and REPLY lines, the
    /// text ^aMSGID: and ^aREPLY: should be removed as well as all
    /// leading and trailing white space characters.
    pub msgid_crc: u32,

    /// CRC-32 of REPLY line
    ///
    /// # Remarks
    /// When calculating the CRC-32 of the MSGID and REPLY lines, the
    /// text ^aMSGID: and ^aREPLY: should be removed as well as all
    /// leading and trailing white space characters.
    pub replycrc: u32,

    /// This msg is a reply to..
    pub reply_to: u32,

    /// First reply to this msg
    pub reply1st: u32,

    /// Next msg in reply chain
    pub replynext: u32,

    /// When msg was written (UNIX time)
    pub date_written: u32,

    /// When msg was read by recipient (UNIX time)
    pub date_received: u32,

    /// When msg was processed by tosser/scanner (UNIX time)
    pub date_processed: u32,

    /// Message number (1-based)
    pub message_number: u32,

    /// Msg attribute, see "Msg Attributes"
    pub attributes: u32,

    /// Reserved for future use
    pub attributes2: u32,

    /// Offset of text in ????????.JDT file
    pub offset: u32,

    /// Length of message text
    pub txt_len: u32,

    /// CRC-32 of password to access message
    /// Set to CRC_SEED (0xFFFFFFFF) for no password.
    pub password_crc: u32,

    /// Cost of message
    pub cost: u32,
}

impl Default for JamMessageHeader {
    fn default() -> Self {
        Self {
            sub_fields: Vec::new(),
            times_read: 0,
            msgid_crc: CRC_SEED,
            replycrc: CRC_SEED,
            reply_to: 0,
            reply1st: 0,
            replynext: 0,
            date_written: 0,
            date_received: 0,
            date_processed: 0,
            message_number: 0,
            attributes: 0,
            attributes2: 0,
            offset: 0,
            txt_len: 0,
            password_crc: CRC_SEED,
            cost: 0,
        }
    }
}

impl JamMessageHeader {
    pub const FIXED_HEADER_SIZE: usize = 76;

    pub fn get_subject(&self) -> Option<&BString> {
        for s in &self.sub_fields {
            if s.get_type() == &SubfieldType::Subject {
                return Some(&s.content);
            }
        }
        None
    }

    pub fn get_from(&self) -> Option<&BString> {
        for s in &self.sub_fields {
            if s.get_type() == &SubfieldType::SenderName {
                return Some(&s.content);
            }
        }
        None
    }

    pub fn get_to(&self) -> Option<&BString> {
        for s in &self.sub_fields {
            if s.get_type() == &SubfieldType::RecvName {
                return Some(&s.content);
            }
        }
        None
    }

    /// True, if a password is required to access this msg base
    pub fn needs_password(&self) -> bool {
        self.password_crc != CRC_SEED
    }

    /// Checks if a password is valid.
    pub fn is_password_valid(&self, password: &str) -> bool {
        self.password_crc == CRC_SEED || self.password_crc == JamMessageBase::get_crc(&BString::new(password.as_bytes().to_vec()))
    }

    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let data = &mut [0; Self::FIXED_HEADER_SIZE];
        file.read_exact(data)?;
        if !data.starts_with(&JAM_SIGNATURE) {
            return Err(Box::new(JamError::InvalidHeaderSignature));
        }
        let data = &data[4..];
        convert_single_u16!(revision, data);
        if revision != 1 {
            return Err(Box::new(JamError::UnsupportedMessageHeaderRevision(revision)));
        }
        let mut data = &data[4..];
        // convert_u32!(reserved_word, data);
        convert_u32!(subfield_data_len, data); // length of subfields in bytes
        convert_u32!(times_read, data);
        convert_u32!(msgid_crc, data);
        convert_u32!(reply_crc, data);
        convert_u32!(reply_to, data);
        convert_u32!(reply1st, data);
        convert_u32!(replynext, data);
        convert_u32!(date_written, data);
        convert_u32!(date_received, data);
        convert_u32!(date_processed, data);
        convert_u32!(message_number, data);
        convert_u32!(attribute, data);
        convert_u32!(attribute2, data);
        convert_u32!(offset, data);
        convert_u32!(txt_len, data);
        convert_u32!(password_crc, data);
        convert_u32!(cost, data);

        let mut subfield_data = vec![0; subfield_data_len as usize];
        file.read_exact(&mut subfield_data)?;

        let mut sub_fields = Vec::new();
        let mut idx = 0;
        while idx < subfield_data_len as usize {
            let sub_field = MessageSubfield::deserialize(&subfield_data, &mut idx)?;
            sub_fields.push(sub_field);
        }

        Ok(Self {
            sub_fields,
            times_read,
            msgid_crc,
            replycrc: reply_crc,
            reply_to,
            reply1st,
            replynext,
            date_written,
            date_received,
            date_processed,
            message_number,
            attributes: attribute,
            attributes2: attribute2,
            offset,
            txt_len,
            password_crc,
            cost,
        })
    }

    pub fn write(&self, file: &mut BufWriter<File>) -> crate::Result<()> {
        file.write_all(&JAM_SIGNATURE)?;
        // revision
        file.write_all(&u16::to_le_bytes(1))?;
        // reserved_word
        file.write_all(&u16::to_le_bytes(0))?;
        let subfield_data_len = self.sub_fields.iter().map(|sf| 8 + sf.content.len()).sum::<usize>();
        file.write_all(&u32::to_le_bytes(subfield_data_len as u32))?;
        file.write_all(&self.times_read.to_le_bytes())?;
        file.write_all(&self.msgid_crc.to_le_bytes())?;
        file.write_all(&self.replycrc.to_le_bytes())?;
        file.write_all(&self.reply_to.to_le_bytes())?;
        file.write_all(&self.reply1st.to_le_bytes())?;
        file.write_all(&self.replynext.to_le_bytes())?;
        file.write_all(&self.date_written.to_le_bytes())?;
        file.write_all(&self.date_received.to_le_bytes())?;
        file.write_all(&self.date_processed.to_le_bytes())?;
        file.write_all(&self.message_number.to_le_bytes())?;
        file.write_all(&self.attributes.to_le_bytes())?;
        file.write_all(&self.attributes2.to_le_bytes())?;
        file.write_all(&self.offset.to_le_bytes())?;
        file.write_all(&self.txt_len.to_le_bytes())?;
        file.write_all(&self.password_crc.to_le_bytes())?;
        file.write_all(&self.cost.to_le_bytes())?;
        for sub_field in &self.sub_fields {
            let id: u32 = sub_field.field_type.into();
            file.write_all(&id.to_le_bytes())?;
            file.write_all(&u32::to_le_bytes(sub_field.content.len() as u32))?;
            file.write_all(&sub_field.content)?;
        }
        Ok(())
    }

    pub fn is_deleted(&self) -> bool {
        self.attributes & attributes::MSG_DELETED != 0
    }

    pub fn is_locked(&self) -> bool {
        self.attributes & attributes::MSG_LOCKED != 0
    }

    pub fn is_private(&self) -> bool {
        self.attributes & attributes::MSG_PRIVATE != 0
    }

    pub fn is_read(&self) -> bool {
        self.attributes & attributes::MSG_READ != 0
    }

    pub fn is_receipt_req(&self) -> bool {
        self.attributes & attributes::MSG_RECEIPTREQ != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubfieldType {
    /// Unknown subfield type
    Unknown(u32),

    /// A network address. This is used to specify the originating address.
    /// More than one OADDRESS field may exist. DATLEN must not exceed 100
    /// characters. For a FidoNet-style address, this field must follow the
    /// ZONE:NET/NODE.POINT@DOMAIN format where .POINT is excluded if zero
    /// and @DOMAIN is excluded if unknown.
    Address0,

    /// network address. This is used to specify the destination address.
    /// More than one DADDRESS field may exist (e.g. carbon copies). DATLEN
    /// must not exceed 100 characters. For a FidoNet-style address, this
    /// field must follow the ZONE:NET/NODE.POINT@DOMAIN format where .POINT
    /// is excluded if zero and @DOMAIN is excluded if unknown.
    AddressD,

    /// The sender (author) of the message. DATLEN must not exceed 100 characters.
    SenderName,

    /// The recipient of the message. DATLEN must not exceed 100 characters.
    RecvName,

    /// Used to store the message identification data. All data not relevant
    /// to the actual ID string, including leading and trailing white space
    /// characters should be removed. DATLEN must not exceed 100 characters.
    MsgID,

    /// Used to store the message reply data. All data not relevant to the
    /// actual reply string, including leading and trailing white space
    /// characters should be removed. DATLEN must not exceed 100 characters.
    ReplyID,

    /// The subject of the message. DATLEN must not exceed 100 characters.
    /// Note that this field may not be used for FidoNet-style file attaches
    /// or file requests.
    Subject,

    /// Used to store the FTN PID kludge line. Only the actual PID data is
    /// stored and ^aPID: is stripped along with any leading and trailing
    /// white space characters. DATLEN must not exceed 40 characters.
    PID,

    /// This is also referred to as ^aVia information in FTNs. It contains
    /// information about a system which the message has travelled through.
    /// The format of the field is  where:
    ///
    /// YYYY is the year (1992-9999)
    ///   MM is the month (01-12)
    ///   DD is the day (01-31)
    ///   HH is the hour (00-23)
    ///   MM is the minute (00-59)
    ///   SS is the second (00-59)
    ///
    /// The timestamp is stored in ASCII (0-9) characters. The network
    /// address is the address of the system. It is expressed in ASCII
    /// notation in the native format of the forwarding system.
    Trace,

    /// A file attached to the message. Only one filename may be specified
    /// per subfield. No wildcard characters are allowed. If this subfield
    /// is present in a message header, the ATTRIBUTE must include the
    /// MSG_FILEATTACH bit.
    EnclFile,

    /// Identical to ENCLOSEDFILE with the exception that the filename is
    /// followed by a  (00H) and an alias filename to be transmited to
    /// the remote system in place of the local name of the file.
    EnclFwAlias,

    /// A request for one or more files. Only one filemask may be specified
    /// per subfield. If the filemask contains a complete path, it is to be
    /// regarded as an update file request. If this subfield is present in a
    /// message header, the ATTRIBUTE must include the MSG_FILEREQUEST bit.
    /// To indicate that a password is to be transmitted along with the
    /// request, a  (00H) character followed by the password is
    /// appended. E.g. SECRET*.*MYPASSWORD.
    EnclFreq,

    /// One or more files attached to the message. Only one filename may be
    /// specified per subfield. Wildcard characters are allowed. If this
    /// subfield is present in a message header, the ATTRIBUTE must include
    /// the MSG_FILEATTACH bit.
    EnclFieleWc,

    /// One or more files attached to the message. The filename points to an
    /// ASCII file with one filename entry per line. If alias filenames are
    /// to be used, they are specified after the actual filename and
    /// separated by a  (00H) character, e.g. C:\MYFILE.LZHNEWS.
    /// Wildcard characters are not allowed.
    EnclIndFile,

    /// Reserved for future use.
    EmbInDat,

    /// An FTS-compliant "kludge" line not otherwise represented here. All
    /// data not relevant to the actual kludge line, including leading and
    /// trailing white space and ^A (01H) characters should be removed.
    /// DATLEN must not exceed 255 characters. The FTS kludges INTL, TOPT,
    /// and FMPT must never be stored as separate SubFields. Their data must
    /// be extracted and used for the address SubFields.
    FTSKludge,

    /// Used to store two-dimensional (net/node) SEEN-BY information often
    /// used in FTN conference environments. Only the actual SEEN-BY data is
    /// stored and ^aSEEN-BY: or SEEN-BY: is stripped along with any leading
    /// and trailing white space characters.
    SeenBy2D,

    /// Used to store two-dimensional (net/node) PATH information often used
    /// in FTN conference environments. Only the actual PATH data is stored
    /// and ^aPATH: is stripped along with any leading and trailing white
    /// space characters.
    Path2D,

    /// Used to store the FTN FLAGS kludge information. Note that all FLAG
    /// options that have binary representation in the JAM message header
    /// must be removed from the FLAGS string prior to storing it. Only
    /// the actual flags option string is stored and ^aFLAGS is stripped
    /// along with any leading and trailing white space characters.
    Flags,

    /// Time zone information. This subfield consists of four mandatory
    /// bytes and one optional. The first character may be a plus (+) or a
    /// minus (-) character to indicate a location east (plus) or west
    /// (minus) of UTC 0000. The plus character is implied unless the first
    /// character is a minus character. The following four bytes must be
    /// digits in the range zero through nine and indicates the offset in
    /// hours and minutes. E.g. 0100 indicates an offset of one hour east of
    /// UTC.
    TZUTCInfo,

    /// IcyBoard extensions

    // Note: DateReceived doesn't make sense to add here
    //       The unix time however can be guessed it's the next unix time after the DateWritten
    /// Like DateWritten but as RFC3339
    DateWritten,
    /// Like DateProcessed but as RFC3339
    DateProcessed,
    // PCBoard packout date (RFC3339) for auto delete
    PackoutDate,
}

impl From<u32> for SubfieldType {
    fn from(value: u32) -> Self {
        match value {
            0 => SubfieldType::Address0,
            1 => SubfieldType::AddressD,
            2 => SubfieldType::SenderName,
            3 => SubfieldType::RecvName,
            4 => SubfieldType::MsgID,
            5 => SubfieldType::ReplyID,
            6 => SubfieldType::Subject,
            7 => SubfieldType::PID,
            8 => SubfieldType::Trace,
            9 => SubfieldType::EnclFile,
            10 => SubfieldType::EnclFwAlias,
            11 => SubfieldType::EnclFreq,
            12 => SubfieldType::EnclFieleWc,
            13 => SubfieldType::EnclIndFile,
            1000 => SubfieldType::EmbInDat,
            2000 => SubfieldType::FTSKludge,
            2001 => SubfieldType::SeenBy2D,
            2002 => SubfieldType::Path2D,
            2003 => SubfieldType::Flags,
            2204 => SubfieldType::TZUTCInfo,

            7001 => SubfieldType::DateWritten,
            7002 => SubfieldType::DateProcessed,
            7003 => SubfieldType::PackoutDate,
            _ => SubfieldType::Unknown(value),
        }
    }
}

impl From<SubfieldType> for u32 {
    fn from(value: SubfieldType) -> u32 {
        match value {
            SubfieldType::Address0 => 0,
            SubfieldType::AddressD => 1,
            SubfieldType::SenderName => 2,
            SubfieldType::RecvName => 3,
            SubfieldType::MsgID => 4,
            SubfieldType::ReplyID => 5,
            SubfieldType::Subject => 6,
            SubfieldType::PID => 7,
            SubfieldType::Trace => 8,
            SubfieldType::EnclFile => 9,
            SubfieldType::EnclFwAlias => 10,
            SubfieldType::EnclFreq => 11,
            SubfieldType::EnclFieleWc => 12,
            SubfieldType::EnclIndFile => 13,
            SubfieldType::EmbInDat => 1000,
            SubfieldType::FTSKludge => 2000,
            SubfieldType::SeenBy2D => 2001,
            SubfieldType::Path2D => 2002,
            SubfieldType::Flags => 2003,
            SubfieldType::TZUTCInfo => 2204,

            SubfieldType::DateWritten => 7001,
            SubfieldType::DateProcessed => 7002,
            SubfieldType::PackoutDate => 7003,

            SubfieldType::Unknown(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MessageSubfield {
    field_type: SubfieldType,
    content: BString,
}

impl MessageSubfield {
    pub fn new(field_type: SubfieldType, content: BString) -> Self {
        Self { field_type, content }
    }

    pub fn get_string(&self) -> &BString {
        &self.content
    }

    pub fn get_type(&self) -> &SubfieldType {
        &self.field_type
    }

    fn deserialize(subfield_data: &[u8], idx: &mut usize) -> crate::Result<Self> {
        let mut data = &subfield_data[*idx..];
        if data.len() < 8 {
            return Err(JamError::InvalidSubfieldLength(data.len() as u32, 8).into());
        }
        convert_u32!(id, data);
        convert_u32!(data_len, data);
        *idx += 8;
        let end = *idx + data_len as usize;
        if end > subfield_data.len() {
            return Err(JamError::InvalidSubfieldLength(data_len, subfield_data.len()).into());
        }
        let buffer = subfield_data[*idx..end].to_vec();
        *idx = end;
        Ok(Self {
            field_type: id.into(),
            content: BString::new(buffer),
        })
    }
}

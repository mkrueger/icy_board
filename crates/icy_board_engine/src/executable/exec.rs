use std::{fs, io::stdout, path::Path};

use crossterm::execute;
use crossterm::style::{Attribute, Print, SetAttribute};
use thiserror::Error;

use crate::Res;
use crate::crypt::{decode_rle, decrypt_chunks, encode_rle};
use crate::executable::disassembler::DisassembleVisitor;

use super::{LAST_PPLC, VariableTable, VariableType};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ExecutableError {
    #[error("Invalid PPE file")]
    InvalidPPEFile,

    #[error("Unsupported version: {0} (Only up to {LAST_PPLC})")]
    UnsupporrtedVersion(u16),

    #[error("Too many declarations: {0}")]
    TooManyDeclarations(usize),

    #[error("String constant too long: {0}")]
    StringConstantTooLong(usize),

    #[error("String type invalid: {0}")]
    StringTypeInvalid(VariableType),

    #[error("Invalid index in variable table: {0} > max:{1}")]
    InvalidVariableIndexInTable(usize, usize),

    #[error("Function/Procedure header type mismatch: {0:?} != {1:?}")]
    FunctionHeaderTypeMismatch(VariableType, VariableType),

    #[error("Buffer too short: {0}")]
    BufferTooShort(usize),

    #[error("Function/Procedure is not supported in ppe version ({0})")]
    FunctionsNotSupported(u16),

    #[error("Variable count exceeds maximum: {0} ({1})")]
    VariableCountExceedsMaximum(usize, usize),

    #[error("Variable id mismatch: {0} != {1}")]
    VariableIdMismatch(usize, i32),
}

#[derive(Clone)]
pub struct Executable {
    pub runtime: u16,
    pub variable_table: VariableTable,
    pub script_buffer: Vec<i16>,
}

static PREAMBLE: &[u8] = b"PCBoard Programming Language Executable";

const HEADER_SIZE: usize = 48;

impl Executable {
    /// .
    ///
    /// # Examples
    ///
    /// # Errors
    ///
    /// Panics if .
    pub fn read_file<P: AsRef<Path>>(file_name: &P, print_header_information: bool) -> Res<Self> {
        let mut buffer = fs::read(file_name)?;
        Self::from_buffer(&mut buffer, print_header_information)
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn from_buffer(buffer: &mut [u8], print_header_information: bool) -> Res<Executable> {
        if !buffer.starts_with(PREAMBLE) {
            return Err(Box::new(ExecutableError::InvalidPPEFile));
        }
        let version = ((buffer[40] & 15) as u16 * 10 + (buffer[41] as u16 & 15)) * 100 + (buffer[43] as u16 & 15) * 10 + (buffer[44] as u16 & 15);

        if version > LAST_PPLC {
            return Err(Box::new(ExecutableError::UnsupporrtedVersion(version)));
        }

        let buffer = &mut buffer[HEADER_SIZE..];
        let (mut i, variable_table) = VariableTable::deserialize(version, buffer)?;
        let code_size = u16::from_le_bytes(buffer[i..=(i + 1)].try_into()?) as usize;
        i += 2;
        let real_size = buffer.len() - i;
        if print_header_information {
            execute!(
                stdout(),
                Print("Format ".to_string()),
                SetAttribute(Attribute::Bold),
                Print(format!("{}.{:00}", version / 100, version % 100)),
                SetAttribute(Attribute::Reset),
                Print(" detected ".to_string()),
                SetAttribute(Attribute::Bold),
                Print(format!("{}", variable_table.len())),
                SetAttribute(Attribute::Reset),
                Print(" variables, ".to_string()),
                SetAttribute(Attribute::Bold),
                Print(format!("{code_size}/{real_size} bytes")),
                SetAttribute(Attribute::Reset),
                Print(" code/compressed size,".to_string()),
                SetAttribute(Attribute::Bold),
                Print(format!("{} bytes", i.saturating_sub(2).saturating_sub(HEADER_SIZE))),
                SetAttribute(Attribute::Reset),
                Print(" variable table".to_string()),
                Print("\n".to_string()),
            )?;
        }

        let code_data: &mut [u8] = &mut buffer[i..];
        let decrypted_data;
        let data: &[u8] = if version > 300 {
            let use_rle = real_size != code_size;
            decrypt_chunks(code_data, version, use_rle);
            if use_rle {
                decrypted_data = decode_rle(&code_data);
                &decrypted_data
            } else {
                code_data
            }
        } else {
            code_data
        };
        if data.len() != code_size {
            log::warn!("WARNING: decoded size({}) differs from expected size ({}).", data.len(), code_size);
        }
        let mut script_buffer = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let k = if i + 1 >= data.len() {
                data[i] as i16
            } else {
                i16::from_le_bytes(data[i..i + 2].try_into()?)
            };
            script_buffer.push(k);
            i += 2;
        }
        Ok(Executable {
            runtime: version,
            variable_table,
            script_buffer,
        })
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn to_buffer(&self) -> Result<Vec<u8>, ExecutableError> {
        if self.runtime > LAST_PPLC {
            return Err(ExecutableError::UnsupporrtedVersion(self.runtime));
        }
        let mut buffer = Vec::new();
        buffer.extend_from_slice(PREAMBLE);
        buffer.push(b' ');
        buffer.push(b' ');
        buffer.push(b'0' + (self.runtime / 100) as u8);
        buffer.push(b'.');
        let minor = self.runtime % 100;
        buffer.push(b'0' + (minor / 10) as u8);
        buffer.push(b'0' + (minor % 10) as u8);
        buffer.extend_from_slice(b"\x0D\x0A\x1A");

        self.variable_table.serialize(&mut buffer)?;

        let mut script_buffer = Vec::new();
        for s in &self.script_buffer {
            script_buffer.extend_from_slice(&s.to_le_bytes());
        }
        let mut code_data = encode_rle(&script_buffer);

        buffer.extend_from_slice(&u16::to_le_bytes(self.script_buffer.len() as u16 * 2));
        // in the very unlikely case the rle compressed buffer is larger than the original buffer
        let use_rle = code_data.len() < script_buffer.len() && self.runtime >= 300;
        if !use_rle {
            code_data = script_buffer;
        }
        crate::crypt::encrypt_chunks(&mut code_data, self.runtime, use_rle);
        buffer.extend_from_slice(&code_data);
        Ok(buffer)
    }

    pub fn print_variable_table(&self) {
        self.variable_table.print_variable_table();
    }
    pub fn print_script_buffer_dump(&self) {
        DisassembleVisitor::print_script_buffer_dump(self);
    }
    pub fn print_disassembler(&self) {
        DisassembleVisitor::new(self).print_disassembler();
    }
}

impl Default for Executable {
    fn default() -> Self {
        Self {
            runtime: LAST_PPLC,
            variable_table: VariableTable::default(),
            script_buffer: Vec::new(),
        }
    }
}

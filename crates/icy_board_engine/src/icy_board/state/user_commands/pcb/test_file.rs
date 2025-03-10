use dizbase::file_base::{FileBase, metadata::MetadataType};
use regex::Regex;

use crate::{
    Res,
    icy_board::{
        commands::CommandType,
        icb_text::IceText,
        state::{
            IcyBoardState,
            functions::{MASK_ASCII, display_flags},
        },
    },
};

impl IcyBoardState {
    pub async fn test_file_command(&mut self) -> Res<()> {
        if self.session.current_conference.directories.is_none() || self.session.current_conference.directories.as_ref().unwrap().is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        let mut found = false;

        while !found {
            let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    IceText::TestFileName,
                    40,
                    &MASK_ASCII,
                    CommandType::TestFile.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                )
                .await?
            };
            if search_pattern.is_empty() {
                return Ok(());
            }

            let Ok(search_regex) = Regex::new(&search_pattern) else {
                self.display_text(IceText::PunctuationError, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            };

            self.session.push_tokens(&"A");
            let dir_numbers = self.get_dir_numbers().await?;
            self.session.disp_options.no_change();
            for (_num, _desc, path, metadata) in dir_numbers.numbers {
                let Ok(base) = self.get_filebase(&path, &metadata).await else {
                    continue;
                };
                let header_count = base.lock().await.len();
                for file in 0..header_count {
                    let file_name = base.lock().await[file].name().to_string();
                    if !search_regex.is_match(&file_name) {
                        continue;
                    }
                    found = true;
                    self.session.op_text = file_name;
                    self.display_text(IceText::VerifyingFile, display_flags::DEFAULT).await?;
                    let full_path = path.join(&self.session.op_text);
                    if !full_path.exists() {
                        log::error!("TEST: File not found: {:?}", full_path);
                        self.display_text(IceText::Failed, display_flags::NEWLINE).await?;
                        continue;
                    }

                    let hash = base.lock().await.read_metadata(&full_path)?;
                    let mut found_metadata = false;
                    for data in hash {
                        if data.metadata_type == MetadataType::Hash {
                            found_metadata = true;
                            let hash = u64::from_le_bytes(data.data.as_slice().try_into().unwrap());
                            let current_hash = FileBase::get_hash(&full_path)?;
                            if hash == current_hash {
                                self.display_text(IceText::Passed, display_flags::NEWLINE).await?;
                            } else {
                                log::error!(
                                    "TEST: File hash invalid: {:?} {:08X} != {:08X}",
                                    full_path,
                                    hash,
                                    FileBase::get_hash(&full_path)?
                                );
                                self.display_text(IceText::Failed, display_flags::NEWLINE).await?;
                            }
                            break;
                        }
                    }
                    if !found_metadata {
                        self.display_text(IceText::Passed, display_flags::NEWLINE).await?;
                    }
                }

                if self.session.disp_options.abort_printout {
                    break;
                }
            }

            if !found {
                self.session.op_text = search_pattern;
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE).await?;
            }
        }
        Ok(())
    }
}

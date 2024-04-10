use std::path::Path;

use icy_ppe::Res;
use serde::{Deserialize, Serialize};

use super::{IcyBoardSerializer, PCBoardImport, PCBoardTextImport};

#[derive(Serialize, Deserialize, Default)]
pub struct Language {
    pub description: String,
    pub locale: String,
    pub extension: String,
    pub yes_char: char,
    pub no_char: char,
}

#[derive(Serialize, Deserialize)]
pub struct SupportedLanguages {
    pub date_formats: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "language")]
    pub languages: Vec<Language>,
}

impl Default for SupportedLanguages {
    fn default() -> Self {
        Self {
            date_formats: vec![
                "MM-DD-YY".to_string(),
                "DD-MM-YY".to_string(),
                "YY-MM-DD".to_string(),
                "MM-DD-YYYY".to_string(),
                "DD-MM-YYYY".to_string(),
                "YYYY-MM-DD".to_string(),
            ],
            languages: Default::default(),
        }
    }
}

impl PCBoardTextImport for SupportedLanguages {
    fn import_data(data: String) -> Res<Self> {
        let mut res = SupportedLanguages::default();
        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            let splitted_line = line.split(',').collect::<Vec<&str>>();
            if splitted_line.len() != 6 {
                continue;
            }

            let description = splitted_line[0].to_string();
            let extension = splitted_line[1].to_string().to_ascii_lowercase();
            let lang = if let Ok(lang_code) = splitted_line[2].parse() {
                convert_dos_country_code(lang_code)
            } else {
                "en_US".to_string()
            };
            let yes_char = splitted_line[4].chars().next().unwrap_or('Y');
            let no_char = splitted_line[5].chars().next().unwrap_or('N');

            res.languages.push(Language {
                description,
                extension,
                locale: lang,
                yes_char,
                no_char,
            });
        }
        Ok(res)
    }
}

impl PCBoardImport for SupportedLanguages {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        PCBoardTextImport::import_pcboard(path)
    }
}

impl IcyBoardSerializer for SupportedLanguages {
    const FILE_TYPE: &'static str = "language file";
}

fn convert_dos_country_code(cc: i32) -> String {
    let res = match cc {
        1 => "en_US",   // United_States
        2 => "fr_CA",   // Candian_French
        3 => "es_41",   // Latin_America
        7 => "ru_RU",   // Russia
        30 => "el_GR",  // Greece
        31 => "nl_NL",  // Netherlands
        32 => "fr_BE",  // Belgium
        33 => "fr_FR",  // France
        34 => "es_ES",  // Spain
        36 => "hu_HU",  // Hungary
        38 => "sr_RS",  // Yugoslavia
        39 => "it_IT",  // Italy
        40 => "ro_RO",  // Romania
        41 => "ch_CH",  // Switzerland
        42 => "cs_CZ",  // Czech_Slovak
        43 => "de_AT",  // Austria
        44 => "en_GB",  // United_Kingdom
        45 => "da_DK",  // Denmark
        46 => "sv_SE",  // Sweden
        47 => "nn_NO",  // Norway
        48 => "pl_PL",  // Poland
        49 => "de_DE",  // Germany
        54 => "es_AR",  // Argentina
        55 => "pt_BR",  // Brazil
        60 => "ms_MY",  // Malaysia
        61 => "en_AU",  // Australia
        63 => "fil_P",  // Philippines
        65 => "ms_SG",  // Singapore
        77 => "kk_KZ",  // Kazakhstan
        81 => "ja_JP",  // Japan
        82 => "ko_KR",  // South_Korea
        84 => "vi_VN",  // Vietnam
        86 => "zh_CN",  // China
        90 => "tr_TR",  // Turkey
        91 => "hi_IN",  // India
        227 => "fr_NE", // Niger
        229 => "fr_BJ", // Benin
        234 => "en_NG", // Nigeria
        298 => "fo_FO", // Faeroe_Islands
        351 => "pt_PT", // Portugal
        354 => "is_IS", // Iceland
        355 => "sq_AL", // Albania
        356 => "mt_MT", // Malta
        358 => "fi_FI", // Finland
        359 => "bg_BG", // Bulgaria
        370 => "lt_LT", // Lithuania
        371 => "lv_LV", // Latvia
        372 => "et_EE", // Estonia
        374 => "hy_AM", // Armenia
        375 => "be_BY", // Belarus
        380 => "uk_UA", // Ukraine
        381 => "sr_RS", // Serbia
        382 => "sr_ME", // Montenegro
        384 => "hr_HR", // Croatia
        386 => "sk_SK", // Slovenia
        387 => "bs_BA", // Bosnia
        389 => "mk_MK", // Macedonia
        886 => "zh_TW", // Taiwan
        785 => "ar",    // Arabic
        972 => "he_IL", // Israel
        976 => "mn_MN", // Mongolia
        992 => "tg_TJ", // Tadjikistan
        993 => "tk_TM", // Turkmenistan
        994 => "az_AZ", // Azerbaijan
        995 => "ka_GE", // Georgia
        996 => "ky_KG", // Kyrgyzstan
        998 => "uz_UZ", // Uzbekistan
        _ => "en_US",
    };

    res.to_string()
}

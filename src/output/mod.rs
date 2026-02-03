pub mod json;
pub mod xml;

use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Xml,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "xml" => Ok(OutputFormat::Xml),
            _ => Err(format!("Invalid format: {}. Use text, json, or xml", s)),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FileOutput {
    pub path: String,
    pub score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<usize>,
    pub lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TreeOutput {
    pub project: String,
    pub files: Vec<FileOutput>,
}

#[derive(Serialize, Debug)]
pub struct CatOutput {
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    pub files_shown: usize,
    pub skipped_binary: usize,
    pub skipped_session: usize,
    pub total_lines: usize,
    pub files: Vec<FileOutput>,
}

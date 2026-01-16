use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct FileHeader {
    pub file_name: String,
    pub file_size: u64,
}

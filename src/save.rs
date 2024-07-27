use std::io::ErrorKind;
use std::path::PathBuf;
use color_eyre::Result;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct SaveData {
    pub(crate) is_custom: bool,
    pub(crate) gid_oid: String,
    pub(crate) prev_guess: String,
    pub(crate) prev_emoji: String,
    pub(crate) score: u64,
}

impl SaveData {
    pub(crate) fn template(is_custom: bool, gid_oid: String) -> Self {
        Self {
            is_custom,
            gid_oid,
            ..Default::default()
        }
    }

    pub(crate) fn update(&mut self, prev_guess: String, prev_emoji: String, score: u64) {
        self.prev_guess = prev_guess;
        self.prev_emoji = prev_emoji;
        self.score = score;
    }

    fn save_file() -> Result<PathBuf> {
        let save_dir = dirs::data_local_dir().ok_or(std::io::Error::new(ErrorKind::NotFound, "Could not find data local directory!"))?;
        Ok(save_dir.join("wbr_save.json"))
    }

    pub(crate) fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self)?;
        std::fs::write(Self::save_file()?, &json)?;
        Ok(())
    }

    pub(crate) fn load() -> Result<Option<Self>> {
        let path = Self::save_file()?;
        if path.exists() {
            let json = std::fs::read_to_string(&path)?;
            std::fs::remove_file(&path)?;
            Ok(Some(serde_json::from_str::<Self>(&json)?))
        } else {
            Ok(None)
        }
    }
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            is_custom: false,
            gid_oid: String::new(),
            prev_guess: String::new(),
            prev_emoji: String::new(),
            score: 0,
        }
    }
}
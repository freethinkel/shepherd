//! Apple's AR models of its devices, for the 3D view. They can't ship with Shepherd (Apple licenses no
//! redistribution), so each is fetched from apple.com on this machine the first time it's shown, checked against the
//! checksum t3code recorded for it (its models/sources.json), and kept in the cache.
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const IPHONE_18_PRO: (&str, &str) = (
    "https://www.apple.com/105/media/us/iphone-18-pro/2026/591df885-5ee2-4173-86e6-401021249f7c/ar/iphone-18-pro-e-sim.usdz",
    "0f2dac354fbc6505e7917983de460f2cc68e253f96aabf52e9f0468a4803f317",
);
const IPHONE_DUO: (&str, &str) = (
    "https://www.apple.com/105/media/us/iphone-duo/2026/9305e4b9-72d9-4c05-9381-b572adadd5e5/ar/iPhone_Duo_e-sim_Star-White_Variant.usdz",
    "5cab2ea636da0bc0b06c8abf4809843498f7c1d680042b4f95e383b5c6a718b2",
);
const IPAD_PRO_13_M5: (&str, &str) = (
    "https://www.apple.com/105/media/us/ipad-pro/2025/adee90db-c01e-430d-b726-fe64c0063f08/ar/ipad-pro-space-black.usdz",
    "48098daf7c49117c81a4e4e1808899b93fad081faeba6c8b65124946b83b5ac6",
);

/// The file model `id` (t3code's ids) comes from, and its checksum.
fn source(id: &str) -> Option<(&'static str, &'static str)> {
    match id {
        // one file holds both iPhones
        "iphone-18-pro" | "iphone-18-pro-max" => Some(IPHONE_18_PRO),
        "ipad-pro-13-m5" => Some(IPAD_PRO_13_M5),
        "iphone-duo" => Some(IPHONE_DUO),
        _ => None,
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn cached(sha: &str) -> PathBuf {
    crate::settings::cache_dir().join("device-models").join(format!("{sha}.usdz"))
}

/// Model `id`'s USDZ: from the cache, else from apple.com. Blocks while it downloads (a few MB, once).
pub fn usdz(id: &str) -> anyhow::Result<Vec<u8>> {
    let (url, sha) = source(id).ok_or_else(|| anyhow::anyhow!("no model for {id}"))?;
    let path = cached(sha);
    if let Ok(bytes) = std::fs::read(&path)
        && sha256(&bytes) == sha
    {
        return Ok(bytes);
    }
    let mut response = ureq::get(url).call()?;
    let bytes = response.body_mut().with_config().limit(64 << 20).read_to_vec()?;
    anyhow::ensure!(sha256(&bytes) == sha, "apple.com served a different {id} model than expected");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("part");
    std::fs::write(&partial, &bytes)?;
    std::fs::rename(&partial, &path)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn known_models_only() {
        assert_eq!(super::source("iphone-18-pro"), super::source("iphone-18-pro-max"));
        assert!(super::source("ipad-pro-13-m5").is_some());
        assert!(super::source("iphone-duo").is_some());
        assert!(super::source("pixel-9").is_none());
        assert_eq!(super::sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}

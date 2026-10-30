pub mod budget;
pub mod calibration;
pub mod command;
pub mod export;
pub mod flow;
pub mod gpu;
pub mod input;
pub mod integrate;
pub mod lens;
pub mod output;
pub mod pipeline;
pub mod process;
pub mod progress;
pub mod register;
pub mod render;
pub mod review;
pub mod separate;
pub mod sequence;
pub mod sky;
pub mod stars;
pub mod store;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const DECODER_REVISION: &str = "dbe90dc1b5959708cc6424c7fcc18d55d52d2593";

pub fn error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::other(message.into()).into()
}

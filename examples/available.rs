use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use hwcodec::common::get_gpu_signature;

fn main() {
    let start = std::time::Instant::now();
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    #[cfg(windows)]
    vram();
    log::info!(
        "signature: {:?}, elapsed: {:?}",
        get_gpu_signature(),
        start.elapsed()
    );
}

#[cfg(windows)]
fn vram() {
    use hwcodec::common::MAX_GOP;
    use hwcodec::vram::{decode, encode, DynamicContext};
    println!("vram:");
    println!("encoders:");
    let encoders = encode::available(DynamicContext {
        width: 1920,
        height: 1080,
        kbitrate: 5000,
        framerate: 30,
        gop: MAX_GOP as _,
        device: None,
    });
    encoders.iter().map(|e| println!("{:?}", e)).count();
    println!("decoders:");
    let decoders = decode::available();
    decoders.iter().map(|e| println!("{:?}", e)).count();
}

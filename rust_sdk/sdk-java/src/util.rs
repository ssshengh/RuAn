use log::error;

pub fn exec_callback<F: FnOnce() -> anyhow::Result<()>>(func: F) {
    if let Err(e) = func() {
        error!("[jni] {:#?}", e);
    }
}
#[cfg(target_os = "macos")]
const QOS_CLASS_BACKGROUND: u32 = 0x09;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
}

#[cfg(target_os = "macos")]
pub fn lower_current_thread_priority() {
    // SAFETY: sets the quality of service of the calling thread only; no pointers are passed.
    unsafe { pthread_set_qos_class_self_np(QOS_CLASS_BACKGROUND, 0) };
}

#[cfg(windows)]
pub fn lower_current_thread_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    };
    // SAFETY: the pseudo handle of the calling thread is always valid; background mode lowers its CPU and disk priority.
    let result = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) };
    drop(result);
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn lower_current_thread_priority() {}

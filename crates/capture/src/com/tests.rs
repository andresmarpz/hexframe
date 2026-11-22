use windows::Win32::System::Com::{
    APTTYPE, APTTYPE_MAINSTA, APTTYPE_STA, APTTYPEQUALIFIER, COINIT_APARTMENTTHREADED,
    CoGetApartmentType, CoInitializeEx, CoUninitialize,
};

use super::ComGuard;

fn apartment() -> Option<APTTYPE> {
    let mut kind = APTTYPE::default();
    let mut qualifier = APTTYPEQUALIFIER::default();
    // SAFETY: both pointers refer to live locals.
    unsafe { CoGetApartmentType(&mut kind, &mut qualifier) }
        .ok()
        .map(|()| kind)
}

#[test]
fn a_thread_already_in_single_threaded_mode_gets_an_error_and_no_uninit() {
    std::thread::spawn(|| {
        // SAFETY: balanced by the CoUninitialize at the end of this thread.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .expect("sta init");
        assert!(ComGuard::mta().is_err());
        let still = apartment();
        assert!(
            matches!(still, Some(APTTYPE_STA | APTTYPE_MAINSTA)),
            "{still:?}"
        );
        // SAFETY: balances the CoInitializeEx above; a second uninit would be a bug in the guard.
        unsafe { CoUninitialize() };
    })
    .join()
    .expect("thread");
}

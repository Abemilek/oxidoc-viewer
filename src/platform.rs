//! Android platform helpers. JNI is used only to open the "All files access" settings screen
//! (MANAGE_EXTERNAL_STORAGE) and to query whether access was granted.
use std::path::PathBuf;

pub type AppHandle = android_activity::AndroidApp;

const PACKAGE: &str = "dev.visordocx.rs";

pub fn quick_dirs(h: &AppHandle) -> Vec<(String, PathBuf)> {
    let mut v = vec![
        ("Storage".to_string(), PathBuf::from("/storage/emulated/0")),
        (
            "Downloads".to_string(),
            PathBuf::from("/storage/emulated/0/Download"),
        ),
        (
            "Documents".to_string(),
            PathBuf::from("/storage/emulated/0/Documents"),
        ),
    ];
    if let Some(p) = h.external_data_path() {
        v.push(("App folder".to_string(), p));
    }
    v
}

pub fn has_access(h: &AppHandle) -> bool {
    use jni::JavaVM;
    let r = (|| -> Option<bool> {
        let vm = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        match env.call_static_method(
            "android/os/Environment",
            "isExternalStorageManager",
            "()Z",
            &[],
        ) {
            Ok(v) => v.z().ok(),
            Err(_) => {
                let _ = env.exception_clear();
                None
            }
        }
    })();
    // If the check failed (older Android), assume access is granted.
    r.unwrap_or(true)
}

pub fn request_access(h: &AppHandle) {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let _ = (|| -> Option<()> {
        let vm = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(h.activity_as_ptr() as jni::sys::jobject) };

        // 1) Try the app-specific settings screen.
        let specific = (|| -> Option<()> {
            let action: JObject = env
                .new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")
                .ok()?
                .into();
            let intent = env
                .new_object(
                    "android/content/Intent",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&action)],
                )
                .ok()?;
            let uri_str: JObject = env.new_string(format!("package:{PACKAGE}")).ok()?.into();
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&uri_str)],
                )
                .ok()?
                .l()
                .ok()?;
            env.call_method(
                &intent,
                "setData",
                "(Landroid/net/Uri;)Landroid/content/Intent;",
                &[JValue::Object(&uri)],
            )
            .ok()?;
            env.call_method(
                &activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )
            .ok()?;
            Some(())
        })();
        if specific.is_some() {
            return Some(());
        }
        let _ = env.exception_clear();

        // 2) Fallback: general list of apps with all-files access.
        let action: JObject = env
            .new_string("android.settings.MANAGE_ALL_FILES_ACCESS_PERMISSION")
            .ok()?
            .into();
        let intent = env
            .new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&action)],
            )
            .ok()?;
        if env
            .call_method(
                &activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )
            .is_err()
        {
            let _ = env.exception_clear();
        }
        Some(())
    })();
}

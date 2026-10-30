//! Android platform helpers. JNI is used only to open the "All files access" settings screen
//! (MANAGE_EXTERNAL_STORAGE) and to query whether access was granted.
use std::path::PathBuf;

pub type AppHandle = android_activity::AndroidApp;

const PACKAGE: &str = "dev.oxidoc.viewer";

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

/// Height of the system status bar in physical pixels (0.0 if it cannot be queried).
/// The window is drawn edge-to-edge, so the UI must leave this space free at the top.
pub fn status_bar_height_px(h: &AppHandle) -> f32 {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let r = (|| -> Option<f32> {
        let vm = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(h.activity_as_ptr() as jni::sys::jobject) };
        let res = env
            .call_method(
                &activity,
                "getResources",
                "()Landroid/content/res/Resources;",
                &[],
            )
            .ok()?
            .l()
            .ok()?;
        let name: JObject = env.new_string("status_bar_height").ok()?.into();
        let kind: JObject = env.new_string("dimen").ok()?.into();
        let pkg: JObject = env.new_string("android").ok()?.into();
        let id = env
            .call_method(
                &res,
                "getIdentifier",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                &[
                    JValue::Object(&name),
                    JValue::Object(&kind),
                    JValue::Object(&pkg),
                ],
            )
            .ok()?
            .i()
            .ok()?;
        if id == 0 {
            return None;
        }
        let px = env
            .call_method(&res, "getDimensionPixelSize", "(I)I", &[JValue::Int(id)])
            .ok()?
            .i()
            .ok()?;
        Some(px as f32)
    })();
    r.unwrap_or(0.0)
}

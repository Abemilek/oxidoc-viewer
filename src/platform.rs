//! Platform-specific code. On Android, JNI is used only to open the "All files access"
//! settings screen (MANAGE_EXTERNAL_STORAGE) and to query whether access was granted.
use std::path::PathBuf;

#[cfg(target_os = "android")]
pub type AppHandle = android_activity::AndroidApp;
#[cfg(not(target_os = "android"))]
pub type AppHandle = ();

#[cfg(target_os = "android")]
const PACKAGE: &str = "dev.visordocx.rs";

#[cfg(not(target_os = "android"))]
pub fn quick_dirs(_h: &AppHandle) -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        v.push(("Inicio".to_string(), home.clone()));
        for (label, d) in [
            ("Documentos", "Documentos"),
            ("Documents", "Documents"),
            ("Descargas", "Descargas"),
            ("Downloads", "Downloads"),
        ] {
            let p = home.join(d);
            if p.is_dir() {
                v.push((label.to_string(), p));
            }
        }
    }
    v
}

#[cfg(target_os = "android")]
pub fn quick_dirs(h: &AppHandle) -> Vec<(String, PathBuf)> {
    let mut v = vec![
        (
            "Almacenamiento".to_string(),
            PathBuf::from("/storage/emulated/0"),
        ),
        (
            "Descargas".to_string(),
            PathBuf::from("/storage/emulated/0/Download"),
        ),
        (
            "Documentos".to_string(),
            PathBuf::from("/storage/emulated/0/Documents"),
        ),
    ];
    if let Some(p) = h.external_data_path() {
        v.push(("Carpeta de la app".to_string(), p));
    }
    v
}

#[cfg(not(target_os = "android"))]
pub fn has_access(_h: &AppHandle) -> bool {
    true
}

#[cfg(target_os = "android")]
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

#[cfg(not(target_os = "android"))]
pub fn request_access(_h: &AppHandle) {}

#[cfg(target_os = "android")]
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

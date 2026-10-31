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

/// Closes the activity (used by the Back gesture on the first screen).
pub fn finish_activity(h: &AppHandle) {
    use jni::objects::JObject;
    use jni::JavaVM;

    let _ = (|| -> Option<()> {
        let vm = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(h.activity_as_ptr() as jni::sys::jobject) };
        if env.call_method(&activity, "finish", "()V", &[]).is_err() {
            let _ = env.exception_clear();
        }
        Some(())
    })();
}

const MAX_LAUNCH_DOC: usize = 150 * 1024 * 1024;

/// Reads the document that started the app through "Open with" (an `ACTION_VIEW` intent).
/// Returns the display name and the file contents. The data is read through the
/// `ContentResolver`, so no storage permission is needed for it.
pub fn launch_document(h: &AppHandle) -> Option<(String, Vec<u8>)> {
    use jni::objects::{JByteArray, JObject, JString, JValue};
    use jni::{JNIEnv, JavaVM};

    fn string_of(env: &mut JNIEnv, obj: JObject) -> Option<String> {
        if obj.is_null() {
            return None;
        }
        let js = JString::from(obj);
        let s: String = env.get_string(&js).ok()?.into();
        Some(s)
    }

    fn display_name(env: &mut JNIEnv, resolver: &JObject, uri: &JObject) -> Option<String> {
        let null = JObject::null();
        let cursor = env
            .call_method(
                resolver,
                "query",
                "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
                &[
                    JValue::Object(uri),
                    JValue::Object(&null),
                    JValue::Object(&null),
                    JValue::Object(&null),
                    JValue::Object(&null),
                ],
            )
            .ok()?
            .l()
            .ok()?;
        if cursor.is_null() {
            return None;
        }
        let mut result = None;
        let col: JObject = env.new_string("_display_name").ok()?.into();
        let idx = env
            .call_method(
                &cursor,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&col)],
            )
            .ok()?
            .i()
            .ok()?;
        if idx >= 0 {
            let moved = env
                .call_method(&cursor, "moveToFirst", "()Z", &[])
                .ok()?
                .z()
                .ok()?;
            if moved {
                let s = env
                    .call_method(
                        &cursor,
                        "getString",
                        "(I)Ljava/lang/String;",
                        &[JValue::Int(idx)],
                    )
                    .ok()?
                    .l()
                    .ok()?;
                result = string_of(env, s);
            }
        }
        let _ = env.call_method(&cursor, "close", "()V", &[]);
        result
    }

    let r = (|| -> Option<(String, Vec<u8>)> {
        let vm = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(h.activity_as_ptr() as jni::sys::jobject) };

        let intent = env
            .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])
            .ok()?
            .l()
            .ok()?;
        if intent.is_null() {
            return None;
        }
        let action = env
            .call_method(&intent, "getAction", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        if string_of(&mut env, action).as_deref() != Some("android.intent.action.VIEW") {
            return None;
        }
        let uri = env
            .call_method(&intent, "getData", "()Landroid/net/Uri;", &[])
            .ok()?
            .l()
            .ok()?;
        if uri.is_null() {
            return None;
        }
        let resolver = env
            .call_method(
                &activity,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )
            .ok()?
            .l()
            .ok()?;

        let name = display_name(&mut env, &resolver, &uri)
            .or_else(|| {
                let seg = env
                    .call_method(&uri, "getLastPathSegment", "()Ljava/lang/String;", &[])
                    .ok()?
                    .l()
                    .ok()?;
                string_of(&mut env, seg)
            })
            .unwrap_or_else(|| "document".to_string());

        let stream = env
            .call_method(
                &resolver,
                "openInputStream",
                "(Landroid/net/Uri;)Ljava/io/InputStream;",
                &[JValue::Object(&uri)],
            )
            .ok()?
            .l()
            .ok()?;
        if stream.is_null() {
            return None;
        }
        let chunk_obj: JObject = env.new_byte_array(64 * 1024).ok()?.into();
        // Typed, non-owning alias of the same array (needed to copy the bytes out).
        let chunk = JByteArray::from(unsafe { JObject::from_raw(chunk_obj.as_raw()) });
        let mut tmp = vec![0i8; 64 * 1024];
        let mut out: Vec<u8> = Vec::new();
        loop {
            let n = env
                .call_method(&stream, "read", "([B)I", &[JValue::Object(&chunk_obj)])
                .ok()?
                .i()
                .ok()?;
            if n < 0 {
                break;
            }
            let n = n as usize;
            if n == 0 {
                continue;
            }
            env.get_byte_array_region(&chunk, 0, &mut tmp[..n]).ok()?;
            out.extend(tmp[..n].iter().map(|b| *b as u8));
            if out.len() > MAX_LAUNCH_DOC {
                return None;
            }
        }
        let _ = env.call_method(&stream, "close", "()V", &[]);
        Some((name, out))
    })();

    // A failed call can leave a pending Java exception; clear it so later calls keep working.
    if r.is_none() {
        if let Ok(vm) = unsafe { JavaVM::from_raw(h.vm_as_ptr() as *mut jni::sys::JavaVM) } {
            if let Ok(mut env) = vm.attach_current_thread() {
                let _ = env.exception_clear();
            }
        }
    }
    r
}

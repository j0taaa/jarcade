use jarcade::feedback::Pulse;
use jarcade::settings::Settings;

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn jarcade_wake() {
    macroquad::miniquad::window::schedule_update();
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn jarcade_arm_timer(milliseconds: f64);
    fn jarcade_load(buffer: *mut u8, capacity: usize) -> usize;
    fn jarcade_save(buffer: *const u8, length: usize) -> i32;
    fn jarcade_fih_load(buffer: *mut u8, capacity: usize) -> usize;
    fn jarcade_fih_save(buffer: *const u8, length: usize) -> i32;
    fn jarcade_interrupted() -> i32;
    fn jarcade_haptics_supported() -> i32;
    fn jarcade_haptic(kind: i32);
    fn jarcade_appearance(saver: i32);
    fn jarcade_announce(buffer: *const u8, length: usize);
}

pub fn configure_display() {
    #[cfg(target_os = "ios")]
    ios_haptics::configure_display();
}

pub fn appearance(saver: bool) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: scalar boolean passed to our bundled web adapter.
    unsafe {
        jarcade_appearance(i32::from(saver));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = saver;
}

pub fn announce(text: &str) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the adapter reads the valid UTF-8 slice synchronously.
    unsafe {
        jarcade_announce(text.as_ptr(), text.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

pub fn haptics_supported() -> bool {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: scalar capability check in our adapter.
    unsafe {
        jarcade_haptics_supported() != 0
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        true
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    {
        false
    }
}

pub fn haptic(pulse: Pulse) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: small enum value accepted by the adapter, no pointers retained.
    unsafe {
        jarcade_haptic(pulse as i32);
    }
    #[cfg(target_os = "android")]
    android_haptic(pulse);
    #[cfg(target_os = "ios")]
    ios_haptics::play(pulse);
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    let _ = pulse;
}

/// One sleeping worker on native platforms, one cancellable timeout on web.
/// No polling and no active timer while a menu or paused game is visible.
pub struct WakeTimer {
    #[cfg(not(target_arch = "wasm32"))]
    sender: std::sync::mpsc::Sender<Option<std::time::Duration>>,
    #[cfg(not(target_arch = "wasm32"))]
    pending: std::cell::Cell<bool>,
}

impl WakeTimer {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc::{self, RecvTimeoutError};
            let (sender, receiver) = mpsc::channel::<Option<std::time::Duration>>();
            std::thread::spawn(move || {
                let mut delay = None;
                loop {
                    match delay {
                        None => match receiver.recv() {
                            Ok(next) => delay = next,
                            Err(_) => break,
                        },
                        Some(duration) => match receiver.recv_timeout(duration) {
                            Ok(next) => delay = next,
                            Err(RecvTimeoutError::Timeout) => {
                                macroquad::miniquad::window::schedule_update();
                                delay = None;
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                        },
                    }
                }
            });
            Self {
                sender,
                pending: std::cell::Cell::new(false),
            }
        }
        #[cfg(target_arch = "wasm32")]
        Self {}
    }

    pub fn arm(&self, seconds: Option<f64>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let immediate = seconds == Some(0.0);
            let delay = if immediate {
                None
            } else {
                seconds.map(std::time::Duration::from_secs_f64)
            };
            // Leave the worker asleep during display-paced rendering.
            let was_pending = self.pending.replace(delay.is_some());
            if delay.is_some() || was_pending {
                let _ = self.sender.send(delay);
            }
            if immediate {
                macroquad::miniquad::window::schedule_update();
            }
        }
        #[cfg(target_arch = "wasm32")]
        // SAFETY: the bundled web adapter provides this scalar-only import.
        unsafe {
            jarcade_arm_timer(seconds.map_or(-1.0, |s| s * 1000.0));
        }
    }
}

pub fn web_interrupted() -> bool {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the bundled adapter returns and clears a boolean interruption flag.
    unsafe {
        jarcade_interrupted() != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn load_settings() -> Settings {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buffer = [0u8; 128];
        // SAFETY: adapter writes at most capacity bytes into this live buffer.
        let length = unsafe { jarcade_load(buffer.as_mut_ptr(), buffer.len()) }.min(buffer.len());
        Settings::decode(std::str::from_utf8(&buffer[..length]).unwrap_or_default())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        settings_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|value| Settings::decode(&value))
            .unwrap_or_default()
    }
}

pub fn save_settings(settings: Settings) -> bool {
    let value = settings.encode();
    #[cfg(target_arch = "wasm32")]
    // SAFETY: adapter reads this valid buffer synchronously, retaining no pointer.
    unsafe {
        jarcade_save(value.as_ptr(), value.len()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(path) = settings_path() else {
            return false;
        };
        let Some(parent) = path.parent() else {
            return false;
        };
        // A failed/corrupt save must never prevent playing.
        std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::write(path, value))
            .is_ok()
    }
}

pub fn load_fih() -> jarcade::fih::Fih {
    let now = macroquad::miniquad::date::now();
    #[cfg(target_arch = "wasm32")]
    {
        let mut buffer = [0u8; 1024];
        // SAFETY: the adapter copies at most capacity bytes synchronously.
        let length =
            unsafe { jarcade_fih_load(buffer.as_mut_ptr(), buffer.len()) }.min(buffer.len());
        jarcade::fih::Fih::decode(
            std::str::from_utf8(&buffer[..length]).unwrap_or_default(),
            now,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let value = settings_path()
            .and_then(|p| std::fs::read_to_string(p.with_file_name("fih.txt")).ok())
            .unwrap_or_default();
        jarcade::fih::Fih::decode(&value, now)
    }
}

pub fn save_fih(pet: &jarcade::fih::Fih) -> bool {
    let value = pet.encode();
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the adapter reads this UTF-8 slice synchronously.
    unsafe {
        jarcade_fih_save(value.as_ptr(), value.len()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(path) = settings_path().map(|p| p.with_file_name("fih.txt")) else {
            return false;
        };
        let Some(parent) = path.parent() else {
            return false;
        };
        let temp = path.with_extension("tmp");
        std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::write(&temp, value))
            .and_then(|()| std::fs::rename(temp, path))
            .is_ok()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn settings_path() -> Option<std::path::PathBuf> {
    #[cfg(not(target_os = "android"))]
    use std::path::PathBuf;
    #[cfg(target_os = "android")]
    let base = android_files_dir()?;
    #[cfg(target_os = "ios")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Documents");
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows"
    )))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("jarcade/settings.txt"))
}

#[cfg(target_os = "android")]
fn android_files_dir() -> Option<std::path::PathBuf> {
    use macroquad::miniquad::native::android::{ACTIVITY, attach_jni_env};
    // SAFETY: called on miniquad's attached render thread, after Activity startup.
    // JNI references are checked and released before returning an owned path.
    unsafe {
        let env = attach_jni_env();
        let activity = ACTIVITY;
        let class = (**env).GetObjectClass?(env, activity);
        let method = (**env).GetMethodID?(
            env,
            class,
            c"getFilesDir".as_ptr(),
            c"()Ljava/io/File;".as_ptr(),
        );
        let file = (**env).CallObjectMethod?(env, activity, method);
        (**env).DeleteLocalRef?(env, class);
        if file.is_null() {
            return None;
        }
        let class = (**env).GetObjectClass?(env, file);
        let method = (**env).GetMethodID?(
            env,
            class,
            c"getAbsolutePath".as_ptr(),
            c"()Ljava/lang/String;".as_ptr(),
        );
        let path = (**env).CallObjectMethod?(env, file, method);
        (**env).DeleteLocalRef?(env, class);
        (**env).DeleteLocalRef?(env, file);
        if path.is_null() {
            return None;
        }
        let bytes = (**env).GetStringUTFChars?(env, path, std::ptr::null_mut());
        if bytes.is_null() {
            (**env).DeleteLocalRef?(env, path);
            return None;
        }
        let result = std::ffi::CStr::from_ptr(bytes)
            .to_str()
            .ok()
            .map(std::path::PathBuf::from);
        (**env).ReleaseStringUTFChars?(env, path, bytes);
        (**env).DeleteLocalRef?(env, path);
        result
    }
}

#[cfg(target_os = "android")]
fn android_haptic(pulse: Pulse) {
    use macroquad::miniquad::native::android::{ACTIVITY, attach_jni_env, ndk_sys::jvalue};
    // SAFETY: miniquad's render thread is attached to the VM. Vibrator is a
    // system service (no View mutation), so no UI-thread dispatch is required.
    // A local JNI frame owns every reference; errors are cleared before return.
    unsafe {
        let env = attach_jni_env();
        let jni = &**env;
        if jni.PushLocalFrame.unwrap()(env, 32) < 0 {
            return;
        }
        let run = || -> Option<()> {
            let activity = ACTIVITY;
            let class = jni.GetObjectClass?(env, activity);
            let method = jni.GetMethodID?(
                env,
                class,
                c"getSystemService".as_ptr(),
                c"(Ljava/lang/String;)Ljava/lang/Object;".as_ptr(),
            );
            if method.is_null() {
                return None;
            }
            let name = jni.NewStringUTF?(env, c"vibrator".as_ptr());
            let vibrator =
                jni.CallObjectMethodA?(env, activity, method, [jvalue { l: name }].as_ptr());
            if vibrator.is_null() {
                return None;
            }
            let class = jni.GetObjectClass?(env, vibrator);
            let has = jni.GetMethodID?(env, class, c"hasVibrator".as_ptr(), c"()Z".as_ptr());
            if has.is_null() || jni.CallBooleanMethodA?(env, vibrator, has, std::ptr::null()) == 0 {
                return None;
            }
            let duration = match pulse {
                Pulse::Tap => 8,
                Pulse::Eat => 18,
                Pulse::Lost => 35,
                Pulse::Won => 45,
            };
            let version = jni.FindClass?(env, c"android/os/Build$VERSION".as_ptr());
            let sdk_field = jni.GetStaticFieldID?(env, version, c"SDK_INT".as_ptr(), c"I".as_ptr());
            let sdk = jni.GetStaticIntField?(env, version, sdk_field);
            if sdk >= 26 {
                let effect_class = jni.FindClass?(env, c"android/os/VibrationEffect".as_ptr());
                let create = jni.GetStaticMethodID?(
                    env,
                    effect_class,
                    c"createOneShot".as_ptr(),
                    c"(JI)Landroid/os/VibrationEffect;".as_ptr(),
                );
                if create.is_null() {
                    return None;
                }
                let effect = jni.CallStaticObjectMethodA?(
                    env,
                    effect_class,
                    create,
                    [jvalue { j: duration }, jvalue { i: -1 }].as_ptr(),
                );
                if effect.is_null() {
                    return None;
                }
                let vibrate = jni.GetMethodID?(
                    env,
                    class,
                    c"vibrate".as_ptr(),
                    c"(Landroid/os/VibrationEffect;)V".as_ptr(),
                );
                if vibrate.is_null() {
                    return None;
                }
                jni.CallVoidMethodA?(env, vibrator, vibrate, [jvalue { l: effect }].as_ptr());
            } else {
                let vibrate = jni.GetMethodID?(env, class, c"vibrate".as_ptr(), c"(J)V".as_ptr());
                if vibrate.is_null() {
                    return None;
                }
                jni.CallVoidMethodA?(env, vibrator, vibrate, [jvalue { j: duration }].as_ptr());
            }
            Some(())
        };
        let _ = run();
        if jni.ExceptionCheck.unwrap()(env) != 0 {
            jni.ExceptionClear.unwrap()(env);
        }
        jni.PopLocalFrame.unwrap()(env, std::ptr::null_mut());
    }
}

#[cfg(target_os = "ios")]
#[allow(unexpected_cfgs)] // objc-rs macros use an optional internal cargo-clippy cfg.
mod ios_haptics {
    use super::Pulse;
    use objc_rs::{class, msg_send, runtime::Object, sel, sel_impl};
    use std::ffi::c_void;

    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *mut c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    pub fn configure_display() {
        // SAFETY: configure UIKit only on the main queue, after the view exists.
        unsafe {
            dispatch_async_f(
                std::ptr::addr_of!(_dispatch_main_q).cast_mut(),
                std::ptr::null_mut(),
                configure_view,
            );
        }
    }

    extern "C" fn configure_view(_: *mut c_void) {
        // SAFETY: the backend owns this live UIView; guard the Metal-only selector.
        unsafe {
            let view = macroquad::miniquad::window::apple_view().cast::<Object>();
            let supported: bool =
                msg_send![view, respondsToSelector: sel!(setPreferredFramesPerSecond:)];
            if supported {
                let window: *mut Object = msg_send![view, window];
                let screen: *mut Object = msg_send![window, screen];
                let maximum: isize = msg_send![screen, maximumFramesPerSecond];
                if maximum > 0 {
                    let _: () = msg_send![view, setPreferredFramesPerSecond: maximum];
                }
            }
        }
    }

    pub fn play(pulse: Pulse) {
        let context = Box::into_raw(Box::new(pulse)).cast();
        // SAFETY: libdispatch retains the context until the main-thread callback;
        // that callback takes ownership exactly once. All UIKit calls stay there.
        unsafe {
            dispatch_async_f(
                std::ptr::addr_of!(_dispatch_main_q).cast_mut(),
                context,
                perform,
            );
        }
    }

    extern "C" fn perform(context: *mut c_void) {
        // SAFETY: `play` supplies the owned Pulse, and libdispatch invokes us on
        // the main queue. Allocated feedback generators are released locally.
        unsafe {
            let pulse = *Box::from_raw(context.cast::<Pulse>());
            objc_rs::rc::autoreleasepool(|| {
                if matches!(pulse, Pulse::Lost | Pulse::Won) {
                    let generator: *mut Object =
                        msg_send![class!(UINotificationFeedbackGenerator), new];
                    let kind: isize = if pulse == Pulse::Won { 0 } else { 2 };
                    let _: () = msg_send![generator, notificationOccurred: kind];
                    let _: () = msg_send![generator, release];
                } else {
                    let allocated: *mut Object =
                        msg_send![class!(UIImpactFeedbackGenerator), alloc];
                    let style: isize = if pulse == Pulse::Tap { 0 } else { 1 };
                    let generator: *mut Object = msg_send![allocated, initWithStyle: style];
                    let _: () = msg_send![generator, impactOccurred];
                    let _: () = msg_send![generator, release];
                }
            });
        }
    }
}

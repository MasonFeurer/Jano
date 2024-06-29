#[cfg(feature = "egui_27")]
pub mod egui_app;
#[cfg(any(feature = "wgpu_19", feature = "wgpu_20"))]
pub mod graphics;
pub mod input;

#[cfg(feature = "egui_27")]
pub use egui_27 as egui;
#[cfg(feature = "egui-wgpu")]
pub use egui_wgpu;
#[cfg(feature = "wgpu_19")]
pub use wgpu_19 as wgpu;
#[cfg(feature = "wgpu_20")]
pub use wgpu_20 as wgpu;

#[cfg(feature = "serde")]
pub use serde;

pub use android_activity;
pub use glam;
pub use jni;
pub use log;
pub use ndk;
pub use ndk_sys;

pub use input::*;

use android_activity::{AndroidApp, MainEvent, PollEvent};

use glam::{uvec2, vec2, UVec2, Vec2};

use std::str::FromStr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub type Window = ndk::native_window::NativeWindow;

static mut ANDROID: Option<AndroidApp> = None;
pub fn android() -> &'static AndroidApp {
    // SAFETY: ANDROID is only ever mutated at the beginning of android_main, after that, it is perfectly safe to access ANDROID.
    let err = "ANDROID not initialized ; try caling jano::init_android() first";
    unsafe { ANDROID.as_ref().expect(err) }
}

/// A raw picture obtained from MainActivity.takePicture().
/// Always stored as ARGB, 1 byte per channel.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub data: Vec<u8>,
    pub size: UVec2,
}

/// The raw picture recieved from JVM when user submits photo for MainActivity.takePhoto().
static PICTURE_TAKEN: Mutex<Option<Picture>> = Mutex::new(None);

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // This code is only called by the Android JVM, so `buf` should be valid.
#[no_mangle]
/// Called by the JVM after the user submits a photo for MainActivity.takePicture().
/// The pixel buffer for the photos passed by the JVM will always be ARGB, 1 byte per channel.
/// The resulting photo will be stored in PICTURE_TAKEN.
pub extern "C" fn Java_nodomain_jano_MainActivity_onPictureTaken(
    env: jni::JNIEnv,
    _class: jni::objects::JObject,
    buf: jni::sys::jarray,
    w: jni::sys::jint,
    h: jni::sys::jint,
) {
    if buf as usize == 0 {
        log::warn!("Rust onPictureTaken function recieved null buf");
        return;
    }
    use jni::objects::{JObject, JPrimitiveArray};

    let j_obj = unsafe { JObject::from_raw(buf) };
    let j_arr = JPrimitiveArray::from(j_obj);

    let len = env.get_array_length(&j_arr).unwrap() as usize;
    let mut buf_vec = vec![0i8; len];
    env.get_byte_array_region(j_arr, 0, &mut buf_vec).unwrap();

    let buf_vec: Vec<u8> = unsafe { std::mem::transmute(buf_vec) };
    *PICTURE_TAKEN.lock().unwrap() = Some(Picture {
        data: buf_vec,
        size: uvec2(w as u32, h as u32),
    });

    log::info!("Rust onPictureTaken recieved {len} bytes");
}

static TOP_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static RIGHT_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static BOTTOM_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static LEFT_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // This code is only called by the Android JVM, so `cutouts` should be valid.
#[no_mangle]
/// Callback from Java code to update display insets (cutouts).
pub extern "C" fn Java_nodomain_jano_MainActivity_onDisplayInsets(
    env: jni::JNIEnv,
    _class: jni::objects::JObject,
    cutouts: jni::sys::jarray,
) {
    if cutouts as usize == 0 {
        log::warn!("Rust onDisplayInsets function recieved null cutouts");
        return;
    }
    use jni::objects::{JObject, JPrimitiveArray};

    let mut array: [i32; 4] = [0; 4];
    unsafe {
        let j_obj = JObject::from_raw(cutouts);
        let j_arr = JPrimitiveArray::from(j_obj);
        env.get_int_array_region(j_arr, 0, array.as_mut()).unwrap();
    }

    TOP_DISPLAY_INSET.store(array[0], Ordering::Relaxed);
    RIGHT_DISPLAY_INSET.store(array[1], Ordering::Relaxed);
    BOTTOM_DISPLAY_INSET.store(array[2], Ordering::Relaxed);
    LEFT_DISPLAY_INSET.store(array[3], Ordering::Relaxed);
    log::info!("Setting DISPLAY_INSETS to {array:?}");
}

pub fn display_cutout(size: Vec2) -> (Vec2, Vec2) /* (min, max) */ {
    (
        vec2(
            LEFT_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
            TOP_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
        ),
        vec2(
            size.x - RIGHT_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
            size.y - BOTTOM_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
        ),
    )
}

pub fn init_android(android: AndroidApp) {
    // Enforce that ANDROID is only mutated once.
    // This is because all accesses to ANDROID via crate::android()
    // will directly access the data without any thread locking.
    // This of course doesn't prevent ALL data races.
    // For example if the user of this crate spawns another thread where they call `android()`,
    // and at the same time in another thread call `init_android()`.
    // We are ignoring this possibility because:
    // 1. It would be very rare to actually pull off.
    // 2. The user of this library can reasonably be expected to call `init_android` before ever calling `android()` on any thread.
    if unsafe { ANDROID.is_some() } {
        log::warn!("Attempted to init static ANDROID a second time");
        return;
    }
    unsafe { ANDROID = Some(android) }
}

#[derive(Clone, Copy)]
pub struct FrameStats {
    pub fps: u32,
}

pub trait AppState {
    fn on_main_event(&mut self, event: MainEvent, draw_frames: &mut bool) -> bool;
    fn on_frame(&mut self, stats: FrameStats);
    fn on_picture_taken(&mut self, _pic: Picture) {}
}

pub fn android_main<A: AppState>(temp_android: AndroidApp, mut app: A) {
    android_logd_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("main", log::LevelFilter::Info)
        .init();
    init_android(temp_android);

    let timeout = Duration::from_millis(1000 / 60);
    let mut draw_frames = false;
    let mut frame_count = 0;
    let mut last_fps_update = SystemTime::now();
    let mut fps = 0;

    let mut quit = false;
    while !quit {
        android().poll_events(Some(timeout), |event| match event {
            PollEvent::Wake => {}
            PollEvent::Timeout => {}
            PollEvent::Main(event) => {
                if app.on_main_event(event, &mut draw_frames) {
                    quit = true;
                }
            }
            _ => {}
        });

        let mut picture = PICTURE_TAKEN.lock().unwrap();
        if picture.is_some() {
            let pic = picture.take().unwrap();
            app.on_picture_taken(pic);
        }

        if draw_frames {
            // Update FPS
            frame_count += 1;
            if SystemTime::now()
                .duration_since(last_fps_update)
                .unwrap()
                .as_secs()
                >= 1
            {
                last_fps_update = SystemTime::now();
                fps = frame_count;
                frame_count = 0;
            }
            app.on_frame(FrameStats { fps });
        }
    }
}

pub fn local_utc_offset() -> std::io::Result<i32> {
    use jni::objects::JObject;

    let activity = android().activity_as_ptr();
    let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
    let vm =
        unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }.unwrap();
    let mut env = vm.get_env().unwrap();

    match env.call_method(activity, "localUtcOffset", "()I", &[]) {
        Err(_err) => panic!("JNI call to SocketWrapper.localUtcOffset failed"),
        Ok(obj) => match obj {
            jni::objects::JValueGen::Int(v) => Ok(v),
            _ => {
                panic!("Java function SocketWrapper.localUtcOffset returned non-int value")
            }
        },
    }
}

pub fn get_java_io_err(env: &mut jni::JNIEnv) -> Option<std::io::Error> {
    let activity = android().activity_as_ptr();
    let activity = unsafe { jni::objects::JObject::from_raw(activity as jni::sys::jobject) };
    let activity_class = env.get_object_class(activity).unwrap();

    let msg =
        match env.call_static_method(&activity_class, "getLastErr", "()Ljava/lang/String;", &[]) {
            Err(err) => panic!("JNI call to SocketWrapper.connect() failed : {err}"),
            Ok(object) => {
                let jni::objects::JValueGen::Object(object) = object else {
                    panic!("Java function MainActivity.getLastErr() did not return Object")
                };
                if object.as_raw() as usize == 0 {
                    // Object returned is null
                    return None;
                }
                env.get_string(&jni::objects::JString::from(object))
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            }
        };
    let code = match env.call_static_method(
        activity_class,
        "getLastErrCode",
        "()Ljava/lang/String;",
        &[],
    ) {
        Err(err) => panic!("JNI call to SocketWrapper.connect() failed : {err}"),
        Ok(object) => {
            let jni::objects::JValueGen::Object(object) = object else {
                panic!("Java function MainActivity.getLastErrCode() did not return Object")
            };
            if object.as_raw() as usize == 0 {
                // Object returned is null
                return None;
            }
            env.get_string(&jni::objects::JString::from(object))
                .unwrap()
                .to_string_lossy()
                .to_string()
        }
    };
    let kind = match code.as_str() {
        "ECONNREFUSED" => std::io::ErrorKind::ConnectionRefused,
        "ECONNRESET" => std::io::ErrorKind::ConnectionReset,
        _ => std::io::ErrorKind::Other,
    };

    Some(std::io::Error::new(kind, msg))
}

pub fn take_picture() -> Result<(), String> {
    use jni::objects::JObject;

    let activity = android().activity_as_ptr();
    let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
    let vm =
        unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }.unwrap();
    let mut env = vm.get_env().unwrap();
    match env.call_method(activity, "takePicture", "()V", &[]) {
        Ok(_) => {}
        Err(err) => Err(format!(
            "JNI call to MainActivity.takePicture() failed : {err:?}"
        ))?,
    }
    Ok(())
}

pub fn set_keyboard_visibility(vis: bool) -> Result<(), String> {
    use jni::objects::JObject;

    let activity = android().activity_as_ptr();
    let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
    let vm =
        unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }.unwrap();
    let mut env = vm.get_env().unwrap();
    let method = match vis {
        true => "showSoftKeyboard",
        false => "hideSoftKeyboard",
    };
    if let Err(_err) = env.call_method(activity, method, "()V", &[]) {
        Err(format!("JNI call to MainActivity.{method}() failed"))?
    }
    Ok(())
}

pub fn get_clipboard_content() -> Result<String, String> {
    use jni::objects::{JObject, JString, JValueGen};

    let activity = android().activity_as_ptr();
    let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
    let vm =
        unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }.unwrap();
    let mut env = vm.get_env().unwrap();

    match env.call_method(activity, "getClipboardContent", "()Ljava/lang/String;", &[]) {
        Ok(s) => {
            let JValueGen::Object(object) = s else {
                Err(String::from(
                    "Java function MainActivity.getClipboardContent() returned non-Object value",
                ))?
            };
            // check if the object returned was null
            if object.as_raw() as usize == 0 {
                Err(String::from(
                    "Java function MainActivity.getClipboardContent() returned null",
                ))?
            }
            Ok(env
                .get_string(&JString::from(object))
                .unwrap()
                .to_string_lossy()
                .to_string())
        }
        Err(_) => Err("JNI call to MainActivity.getClipboardContent() failed".into()),
    }
}

pub fn set_clipboard_content(value: &str) -> Result<(), String> {
    use jni::objects::{JObject, JString};

    let activity = android().activity_as_ptr();
    let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
    let vm =
        unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }.unwrap();
    let mut env = vm.get_env().unwrap();

    let java_value: JString = match env.new_string(value) {
        Ok(v) => v,
        Err(_) => Err(String::from("JNI jstring creation failed"))?,
    };

    match env.call_method(
        activity,
        "setClipboardContent",
        "(Ljava/lang/String;)V",
        &[(&java_value).into()],
    ) {
        Err(_) => Err("JNI call to MainActivity.setClipboardContent() failed".into()),
        Ok(_) => Ok(()),
    }
}

/// A good-enough drop-in-replacement of std::net::TcpStream
///
/// Implemented functions:
/// - connect
/// - connect_timeout
/// - set_read_timeout
/// - read_timeout
/// - set_nodelay
/// - no_delay
/// - local_address
///
/// Missing functions:
/// - peek
/// - peer_address
/// - set_write_timeout
/// - write_timeout
/// - set_ttl
/// - ttl
/// - take_error
/// - set_nonblocking
///
/// Implemented traits:
/// - std::io::Write
/// - std::io::Read
///
/// Missing traits:
/// - AsFd
/// - AsRawFd
/// - Into<OwndedFd>
/// - FromRawFd
/// - IntoRawFd
///
#[derive(Debug)]
pub struct TcpStream(jni::objects::GlobalRef);
impl TcpStream {
    pub fn as_raw(&self) -> &jni::objects::GlobalRef {
        &self.0
    }

    pub fn connect<A: std::net::ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let mut err = None;
        for addr in addr.to_socket_addrs()? {
            let port = addr.port();
            let addr_str = match addr {
                std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
                std::net::SocketAddr::V6(v6) => v6.ip().to_string(),
            };
            match Self::connect_single(&addr_str, port) {
                Ok(v) => return Ok(v),
                Err(cerr) => err = Some(cerr),
            }
        }
        Err(err.unwrap())
    }

    pub fn connect_timeout(
        addr: &std::net::SocketAddr,
        timeout: Duration,
    ) -> std::io::Result<Self> {
        use jni::objects::{JObject, JString, JValueGen};
        let (addr, port) = {
            let addr_str = match addr {
                std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
                std::net::SocketAddr::V6(v6) => v6.ip().to_string(),
            };
            (addr_str, addr.port())
        };

        let activity = android().activity_as_ptr();
        let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        let address_java_str: JString = match env.new_string(addr) {
            Ok(v) => v,
            Err(_) => panic!("JNI jstring construction failed"),
        };

        let activity_class = env.get_object_class(activity).unwrap();

        match env.call_static_method(
            activity_class,
            "connectNewSocket",
            "(Ljava/lang/String;II)Lnodomain/jano/SocketWrapper;",
            &[
                (&address_java_str).into(),
                JValueGen::Int(port as i32),
                JValueGen::Int(timeout.as_millis() as i32),
            ],
        ) {
            Err(_) => panic!("JNI call to SocketWrapper.connect() failed"),
            Ok(object) => {
                // static method 'connect' should return a SocketWrapper or null
                let JValueGen::Object(object) = object else {
                    panic!("Java function MainActivity.connectNewSocketTimeout() did not return Object")
                };
                if object.as_raw() as usize == 0 {
                    // Object returned was null
                    Err(get_java_io_err(&mut env).unwrap())?
                }
                Ok(Self(env.new_global_ref(object).unwrap()))
            }
        }
    }

    pub fn connect_single(address: &str, port: u16) -> std::io::Result<Self> {
        use jni::objects::{JObject, JString, JValueGen};

        let activity = android().activity_as_ptr();
        let activity = unsafe { JObject::from_raw(activity as jni::sys::jobject) };
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        let address_java_str: JString = match env.new_string(address) {
            Ok(v) => v,
            Err(_) => panic!("JNI jstring construction failed"),
        };

        let activity_class = env.get_object_class(activity).unwrap();

        match env.call_static_method(
            activity_class,
            "connectNewSocket",
            "(Ljava/lang/String;I)Lnodomain/jano/SocketWrapper;",
            &[(&address_java_str).into(), JValueGen::Int(port as i32)],
        ) {
            Err(_) => panic!("JNI call to SocketWrapper.connect() failed"),
            Ok(object) => {
                // static method 'connect' should return a SocketWrapper or null
                let JValueGen::Object(object) = object else {
                    panic!("Java function MainActivity.connectNewSocket() did not return Object")
                };
                if object.as_raw() as usize == 0 {
                    // Object returned was null
                    Err(get_java_io_err(&mut env).unwrap())?
                }
                Ok(Self(env.new_global_ref(object).unwrap()))
            }
        }
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        use jni::objects::JValueGen;

        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        let millis = dur.map(|dur| dur.as_millis() as i32).unwrap_or(0);

        match env.call_method(&self.0, "setReadTimeout", "(I)I", &[JValueGen::Int(millis)]) {
            Err(_err) => panic!("JNI call to SocketWrapper.setReadTimeout failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(_) => Ok(()),
                _ => panic!("Java function SocketWrapper.setReadTimeout returned non-int value"),
            },
        }
    }
    pub fn read_timeout(&self) -> std::io::Result<Option<Duration>> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        let millis = match env.call_method(&self.0, "readTimeout", "()I", &[]) {
            Err(_err) => panic!("JNI call to SocketWrapper.readTimeout failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(v) => Ok(v),
                _ => {
                    panic!("Java function SocketWrapper.readTimeout returned non-int value")
                }
            },
        }?;
        if millis == 0 {
            Ok(None)
        } else {
            Ok(Some(Duration::from_millis(millis as u64)))
        }
    }

    pub fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        match env.call_method(&self.0, "setNodelay", "(I)I", &[nodelay.into()]) {
            Err(_err) => panic!("JNI call to SocketWrapper.setNodelay failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(_) => Ok(()),
                _ => panic!("Java function SocketWrapper.setNodelay returned non-int value"),
            },
        }
    }
    pub fn nodelay(&self) -> std::io::Result<bool> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        match env.call_method(&self.0, "getNodelay", "()I", &[]) {
            Err(_err) => panic!("JNI call to SocketWrapper.getNodelay failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(0) => Ok(false),
                jni::objects::JValueGen::Int(1) => Ok(true),
                jni::objects::JValueGen::Int(_) => unreachable!(),
                _ => {
                    panic!("Java function SocketWrapper.getNodelay returned non-int value")
                }
            },
        }
    }

    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        let port: i32 = match env.call_method(&self.0, "getPort", "()I", &[]) {
            Err(_err) => panic!("JNI call to SocketWrapper.getPort failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(v) => v,
                v => panic!("Called SocketWrapper.getPort : expected int, got {v:?}"),
            },
        };
        let addr: String = match env.call_method(&self.0, "getAddress", "()Ljava/lang/String;", &[])
        {
            Err(_err) => panic!("JNI call to SocketWrapper.getAddress failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Object(object) => {
                    // check if the object returned was null
                    if object.as_raw() as usize == 0 {
                        panic!("Java function SocketWrapper.getAddress returned null")
                    }
                    env.get_string(&jni::objects::JString::from(object))
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                }
                _ => {
                    panic!("Java function SocketWrapper.getAddress returned non-Object value")
                }
            },
        };
        let addr = std::net::IpAddr::from_str(&addr).unwrap();
        Ok(std::net::SocketAddr::new(addr, port as u16))
    }
}
impl std::io::Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();
        let java_arr = env.byte_array_from_slice(buf).unwrap();

        match env.call_method(&self.0, "write", "([B)I", &[(&java_arr).into()]) {
            Err(_err) => panic!("JNI call to SocketWrapper.write failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(v) => Ok(v as usize),
                _ => panic!("Java function SocketWrapper.write returned non-int value"),
            },
        }
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();
        let java_arr = env.byte_array_from_slice(buf).unwrap();

        match env.call_method(&self.0, "writeAll", "([B)I", &[(&java_arr).into()]) {
            Err(_err) => panic!("JNI call to SocketWrapper.writeAll failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(_) => Ok(()),
                _ => panic!("Java function SocketWrapper.writeAll returned non-int value"),
            },
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        match env.call_method(&self.0, "flush", "()I", &[]) {
            Err(_err) => panic!("JNI call to SocketWrapper.flush failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap())?,
                jni::objects::JValueGen::Int(_) => Ok(()),
                _ => panic!("Java function SocketWrapper.flush returned non-int value"),
            },
        }
    }
}
impl std::io::Read for TcpStream {
    // FIXME: every value in `buf` gets overridden even if the Socket doen't read buf.len() bytes.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();
        let java_arr = env.new_byte_array(buf.len() as i32).unwrap();

        let count: i32 = match env.call_method(&self.0, "read", "([B)I", &[(&java_arr).into()]) {
            Err(_err) => panic!("JNI call to SocketWrapper.read failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap()),
                jni::objects::JValueGen::Int(v) => Ok(v),
                _ => panic!("Java function SocketWrapper.read returned non-int value"),
            },
        }?;
        let java_arr = unsafe {
            env.get_array_elements(&java_arr, jni::objects::ReleaseMode::NoCopyBack)
                .unwrap()
        };
        assert!(java_arr.len() == buf.len());
        let slice_i8: &[i8] = &java_arr;
        let slice_u8: &[u8] = unsafe { std::mem::transmute(slice_i8) };
        buf.clone_from_slice(slice_u8);
        Ok(count as usize)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();
        let java_arr = env.new_byte_array(buf.len() as i32).unwrap();

        match env.call_method(&self.0, "readExact", "([B)I", &[(&java_arr).into()]) {
            Err(_err) => panic!("JNI call to SocketWrapper.readExact failed"),
            Ok(obj) => match obj {
                jni::objects::JValueGen::Int(-1i32) => Err(get_java_io_err(&mut env).unwrap())?,
                jni::objects::JValueGen::Int(1i32) => {
                    // SocketWrapper.readExact() will return 1 when it reaches the end-of-stream.
                    Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, ""))?
                }
                jni::objects::JValueGen::Int(_) => {}
                _ => panic!("Java function SocketWrapper.flush returned non-int value"),
            },
        };
        let java_arr = unsafe {
            env.get_array_elements(&java_arr, jni::objects::ReleaseMode::NoCopyBack)
                .unwrap()
        };
        assert!(java_arr.len() == buf.len());
        let slice_i8: &[i8] = &java_arr;
        let slice_u8: &[u8] = unsafe { std::mem::transmute(slice_i8) };
        buf.clone_from_slice(slice_u8);
        Ok(())
    }
}
impl std::ops::Drop for TcpStream {
    fn drop(&mut self) {
        let vm = unsafe { jni::JavaVM::from_raw(android().vm_as_ptr() as *mut jni::sys::JavaVM) }
            .unwrap();
        let mut env = vm.get_env().unwrap();

        if let Err(err) = env.call_method(&self.0, "destroy", "()V", &[]) {
            log::error!("JNI call to SocketWrapper.destroy() failed : {err}");
        }
    }
}

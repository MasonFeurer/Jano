# Jano
Jano is a rust library that provides useful integration tools for android.
A list of android integrations provided by Jano:
- TcpStream connecting/reading/writing via java.net.Socket
- opening/closing keyboard overlay
- opening camera for a picture
- getting/setting clipboard content (text only)
- creating wgpu Surface (with `wgpu` feature flag enabled)
- touch to mouse event translations
- getting display insets (eg: the space the camera notch/island occupies)

## Example
The most basic android app can be created with the following.

```rust
use jano::{FrameStats};
use jano::android_activity::{AndroidApp, MainEvent};

#[no_mangle]
fn android_main(android: AndroidApp) {
    jano::android_main(android, App::default());
}

#[derive(Default)]
struct App;
impl jano::AppState for App {
    fn on_main_event(&mut self, event: MainEvent, _draw_frames: &mut bool) -> bool {
    	match event {
            MainEvent::Destroy => return true,
            _ => {}
        }
        false
    }
    fn on_frame(&mut self, _stats: FrameStats) {}
}
```

And in the `Cargo.toml` file:
```toml
[package]
name = "simple-example"
version = "0.1.0"
edition = "2021"

[lib]
name = "main"
crate-type = ["cdylib"]

[dependencies]
jano = { git = "https://github.com/MasonFeurer/Jano.git" }
```

## Running
To run a rust project that uses `jano`, you will need to use `jano-cli` (in this repository).
If you have not already, you will have to build `jano-cli` from source.
In the future, it will be available on https://crates.io, but that is not today.

# Jano CLI
## Building Jano CLI
- Download this repository:
- Go to `Jano/jano-cli`
- Use `cargo build`

```sh
git clone "https://github.com/MasonFeurer/Jano.git"
cd Jano/jano-cli
cargo b -r
```
Then, in order for the built `jano-cli` to be usable, you will have to add it to your PATH.
On Linux Debian, this can be done by putting this line into your `~/.bashrc` file:
```sh
export PATH="path_to_the_jano_binary:$PATH"
```

## Examples
To build your project: 
```sh
jano-cli b
```

To build, then run your app on a connected android device:
```sh
jano-cli r
```

To skip building, and run your app on a connected android device:
```sh
jano-cli r --no-build
```

## App attributes
Jano-cli will read your projects `Cargo.toml` for some info before building your app.
It will look at all members of the `[jano]` header in your TOML.
Here are the recognized options and their meaning:

- icon : (string)

  A path to an image file to be used for the android application's icon image.

- name : (string)

  The android application name. This is what android will show as the name of the app on the home screen.
  
- app_id : (string)

  The android application ID. Should look like a namespace.
  This uniquely identifies the app on the android device.
  <p>Example: "com.example.namespace"

- version : (string)

  The "version" value for the android APK. Used for versioning the android app on the android device.
  <p>Example: "0.1.0"
  
- build_targets : [(string), ...]

  The target names to build native libraries for. cargo-ndk is executed for every target.

- no_strip : true|false,

  Wether to strip debug symbols from the generated native library. Passed to cargo-ndk.

- java\_src\_files : [(string), ...]

  Paths to Java files to be copied into the java source directory of the created Java Application.
  Can be used to override MainActivity.java.


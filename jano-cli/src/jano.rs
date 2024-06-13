use crate::create_android_project::create_android_project;
use cargo_subcommand::Subcommand;
use std::path::Path;

use serde::Deserialize;

/// Only the fields from the [jano] namespace in the Cargo.toml manifest.
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct Manifest {
    /// A path to an image file to be used for the android application's icon image.
    icon: Option<String>,
    /// The android application name. This is what android will show as the name of the app on the home screen.
    name: String,
    /// The android application ID. Should look like a namespace (com.example.namespace).
    /// This is uniquely identifies the app on the android device.
    /// Id this value changes, Android will treat it as a completely seperate app.
    app_id: String,
    /// The "version" value for the android APK. Used for versioning the android app on the android device.
    version: String,
    /// The target names to build native libraries for. cargo-ndk is executed for every target.
    build_targets: Vec<String>,
    /// Weather to strip debug symbols from generated native library. Passed to cargo-ndk.
    no_strip: bool,
    /// Paths to Java files to be copied into the java source directory of the created Java Application.
    /// Can be used to override MainActivity.java.
    java_src_files: Vec<String>,
}
impl Manifest {
    pub fn set_defaults(&mut self) {
        if self.name.is_empty() {
            self.name = "Unnamed Jano App".into();
        }
        if self.app_id.is_empty() {
            self.app_id = "unnamed.java.app".into();
        }
        if self.version.is_empty() {
            self.version = "0.1.0".into();
        }
    }
}

impl Manifest {
    pub fn parse_from_toml(path: &Path) -> Result<Self, String> {
        #[derive(serde::Deserialize, Debug)]
        struct Root {
            #[serde(default)]
            jano: Manifest,
        }

        let contents = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
        let mut root: Root = toml::from_str(&contents).map_err(|e| e.to_string())?;
        root.jano.set_defaults();
        Ok(root.jano)
    }
}

fn cmd(dir: Option<&str>, cmd: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = std::process::Command::new(cmd);
    let cmd = if let Some(dir) = dir {
        cmd.current_dir(dir)
    } else {
        &mut cmd
    };
    cmd.args(args)
        .spawn()
        .map_err(|err| err.to_string())?
        .wait()
        .map_err(|err| err.to_string())?
        .success()
        .then_some(())
        .ok_or(format!("Proccess {cmd:?} failed"))
}

pub struct Jano<'a> {
    pub root: String,
    pub cmd: &'a Subcommand,
    pub manifest: Manifest,
    pub build_targets: Vec<String>,
    pub device_serial: Option<String>,
}
impl<'a> Jano<'a> {
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn from_subcommand(cmd: &'a Subcommand) -> Result<Self, String> {
        println!(
            "Using package `{}` in `{}`",
            cmd.package(),
            cmd.manifest().display()
        );
        let manifest = Manifest::parse_from_toml(cmd.manifest())?;

        let build_targets = if let Some(target) = cmd.target() {
            vec![target.to_string()]
        } else if !manifest.build_targets.is_empty() {
            manifest.build_targets.clone()
        } else {
            vec![String::from("armv7-linux-androideabi")]
        };

        let mut root = cmd.manifest().to_path_buf();
        root.pop();
        let root = root.display().to_string();

        Ok(Self {
            root,
            cmd,
            manifest,
            build_targets,
            device_serial: None,
        })
    }

    pub fn check(&self) -> Result<(), String> {
        todo!()
    }
    pub fn build(&self) -> Result<(), String> {
        println!("Building...");

        println!("Creating Android Java Application in \"./android\"...");
        create_android_project(
            &format!("{}/android", self.root()),
            &self.manifest.name,
            &self.manifest.app_id,
        )
        .map_err(|err| err.to_string())?;

        for path in &self.manifest.java_src_files {
            std::fs::copy(
                path,
                &format!("{}/android/app/src/main/java/nodomain/jano", self.root()),
            )
            .map_err(|err| err.to_string())?;
        }
        if let Some(path) = &self.manifest.icon {
            std::fs::copy(
                path,
                format!(
                    "{}/android/app/src/main/res/mipmap-mdpi/ic_launcher.png",
                    self.root()
                ),
            )
            .map_err(|err| err.to_string())?;
        }

        println!("Compiling rust code...");
        for target in &self.build_targets {
            let mut args = vec!["ndk", "-t", target, "-o", "./android/app/src/main/jniLibs"];
            if self.manifest.no_strip {
                args.push("--no-strip");
            }
            args.push("build");
            cmd(Some(self.root()), "cargo", &args)?;
        }

        println!("Building Java application with gradle...");
        cmd(
            Some(&format!("{}/android", self.root())),
            "gradle",
            &["build"],
        )?;
        println!("Jano finished building");
        Ok(())
    }
    pub fn install(&self, _device: Option<&str>) -> Result<(), String> {
        println!("Installing Android app to connected device with `gradle`...");
        cmd(
            Some(&format!("{}/android", self.root())),
            "gradle",
            &["installDebug"],
        )
    }
    pub fn run(&self, no_logcat: bool, _device: Option<&str>) -> Result<(), String> {
        let args = [
            "shell",
            "am",
            "start",
            "-n",
            &format!("{}/nodomain.jano.MainActivity", self.manifest.app_id),
        ];
        cmd(None, "adb", &args)?;

        if no_logcat {
            return Ok(());
        }

        let output = std::process::Command::new("adb")
            .args([
                "shell",
                "pm",
                "list",
                "package",
                "-U",
                &self.manifest.app_id,
            ])
            .output()
            .map_err(|err| err.to_string())?;
        let mut apk_uid = String::from_utf8_lossy(&output.stdout).to_string();
        _ = apk_uid.pop();
        let apk_uid = {
            let chars: Vec<_> = apk_uid.chars().enumerate().collect();
            let colon_idx = chars.into_iter().rev().find(|(_, c)| *c == ':').unwrap().0;
            &apk_uid[(colon_idx + 1)..]
        };

        cmd(None, "adb", &["logcat", "-c"])?;
        cmd(None, "adb", &["logcat", "-v", "color", "--uid", apk_uid])
    }
}

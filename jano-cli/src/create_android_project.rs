use std::{fs, io};

macro_rules! mv {
    ($name:literal,$dst:expr, $($q:literal,$r:expr)*) => {{
        println!("Writing file {:?} to {:?}", $name, $dst);
        fs::write(
            format!("{}/{}", $dst, $name),
            include_str!(concat!("project_files/", $name))$(.replace($q, $r))*,
        )
    }};
    ($name:literal,$dst:expr) => {{
        println!("Writing file {:?} to {:?}", $name, $dst);
        fs::write(
            format!("{}/{}", $dst, $name),
            include_bytes!(concat!("project_files/", $name)),
        )
    }};
    ($dir:literal,$name:literal,$dst:expr, $($q:literal,$r:expr)*) => {{
        println!("Writing file {:?} to {:?}", $name, $dst);
        fs::write(
            format!("{}/{}", $dst, $name),
            include_str!(concat!("project_files/", $dir, "/", $name))$(.replace($q, $r))*,
        )
    }};
    ($dir:literal,$name:literal,$dst:expr) => {{
        println!("Writing file {:?} to {:?}", $name, $dst);
        fs::write(
            format!("{}/{}", $dst, $name),
            include_bytes!(concat!("project_files/", $dir, "/", $name)),
        )
    }};
}

pub fn create_android_project(
    path: &str,
    name: &str,
    app_id: &str,
    orientation: &str,
) -> io::Result<()> {
    let app = format!("{path}/app");
    let main = format!("{app}/src/main");
    let java_src = format!("{main}/java/nodomain/jano");

    _ = fs::create_dir_all(&java_src);
    _ = fs::create_dir_all(format!("{main}/res/mipmap-mdpi"));
    _ = fs::create_dir_all(format!("{main}/res/values"));

    mv!("ic_launcher.png", &format!("{main}/res/mipmap-mdpi"))?;
    mv!("themes.xml", &format!("{main}/res/values"))?;
    mv!("build.gradle", &path)?;
    mv!("settings.gradle", &path)?;
    mv!("gradle.properties", &path)?;
    mv!("app", "build.gradle", &app, "{app_id}", app_id)?;
    mv!("proguard-rules.pro", &app)?;
    mv!(
        "AndroidManifest.xml",
        &main,
        "{app_name}",
        name
        "{orientation}",
        orientation
    )?;
    mv!("java", "MainActivity.java", &java_src)?;
    mv!("java", "SocketWrapper.java", &java_src)?;
    Ok(())
}

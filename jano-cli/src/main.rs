mod create_android_project;
mod jano;

use cargo_subcommand::Subcommand;
use clap::Parser;
use jano::Jano;

#[derive(Parser)]
struct Cmd {
    #[clap(subcommand)]
    sub_cmd: SubCmd,
}

type Args = cargo_subcommand::Args;

#[derive(clap::Subcommand)]
enum SubCmd {
    /// Analyze the current package and report errors, but don't build object files nor an apk.
    #[clap(visible_alias = "c")]
    Check {
        #[clap(flatten)]
        args: Args,
    },
    /// Build the current package and create an apk.
    #[clap(visible_alias = "b")]
    Build {
        #[clap(flatten)]
        args: Args,
    },
    /// Install the app onto a connected USB android device.
    #[clap(visible_alias = "i")]
    Install {
        #[clap(flatten)]
        args: Args,
        /// The serial of the device to install to.
        /// Can be none if you only have 1 device connected.
        device: Option<String>,
        /// Do not build project before installing.
        #[clap(short = 'b', long)]
        no_build: bool,
    },
    /// Run the APK on a connected device.
    #[clap(visible_alias = "r")]
    Run {
        #[clap(flatten)]
        args: Args,
        /// The serial of the device to run on.
        /// Can be none if you only have 1 device connected.
        device: Option<String>,
        /// Do not print or follow `logcat` after running the app
        #[clap(short = 'l', long)]
        no_logcat: bool,
        /// Do not build project before running.
        #[clap(short = 'b', long)]
        no_build: bool,
    },
    /// Print the version of jano-cli
    Version,
    /// Install any missing dependencies for jano-cli.
    Doctor,
}

fn main() -> Result<(), String> {
    let Cmd { sub_cmd } = Cmd::parse();

    match sub_cmd {
        SubCmd::Check { args } => {
            let cmd = Subcommand::new(args).map_err(|err| err.to_string())?;
            let jano = Jano::from_subcommand(&cmd)?;
            jano.check()
        }
        SubCmd::Build { args } => {
            let cmd = Subcommand::new(args).map_err(|err| err.to_string())?;
            let jano = Jano::from_subcommand(&cmd)?;
            jano.build()
        }
        SubCmd::Install {
            args,
            device,
            no_build,
        } => {
            let cmd = Subcommand::new(args).map_err(|err| err.to_string())?;
            let jano = Jano::from_subcommand(&cmd)?;
            if !no_build {
                jano.build()?;
            }
            jano.install(device.as_deref())
        }
        SubCmd::Run {
            args,
            device,
            no_logcat,
            no_build,
        } => {
            let cmd = Subcommand::new(args).map_err(|err| err.to_string())?;
            let jano = Jano::from_subcommand(&cmd)?;
            if !no_build {
                jano.build()?;
            }
            jano.install(device.as_deref())?;
            jano.run(no_logcat, device.as_deref())
        }
        SubCmd::Version => {
            println!("jano - 0.0.1");
            Ok(())
        }
        SubCmd::Doctor => {
            println!("Currently not implemented");
            Ok(())
        }
    }
}
